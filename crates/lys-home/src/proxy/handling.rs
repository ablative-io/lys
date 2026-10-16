use super::{
    Base, BodyExt, CHATGPT_ACCOUNT, CODEX_SESSION, Call, End, HeaderValue, Incoming, Method,
    OpenCall, PathAndQuery, Proxy, ProxyBody, ProxyError, Request, Response, StatusCode, Tee,
    api_of, fresh_id, is_event_stream, now, refusal, run_key,
};

impl Proxy {
    fn route(&self, path_and_query: &str) -> Option<(&'static str, &Base, String)> {
        let providers = [("anthropic", &self.anthropic), ("openai", &self.openai)];
        providers.into_iter().find_map(|(provider, base)| {
            let rest = path_and_query.strip_prefix('/')?.strip_prefix(provider)?;
            match rest.chars().next() {
                None => Some((provider, base, String::from("/"))),
                Some('/') => Some((provider, base, rest.to_owned())),
                Some('?') => Some((provider, base, format!("/{rest}"))),
                Some(_) => None,
            }
        })
    }

    /// Answer one request: forward it, and for a model call journal it
    /// first, capture it as it passes and hand it to the sink at its end.
    pub async fn handle(&self, mut request: Request<Incoming>) -> Response<ProxyBody> {
        if request.uri().path() == "/_lys/account/key" && request.method() == Method::GET {
            return self.accounts.public_key();
        }
        if request.uri().path() == "/_lys/account/bind" && request.method() == Method::POST {
            return match self.accounts.bind(&self.upstream, request).await {
                Ok(answer) => answer,
                Err(error) => refusal(StatusCode::FORBIDDEN, &error.to_string()),
            };
        }
        let path_and_query = request
            .uri()
            .path_and_query()
            .map_or("/", PathAndQuery::as_str)
            .to_owned();
        let (run, unkeyed) = run_key(&path_and_query);
        let Some((provider, base, rest)) = self.route(unkeyed) else {
            return refusal(
                StatusCode::NOT_FOUND,
                "lys-proxy refused the call: its path names no provider. A path begins \
                 /anthropic or /openai, after its run key when it carries one, and a run key \
                 is the 32 lowercase hexadecimal digits a Lys launch gave the run; any other \
                 first part is not a key",
            );
        };
        let api = api_of(request.method(), &rest);
        let binding = match self.accounts.binding(run) {
            Ok(binding) => binding,
            Err(error) => return refusal(StatusCode::FORBIDDEN, &error.to_string()),
        };
        let drawn = match self.select_account(binding.as_ref(), provider, base, &mut request) {
            Ok(drawn) => drawn,
            Err(error) => return refusal(StatusCode::FORBIDDEN, &error.to_string()),
        };
        // A Codex signed in with ChatGPT sends its account's id, and is
        // answered by ChatGPT's Codex backend, whose paths carry no `/v1`.
        // One header lookup; the machine's own Codex sign-in decides.
        let chatgpt = provider == "openai"
            && (request.headers().contains_key(CHATGPT_ACCOUNT)
                || binding
                    .as_ref()
                    .is_some_and(|b| b.provider == self.chatgpt.as_str()));
        let (base, rest) = if chatgpt {
            let rest = rest.strip_prefix("/v1").unwrap_or(&rest).to_owned();
            (&self.chatgpt, rest)
        } else {
            (base, rest)
        };
        let Some(api) = api else {
            // Not a model call: forwarded as it came, and not recorded.
            let request = request.map(BodyExt::boxed_unsync);
            return match self
                .account_send(base, &rest, request, drawn.as_ref())
                .await
            {
                Ok(response) => response.map(BodyExt::boxed_unsync),
                Err(error) => refusal(
                    StatusCode::SERVICE_UNAVAILABLE,
                    &format!("lys-proxy: {error}"),
                ),
            };
        };
        let mut open = OpenCall {
            call_id: fresh_id(),
            provider: provider.to_owned(),
            api,
            started_at: now(),
            // Codex names its session in a header; a Messages call names
            // its own in the body, read as the request passes.
            session: request
                .headers()
                .get(CODEX_SESSION)
                .and_then(|value| value.to_str().ok())
                .and_then(|id| crate::proxy::link::Link::from_record(Some(id)).to_record()),
            run: run.map(str::to_owned),
            admission_ns: None,
            completed: None,
        };
        // Admission gate: the request waits only for its journal record to
        // be put in place. The sync that makes the record hold through a
        // loss of power is made beside the call, never in its way.
        let admission = std::time::Instant::now();
        if let Err(error) = self.journal.put(&open) {
            return refusal(
                StatusCode::SERVICE_UNAVAILABLE,
                &format!("lys-proxy refused the call: {error}"),
            );
        }
        open.admission_ns = Some(u64::try_from(admission.elapsed().as_nanos()).unwrap_or(u64::MAX));
        self.journal.settle_beside(&open.call_id);
        let call = Call::admit(open, &self.capture, self.journal.clone(), self.sink.clone());
        call.asked(request.headers());
        let request = request.map(|body| Tee::new(body, call.request_side()).boxed_unsync());
        match self
            .account_send(base, &rest, request, drawn.as_ref())
            .await
        {
            Ok(response) => {
                let side = call.response_side(
                    is_event_stream(response.headers()),
                    response.status().as_u16(),
                    response.headers(),
                );
                response.map(|body| Tee::new(body, side).boxed_unsync())
            }
            Err(error) => {
                call.finish(End::Failed);
                refusal(
                    StatusCode::SERVICE_UNAVAILABLE,
                    &format!("lys-proxy: {error}"),
                )
            }
        }
    }

