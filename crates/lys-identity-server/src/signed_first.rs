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
//!
//! The body is read here only for a caller not yet judged. A caller judged
//! in full before its body, by a session, the operator's token or a run
//! pass, has its body read whole, however long. Any other caller, one whose
//! credential or signature the route judges over the body or one on a
//! public route, has no more than [`UNVERIFIED_BODY_LIMIT`] bytes kept, and
//! a longer body is refused `BodyTooLarge` before any route reads it.
//!
//! Every refusal answered here first reads the rest of the body and drops
//! it, so the caller finishes sending and receives the refusal: answering
//! while the caller still sends closes the connection under it, and the
//! caller sees a reset in place of the refusal.

use std::collections::HashMap;
use std::future::poll_fn;
use std::pin::Pin;
use std::sync::Arc;

use axum::Router;
use axum::body::{Body, Bytes, HttpBody as _};
use axum::extract::{DefaultBodyLimit, MatchedPath, Request, State};
use axum::http::{HeaderMap, Method as HttpMethod, header};
use axum::middleware::{Next, from_fn_with_state};
use axum::response::{IntoResponse, Response};
use lys_openapi::{Auth, Method};

use crate::error::ServerError;
use crate::routes::{AppState, signed_in};

/// The prefix the screens serve the routes under, when they are served.
const NESTED: &str = "/api";

/// The most bytes of a body read before its caller is verified.
pub(crate) const UNVERIFIED_BODY_LIMIT: usize = 2 * 1024 * 1024;

