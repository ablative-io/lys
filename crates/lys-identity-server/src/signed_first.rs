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
use crate::routes::{AppState, cookie_header};

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
    let brings = headers.contains_key(header::AUTHORIZATION)
        || headers.contains_key(crate::agent_signature::HEADER);
    if brings || auth.contains(&Auth::Public) {
        return Ok(());
    }
    state.sessions.actor(cookie_header(headers)).map(drop)
}