    fn select_account(
        &self,
        binding: Option<&crate::proxy::account_runs::Binding>,
        provider: &str,
        base: &Base,
        request: &mut Request<Incoming>,
    ) -> Result<
        Option<(
            crate::proxy::account::Broker,
            crate::proxy::account::Drawn,
            String,
        )>,
        ProxyError,
    > {
        let drawn = if let Some(binding) = binding {
            if binding.provider != base.as_str()
                && !(provider == "openai" && binding.provider == self.chatgpt.as_str())
            {
                return Err(ProxyError::ModelAccount {
                    account: "run".to_owned(),
                    reason: "account_provider_mismatch".to_owned(),
                });
            }
            Some(self.accounts.drawn(binding, request.headers())?)
        } else {
            let selector = request
                .headers()
                .get("authorization")
                .or_else(|| request.headers().get("x-api-key"))
                .and_then(|v| v.to_str().ok());
            if selector.is_some_and(|v| {
                v.strip_prefix("Bearer ")
                    .unwrap_or(v)
                    .starts_with("lys-account.")
            }) {
                return Err(ProxyError::ModelAccount {
                    account: "run".to_owned(),
                    reason: "account_binding_missing".to_owned(),
                });
            }
            None
        };
        request.headers_mut().remove("lys-model-account");
        if let Some((_, _, source)) = &drawn {
            let value =
                HeaderValue::from_str(source).map_err(|error| ProxyError::ModelAccount {
                    account: source.clone(),
                    reason: format!("account_handle_invalid: {error}"),
                })?;
            request.headers_mut().insert("lys-model-account", value);
        }
        Ok(drawn)
    }

    async fn account_send(
        &self,
        base: &Base,
        rest: &str,
        mut request: Request<ProxyBody>,
        drawn: Option<&(
            super::super::account::Broker,
            super::super::account::Drawn,
            String,
        )>,
    ) -> Result<Response<Incoming>, ProxyError> {
        request.headers_mut().remove("lys-model-account");
        match drawn {
            Some((broker, draw, _)) => {
                request.headers_mut().remove("authorization");
                request.headers_mut().remove("x-api-key");
                super::super::account::through_broker(&self.upstream, broker, draw, rest, request)
                    .await
            }
            None => self.upstream.send(base, rest, request).await,
        }
    }
}