/// Each route's doors, by method and path as the table writes them.
struct Doors {
    state: Arc<AppState>,
    doors: HashMap<(HttpMethod, &'static str), &'static [Auth]>,
}

/// `api` with every table route's doors checked before its handler runs,
/// and no body limit of the framework's own: what is read of a body is
/// decided here, by who the caller is.
pub(crate) fn guarded(api: Router, state: Arc<AppState>) -> Router {
    let doors = crate::openapi::api()
        .routes()
        .iter()
        .map(|route| ((http_method(route.method), route.path), route.auth))
        .collect();
    api.route_layer(from_fn_with_state(Arc::new(Doors { state, doors }), check))
        .layer(DefaultBodyLimit::disable())
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
    if request.headers().contains_key(crate::agent_pass::HEADER) {
        if let Err(error) = pass_credentials(request.headers()) {
            return refuse(request, error.into_response()).await;
        }
        let method = pass_method(request.method());
        let path = request
            .uri()
            .path()
            .strip_prefix(NESTED)
            .unwrap_or(request.uri().path());
        if let Err(refusal) =
            crate::route_actions::admit(&doors.state, method, path, request.headers())
        {
            return refuse(request, *refusal).await;
        }
        // The pass, its seat and its grant are judged before the body.
        return next.run(request).await;
    }
    // A call relayed from an MCP message: its body is the message's, read
    // already under the guard its own caller was given.
    if request
        .extensions()
        .get::<crate::agent_signature::TokenPrincipal>()
        .is_some()
        || crate::agent_signature::relayed_agent().is_some()
    {
        return next.run(request).await;
    }
    if request.method() == HttpMethod::POST
        && matches!(request.uri().path(), "/mcp" | "/api/mcp")
        && request.headers().contains_key(crate::grant_tokens::HEADER)
    {
        return unverified(request, next).await;
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
    let whole = match named {
        Some(auth) => match admitted(&doors.state, auth, request.headers()) {
            Ok(whole) => whole,
            Err(refusal) => return refuse(request, refusal.into_response()).await,
        },
        None => judged(&doors.state, request.headers()),
    };
    if whole {
        return next.run(request).await;
    }
    unverified(request, next).await
}

/// Whether the request may reach its route: a public route, or a caller
/// that brings a credential or a signature for the route to judge, or a
/// live session; otherwise the session's own refusal. `true` when the
/// caller is judged in full here, before its body is read.
fn admitted(state: &AppState, auth: &[Auth], headers: &HeaderMap) -> Result<bool, ServerError> {
    if headers.contains_key(header::COOKIE) && headers.contains_key(crate::agent_signature::HEADER)
    {
        return Err(ServerError::AgentSignatureRefused {
            reason: "an agent signature cannot carry a session cookie",
        });
    }
    if brings(headers) {
        return Ok(false);
    }
    if auth.contains(&Auth::Public) {
        return Ok(judged(state, headers));
    }
    signed_in(state, headers).map(|_actor| true)
}

/// Whether the request carries a credential or a signature that its route
/// judges over the body.
fn brings(headers: &HeaderMap) -> bool {
    headers.contains_key(header::AUTHORIZATION)
        || headers.contains_key(crate::agent_signature::HEADER)
}

/// Whether the caller is judged in full before its body is read: a live
/// session or the operator, bringing nothing the route judges over the
/// body. A caller that is not is answered by its route, as before; only
/// its body is guarded.
fn judged(state: &AppState, headers: &HeaderMap) -> bool {
    !brings(headers) && signed_in(state, headers).is_ok()
}

/// Run `request` for a caller not yet verified: its body is read here, no
/// more than [`UNVERIFIED_BODY_LIMIT`] bytes of it, and handed on whole, or
/// refused by name before any route reads it.
async fn unverified(request: Request, next: Next) -> Response {
    let (parts, body) = request.into_parts();
    match bounded(body).await {
        Ok(bytes) => {
            next.run(Request::from_parts(parts, Body::from(bytes)))
                .await
        }
        Err(refusal) => refusal.into_response(),
    }
}

/// The bytes of `body`, refused `BodyTooLarge` when a frame would take them
/// past [`UNVERIFIED_BODY_LIMIT`]. From that frame on nothing is kept: what
/// was read is dropped, and the rest of the body is [`drained`], so the
/// caller receives the refusal. A frame that is not data, a trailer, is not
/// kept: no route reads one.
async fn bounded(mut body: Body) -> Result<Bytes, ServerError> {
    let mut read = Vec::new();
    while let Some(frame) = poll_fn(|context| Pin::new(&mut body).poll_frame(context)).await {
        let frame = frame.map_err(|error| ServerError::RequestMalformed {
            reason: format!("the request body could not be read: {error}"),
        })?;
        if let Ok(data) = frame.into_data() {
            if data.len() > UNVERIFIED_BODY_LIMIT - read.len() {
                drop(read);
                drained(body).await;
                return Err(ServerError::BodyTooLarge);
            }
            read.extend_from_slice(&data);
        }
    }
    Ok(Bytes::from(read))
}

/// `refusal`, answered once the body of `request` is [`drained`].
async fn refuse(request: Request, refusal: Response) -> Response {
    drained(request.into_body()).await;
    refusal
}

/// Read `body` to its end, one frame at a time, keeping none of it, so the
/// caller finishes sending before it is answered. The body is refused
/// already; how its rest ends changes nothing, so an error ends the read.
async fn drained(mut body: Body) {
    while let Some(Ok(_)) = poll_fn(|context| Pin::new(&mut body).poll_frame(context)).await {}
}

fn pass_credentials(headers: &HeaderMap) -> Result<(), ServerError> {
    if [
        header::COOKIE,
        header::AUTHORIZATION,
        axum::http::HeaderName::from_static(crate::agent_signature::HEADER),
    ]
    .iter()
    .any(|name| headers.contains_key(name))
    {
        return Err(ServerError::AgentPassRefused {
            reason: "a run pass cannot be combined with another credential".to_owned(),
        });
    }
    Ok(())
}

fn pass_method(method: &HttpMethod) -> &str {
    if method == HttpMethod::HEAD {
        "GET"
    } else {
        method.as_str()
    }
}

#[cfg(test)]
#[path = "signed_first_tests.rs"]
mod tests;
