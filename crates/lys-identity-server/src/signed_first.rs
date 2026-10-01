//! Authentication before the body. Every route the table names with no
//! public door is refused to a caller who carries none of the doors it
//! names, before any extractor reads the request, so an anonymous caller
//! learns that it is not signed in and never how its body was malformed.
//!
//! A HEAD is judged as the GET it is served by.
//!
//! A session is judged here in full, since a session is one lookup. A
//! credential in the Authorization header or an agent's signature is only
//! seen to be present, whatever doors the table names: the route judges it,
//! as it must, over the request it signs, and several routes take an app's
//! or a registrar's credential beside a session.

use std::collections::HashMap;
use std::sync::Arc;

use axum::Router;
use axum::extract::{MatchedPath, Request, State};
use axum::http::{HeaderMap, Method as HttpMethod, header};
use axum::middleware::{Next, from_fn_with_state};
use axum::response::{IntoResponse, Response};
use lys_openapi::{Auth, Method};

use crate::error::ServerError;
use crate::routes::{AppState, signed_in};

/// The prefix the screens serve the routes under, when they are served.
const NESTED: &str = "/api";

/// Each route's doors, by method and path as the table writes them.
struct Doors {
    state: Arc<AppState>,
    doors: HashMap<(HttpMethod, &'static str), &'static [Auth]>,
}

/// `api` with every table route's doors checked before its handler runs.
pub(crate) fn guarded(api: Router, state: Arc<AppState>) -> Router {
    let doors = crate::openapi::api()
        .routes()
        .iter()
        .map(|route| ((http_method(route.method), route.path), route.auth))
        .collect();
    api.route_layer(from_fn_with_state(Arc::new(Doors { state, doors }), check))
}

fn http_method(method: Method) -> HttpMethod {
    match method {
        Method::Get => HttpMethod::GET,
        Method::Post => HttpMethod::POST,
        Method::Put => HttpMethod::PUT,
    }
}

async fn check(
    State(doors): State<Arc<Doors>>,
    matched: Option<MatchedPath>,
    request: Request,
    next: Next,
) -> Response {
    if request.headers().contains_key("lys-agent-pass") {
        let method = if request.method() == HttpMethod::HEAD {
            "GET"
        } else {
            request.method().as_str()
        };
        let path = request
            .uri()
            .path()
            .strip_prefix(NESTED)
            .unwrap_or(request.uri().path());
        if let Err(refusal) =
            crate::route_actions::admit(&doors.state, method, path, request.headers())
        {
            return refusal;
        }
        return next.run(request).await;
    }
    if request
        .extensions()
        .get::<crate::agent_signature::TokenPrincipal>()
        .is_some()
        || (request.method() == HttpMethod::POST
            && matches!(request.uri().path(), "/mcp" | "/api/mcp")
            && request.headers().contains_key(crate::grant_tokens::HEADER))
    {
        return next.run(request).await;
    }
    let named = matched.and_then(|matched| {
        let path = matched.as_str();
        let method = match request.method() {
            &HttpMethod::HEAD => HttpMethod::GET,
            method => method.clone(),
        };
        doors
            .doors
            .get(&(method.clone(), path))
            .or_else(|| {
                let inner = path.strip_prefix(NESTED)?;
                doors.doors.get(&(method, inner))
            })
            .copied()
    });
    match named.map(|auth| admitted(&doors.state, auth, request.headers())) {
        Some(Err(refusal)) => refusal.into_response(),
        Some(Ok(())) | None => next.run(request).await,
    }
}

/// Whether the request may reach its route: a public route, or a caller
/// that brings a credential or a signature for the route to judge, or a
/// live session; otherwise the session's own refusal.
fn admitted(state: &AppState, auth: &[Auth], headers: &HeaderMap) -> Result<(), ServerError> {
    if headers.contains_key(header::COOKIE) && headers.contains_key(crate::agent_signature::HEADER)
    {
        return Err(ServerError::AgentSignatureRefused {
            reason: "an agent signature cannot carry a session cookie",
        });
    }
    let brings = headers.contains_key(header::AUTHORIZATION)
        || headers.contains_key(crate::agent_signature::HEADER);
    if brings || auth.contains(&Auth::Public) {
        return Ok(());
    }
    signed_in(state, headers).map(drop)
}
