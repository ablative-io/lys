//! A request is admitted before its credential is presented, then settled from the upstream answer.
use super::{
    Admitted, Arc, Bytes, Checked, MAX_BODY, Refused, Request, Response, Route, SecretsError,
    Settled, Shared, StatusCode, ask, call_upstream, header, on_broker, signed,
};
use axum::response::IntoResponse;
type Called = Result<(Response, Option<u64>), Refused>;
pub(crate) async fn forward(shared: &Arc<Shared>, request: Request) -> Result<Response, Refused> {
    let bad = |error: SecretsError| (StatusCode::BAD_REQUEST, error);
    let internal = |error: SecretsError| (StatusCode::INTERNAL_SERVER_ERROR, error);
    let (parts, body) = request.into_parts();
    let path = parts.uri.path().trim_start_matches('/');
    let (secret, rest) = path.split_once('/').unwrap_or((path, ""));
    let route = shared
        .layout
        .routes()
        .map_err(internal)?
        .get(secret)
        .map(Arc::clone)
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                SecretsError::SecretUnknown {
                    name: secret.to_owned(),
                },
            )
        })?;
    // Guarded: read before the caller is known, as the caller's signature covers it.
    let body = axum::body::to_bytes(body, MAX_BODY)
        .await
        .map_err(|error| {
            bad(SecretsError::Encoding {
                context: "request body",
                reason: error.to_string(),
            })
        })?;
    let (token, presentation) = signed(&parts, &body)?;
    let reserve = match header(&parts.headers, "lys-reserve") {
        None => 0,
        Some(text) => text.parse::<u64>().map_err(|error| {
            bad(SecretsError::Encoding {
                context: "lys-reserve",
                reason: format!("not a whole number: {error}"),
            })
        })?,
    };
    let (token, asks) = on_broker(shared, move |broker| {
        let asks = broker.asks_for(&token);
        (token, asks)
    })
    .await
    .map_err(internal)?;
    let checked = ask(shared, &asks).await.map_err(internal)?;
    let named = secret.to_owned();
    let admitted = on_broker(shared, move |broker| {
        broker.admit_use_for_checked(&token, &presentation, &named, reserve, &checked)
    })
    .await
    .map_err(internal)?
    .map_err(|error| (StatusCode::FORBIDDEN, error))?;
    let ticket = match admitted {
        Admitted::Fresh(ticket) => ticket,
        Admitted::Retried { outcome } => {
            return Ok((
                StatusCode::CONFLICT,
                format!("already presented; first outcome: {outcome}\n"),
            )
                .into_response());
        }
    };
    let checked = ask(shared, &asks).await.map_err(internal)?;
    let ticket = on_broker(shared, move |broker| {
        broker.at_forward_boundary_checked(ticket, &checked)
    })
    .await
    .map_err(internal)?
    .map_err(|error| (StatusCode::FORBIDDEN, error))?;
    let rest = rest.to_owned();
    let reserved = ticket.reserved().unwrap_or(0);
    let (ticket, called) = forward_ticket(shared, &route, parts, &rest, body, ticket).await;
    let Some(ticket) = ticket else {
        return called.map(|answered| answered.0);
    };
    let settled = match &called {
        Ok((_, Some(spent))) => Settled::Spent(*spent),
        Ok((_, None)) => Settled::Unmetered,
        Err(_) => Settled::Failed(reserved),
    };
    let checked = match settled {
        Settled::Failed(_) => Checked::default(),
        Settled::Spent(_) | Settled::Unmetered => ask(shared, &asks).await.map_err(internal)?,
    };
    on_broker(shared, move |broker| {
        broker.settle_checked(ticket, settled, &checked)
    })
    .await
    .map_err(internal)?
    .map_err(internal)?;
    called.map(|answered| answered.0)
}

async fn forward_ticket(
    shared: &Arc<Shared>,
    route: &Route,
    parts: axum::http::request::Parts,
    rest: &str,
    body: Bytes,
    ticket: lys_secrets::Ticket,
) -> (Option<lys_secrets::Ticket>, Called) {
    let internal = |error: SecretsError| (StatusCode::INTERNAL_SERVER_ERROR, error);
    let oauth = ticket.oauth();
    match oauth {
        Err(error) => (Some(ticket), Err(internal(error))),
        Ok(None) => {
            let credential = ticket.credential();
            let called =
                call_upstream(shared, route, parts, rest, body, credential, &[credential]).await;
            (Some(ticket), called)
        }
        Ok(Some(grant)) => {
            let (ticket, lived) = crate::oauth_proxy::live(shared, ticket, grant).await;
            let called = match lived {
                Err(error) => Err(error),
                Ok((grant, retired)) => {
                    let mut tokens = grant.tokens();
                    tokens.extend(retired.iter());
                    call_upstream(
                        shared,
                        route,
                        parts,
                        rest,
                        body,
                        grant.access_token(),
                        &tokens,
                    )
                    .await
                }
            };
            (ticket, called)
        }
    }
}
