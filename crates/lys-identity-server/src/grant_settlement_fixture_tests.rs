fn wire(value: impl serde::Serialize) -> Result<serde_json::Value, ServerError> {
    serde_json::to_value(value).map_err(|error| ServerError::RequestMalformed {
        reason: format!("grant fixture answer could not be encoded: {error}"),
    })
}

fn refusal() -> GrantError {
    GrantError::PermissionEngineUnavailable {
        reason: "instance-private settlement refusal".to_owned(),
    }
}

async fn discover(
    config: &crate::Config,
    listener: tokio::net::TcpListener,
) -> TestResult<crate::oidc::Oidc> {
    let issuer = &config.issuer;
    let document = json!({
        "issuer": issuer,
        "authorization_endpoint": format!("{issuer}/authorize"),
        "token_endpoint": format!("{issuer}/token"),
        "jwks_uri": format!("{issuer}/jwks"),
        "response_types_supported": ["code"],
        "subject_types_supported": ["public"],
        "id_token_signing_alg_values_supported": ["EdDSA"],
    });
    let router = axum::Router::new()
        .route(
            "/.well-known/openid-configuration",
            axum::routing::get(move || {
                let answer = document.clone();
                async move { Json(answer) }
            }),
        )
        .route(
            "/jwks",
            axum::routing::get(|| async { Json(json!({ "keys": [] })) }),
        );
    let worker = tokio::spawn(async move { axum::serve(listener, router).await });
    let result = crate::oidc::Oidc::discover(config).await;
    worker.abort();
    match worker.await {
        Ok(ended) => ended?,
        Err(error) if error.is_cancelled() => {}
        Err(error) => return Err(error.into()),
    }
    Ok(result?)
}

struct Table {
    state: Arc<AppState>,
    fault: Arc<AtomicBool>,
    engine: MemoryRelationships,
    dir: tempfile::TempDir,
    asker: PersonId,
    agent: AgentId,
    holder: AgentId,
    actor: Actor,
}

struct ReadyTable {
    state: AppState,
    fault: Arc<AtomicBool>,
    engine: MemoryRelationships,
    asker: PersonId,
    agent: AgentId,
    holder: AgentId,
    actor: Actor,
}

impl Table {
    fn projection_pending(&self) -> TestResult {
        let path = self.dir.path().join("grants");
        let administrator = self
            .state
            .admission
            .administrator_login()?
            .ok_or("administrator absent")?;
        let root = self
            .state
            .directory
            .lock()
            .map_err(|error| error.to_string())?
            .projection()?
            .person_for(&administrator)
            .ok_or("administrator unbound")?;
        let grants = Grants::open(
            Box::new(move || FileLeafStore::open(&path)),
            Ed25519Identity::load(&self.state.grant_setup.key_file)?,
            Relationships::faulted_projection(
                self.engine.clone(),
                Arc::clone(&self.fault),
                Arc::new(AtomicU64::new(0)),
            ),
            self.state.grant_setup.model()?,
            root,
        )?;
        *self
            .state
            .grants
            .lock()
            .map_err(|error| error.to_string())? = Some(grants);
        self.fault.store(true, Ordering::Release);
        crate::grants::with_grants(&self.state, |judged| {
            let request = RootRequest {
                operation: OperationId::generate()?,
                caller: IdentityId::Person(judged.root),
                route: Route::Api,
                holder: self.asker,
                resource: Resource::new("agent", &self.holder.to_string())?,
                relation: Relation::new("reader")?,
                pass_on: PassOn::UseOnly,
                window: Window::new(0, None)?,
            };
            match judged
                .grants
                .issue_root(judged.directory, &request, crate::session::now())
            {
                Err(GrantError::ProjectionPending { .. }) => Ok(()),
                Err(error) => Err(error.into()),
                Ok(_) => Err(ServerError::RequestMalformed {
                    reason: "pending projection was acknowledged".to_owned(),
                }),
            }
        })?;
        Ok(())
    }

