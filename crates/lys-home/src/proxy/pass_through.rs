//! The forwarding gate records the wait for the upstream response head.

use std::time::Instant;

use http_body_util::BodyExt;
use hyper::body::Incoming;
use hyper::{Request, Response, StatusCode};

use super::{Base, ProxyBody, Upstream, refusal};

/// Gate: forward `request` to `base` unchanged and return the
/// response unchanged. One line per call is written on stderr, when the
/// response head arrives (or the upstream fails): its method, its path
/// (never its query), its status and the milliseconds until the head, and
/// never a header value or a body byte. A failure to reach the upstream is
/// answered 502, naming the failure.
pub async fn pass_through(
    upstream: &Upstream,
    base: &Base,
    request: Request<Incoming>,
) -> Response<ProxyBody> {
    let started = Instant::now();
    let method = request.method().as_str().to_owned();
    let path = request.uri().path().to_owned();
    let path_and_query = request
        .uri()
        .path_and_query()
        .map_or_else(|| path.clone(), |pq| pq.as_str().to_owned());
    let request = request.map(BodyExt::boxed_unsync);
    let (status, response) = match upstream.send(base, &path_and_query, request).await {
        Ok(response) => (response.status(), response.map(BodyExt::boxed_unsync)),
        Err(error) => (
            StatusCode::BAD_GATEWAY,
            refusal(StatusCode::BAD_GATEWAY, &format!("pass-through: {error}")),
        ),
    };
    let duration_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
    let line = serde_json::json!({
        "method": method,
        "path": path,
        "status": status.as_u16(),
        "duration_ms": duration_ms,
    });
    eprintln!("{line}");
    response
}
