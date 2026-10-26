//! Lys's model proxy for a run on any computer.
//!
//! Every run Lys starts makes its model calls through Lys's proxy, under the
//! key its start minted. The proxy listens on loopback beside this server,
//! and a loopback address names nothing on a computer whose runner dials in
//! from elsewhere: a run started there was refused its every call. Such a run
//! is given Lys's own public origin instead, the address that computer
//! already reaches Lys at, and this server carries a keyed model call, and
//! only a keyed one, to the proxy as it came, streamed both ways. An unkeyed
//! path and the proxy's own control paths are never carried, so nothing
//! reaches the proxy from outside except a call under a key a start minted;
//! a key the proxy never bound draws no account.

use axum::body::Body;
use axum::extract::{Request, State};
use axum::http::{HeaderName, StatusCode, header};
use axum::middleware::Next;
use axum::response::Response;
use sha2::{Digest, Sha256};

use crate::config::Config;
use crate::error::ServerError;
use crate::routes::{AppState, Shared};
use crate::runner_client::RunnerRecord;

/// The address a run on a dialled-in computer is given: Lys's public origin
/// with the configured proxy's path. None when Lys runs no proxy.
///
/// # Errors
/// Refuses a redirect address that is not a URL, as sign-in does.
pub fn dialled(config: &Config) -> Result<Option<String>, ServerError> {
    let Some(proxy) = &config.model_proxy else {
        return Ok(None);
    };
    let origin = crate::sign_in::lys_origin(config)?;
    Ok(Some(format!("{origin}{}", path_of(proxy))))
}

/// The proxy address a start on `machine` is given: the dialled address
/// when the machine's runner dials in, the configured one otherwise.
///
/// # Errors
/// Names a network store that cannot be read.
pub fn for_machine<'a>(state: &'a AppState, machine: &str) -> Result<Option<&'a str>, ServerError> {
    let dialled = matches!(
        crate::runner_sessions::machine_runner(state, machine)?,
        Some(RunnerRecord::Dialled { .. })
    );
    Ok(if dialled {
        state.dialled_model_proxy.as_deref()
    } else {
        state.model_proxy.as_deref()
    })
}

/// Whether `path` is a keyed model call: `/<run key>/anthropic/...` or
/// `/<run key>/openai/...`.
#[must_use]
pub fn keyed_call(path: &str) -> bool {
    let (run, rest) = lys_home::proxy::forward::run_key(path);
    run.is_some() && (rest.starts_with("/anthropic/") || rest.starts_with("/openai/"))
}

/// Carries a keyed model call to the proxy beside this server; every other
/// request goes on to Lys's own routes.
pub async fn carry(State(state): State<Shared>, request: Request, next: Next) -> Response {
    crate::wait_request::scope(carry_in(state, request, next)).await
}

async fn carry_in(state: Shared, request: Request, next: Next) -> Response {
    if !keyed_call(request.uri().path()) {
        return next.run(request).await;
    }
    let Some(local) = state.model_proxy.as_deref() else {
        return refused(
            StatusCode::NOT_FOUND,
            "this Lys runs no model proxy, so a keyed model call has nowhere to go",
        );
    };
    forward(local, request).await
}

/// Sends `request` to the proxy at `local` as it came, keyed path and all,
/// and answers with the proxy's answer, streamed.
pub async fn forward(local: &str, request: Request) -> Response {
    let target = format!(
        "{}{}",
        origin_of(local),
        request
            .uri()
            .path_and_query()
            .map_or_else(|| request.uri().path(), |path| path.as_str())
    );
    let (parts, body) = request.into_parts();
    let client = match client() {
        Ok(client) => client,
        Err(error) => return refused(StatusCode::SERVICE_UNAVAILABLE, &error.to_string()),
    };
    let (run, path) = lys_home::proxy::forward::run_key(parts.uri.path());
    let subject = run.map_or_else(|| path.to_owned(), |key| format!("run {}", crate::routes::hex(&Sha256::digest(key.as_bytes()))));
    let mut call = match crate::waits::Call::begin("model upstream", origin_of(local), parts.method.as_str(), &subject) {
        Ok(call) => call,
        Err(error) => return refused(StatusCode::SERVICE_UNAVAILABLE, &error.to_string()),
    };
    let mut outgoing = client
        .request(parts.method, &target)
        .body(reqwest::Body::wrap_stream(body.into_data_stream()));
    for (name, value) in &parts.headers {
        if !hop(name) {
            outgoing = outgoing.header(name, value);
        }
    }
    let sent = call.until(outgoing.send()).await;
    match sent {
        Ok(Ok(answer)) => {
            let mut response = Response::builder().status(answer.status());
            for (name, value) in answer.headers() {
                if !hop(name) {
                    response = response.header(name, value);
                }
            }
            response
                .body(crate::outbound::body(answer, call))
                .unwrap_or_else(|error| {
                    refused(StatusCode::SERVICE_UNAVAILABLE, &error.to_string())
                })
        }
        Ok(Err(error)) => refused(
            StatusCode::SERVICE_UNAVAILABLE,
            &format!("Lys could not reach its model proxy at {local}: {error}"),
        ),
        Err(reason) => refused(StatusCode::SERVICE_UNAVAILABLE, &reason),
    }
}

/// Each carried call owns its client. A model call's stream
/// ends when the provider ends it or a side closes, never on a clock.
fn client() -> Result<reqwest::Client, reqwest::Error> {
    crate::outbound::http().build()
}

/// Headers that belong to one connection, not to the call: never carried.
/// A carried body is streamed, so its length is the stream's own.
fn hop(name: &HeaderName) -> bool {
    [
        header::HOST,
        header::CONNECTION,
        header::TRANSFER_ENCODING,
        header::TE,
        header::TRAILER,
        header::UPGRADE,
        header::PROXY_AUTHORIZATION,
        header::CONTENT_LENGTH,
    ]
    .contains(name)
        || name.as_str() == "keep-alive"
        || name.as_str() == "proxy-connection"
}

/// `scheme://authority` of an address, without its path.
fn origin_of(address: &str) -> &str {
    let start = address.find("://").map_or(0, |at| at + 3);
    address[start..]
        .find('/')
        .map_or(address, |at| &address[..start + at])
}

/// The path an address names after its authority, or nothing.
fn path_of(address: &str) -> &str {
    let start = address.find("://").map_or(0, |at| at + 3);
    address[start..]
        .find('/')
        .map_or("", |at| &address[start + at..])
}

fn refused(status: StatusCode, reason: &str) -> Response {
    let mut response = Response::new(Body::from(serde_json::json!({
        "refusal":"model_upstream_unreachable", "reason":format!("lys refused the model call: {reason}"),
    }).to_string()));
    *response.status_mut() = status;
    response.headers_mut().insert(header::CONTENT_TYPE, axum::http::HeaderValue::from_static("application/json"));
    response
}

#[cfg(test)]
#[path = "model_proxy_route_tests.rs"]
mod tests;