    async fn fresh() -> TestResult<Self> {
        let dir = tempfile::TempDir::new()?;
        match Self::prepare(dir.path()).await {
            Ok(ready) => Ok(Self {
                state: Arc::new(ready.state),
                fault: ready.fault,
                engine: ready.engine,
                dir,
                asker: ready.asker,
                agent: ready.agent,
                holder: ready.holder,
                actor: ready.actor,
            }),
            Err(error) => match dir.close() {
                Ok(()) => Err(error),
                Err(cleanup) => Err(format!(
                    "fixture construction failed: {error}; fixture cleanup failed: {cleanup}"
                )
                .into()),
            },
        }
    }

    async fn prepare(path: &Path) -> TestResult<ReadyTable> {
        let fault = Arc::new(AtomicBool::new(false));
        let engine = MemoryRelationships::default();
        let key_file = path.join("service.key");
        let model_file = path.join("model.json");
        std::fs::write(
            &model_file,
            br#"{"version":1,"relations":{"administrator":["organisation.administer","organisation.admit","organisation.grant","organisation.read"],"operator":["read","operate"],"reader":["read"]}}"#,
        )?;
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let issuer = format!("http://{}", listener.local_addr()?);
        let config: crate::Config = serde_json::from_value(json!({
            "listen": "127.0.0.1:0", "log_dir": path.join("log"),
            "log_origin": "example.test/lys/directory", "event_key_file": key_file,
            "issuer": issuer, "client_id": "settlement-test", "client_secret_file": path.join("client-secret"),
            "redirect_url": "http://127.0.0.1/callback", "reviews": {"period_days": 30}, "session_seconds": 300, "secure_cookie": false,
            "link_audit_source": {"issuer": issuer, "subject": "audit"},
            "grant_log_dir": path.join("grants"), "grant_log_origin": "example.test/lys/grants",
            "grant_model_file": model_file,
            "stores_receipt_file": path.join("install").join("stores.json"),
            "stores_accepted_file": path.join("install").join("stores.accepted.json"),
        }))?;
        std::fs::write(&config.client_secret_file, b"synthetic-test-credential")?;
        let oidc = discover(&config, listener).await?;
        let mut authority = seeded_authority(&config)?;
        let model: Model = config.grant_model()?;
        let grants = fixture_grants(&config, &mut authority, &engine, &fault, &model)?;
        let (asker, agent, holder) = (authority.asker, authority.agent, authority.holder);
        let actor = authority.actor.clone();
        let state = fixture_state(path, &config, oidc, authority, grants, model)?;
        Ok(ReadyTable {
            state,
            fault,
            engine,
            asker,
            agent,
            holder,
            actor,
        })
    }

    fn count(&self) -> TestResult<u64> {
        Ok(FileLeafStore::open(&self.dir.path().join("grants"))?.extent())
    }

    fn headers(&self) -> TestResult<HeaderMap> {
        let cookie = self.state.sessions.begin(self.actor.clone())?;
        let mut headers = HeaderMap::new();
        headers.insert(
            header::COOKIE,
            cookie
                .split(';')
                .next()
                .ok_or("session cookie absent")?
                .parse()?,
        );
        Ok(headers)
    }

    fn close(self) -> TestResult {
        self.fault.store(false, Ordering::Release);
        drop(self.state);
        self.dir.close()?;
        Ok(())
    }

    fn finish<T>(self, result: TestResult<T>) -> TestResult<T> {
        match (result, self.close()) {
            (Ok(value), Ok(())) => Ok(value),
            (Err(error), Ok(())) | (Ok(_), Err(error)) => Err(error),
            (Err(error), Err(cleanup)) => Err(format!(
                "fixture act failed: {error}; fixture cleanup failed: {cleanup}"
            )
            .into()),
        }
    }

    fn goal(&self) -> Item {
        Item {
            goal: Goal {
                id: "judgement".to_owned(),
                holder: Holder {
                    kind: HolderKind::Agent,
                    id: self.agent.to_string(),
                },
                kind: Kind::Goal,
                words: "Read the decision".to_owned(),
                deadline: None,
                active: true,
                evidence: None,
                judged_by: Some("read".to_owned()),
                reminders: Vec::new(),
                responsible: IdentityId::Agent(self.agent).to_string(),
                set_by: self.asker.to_string(),
                at: 0,
            },
            standing: Standing::Open,
            marked: None,
            timers: Vec::new(),
            fired: Vec::new(),
            changes: Vec::new(),
        }
    }
}

include!("grant_settlement_seed_tests.rs");
