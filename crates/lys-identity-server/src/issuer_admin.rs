//! Native issuer routes keep native authentication and stream both bodies.

use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::extract::{ConnectInfo, Request, State};
use axum::http::{HeaderMap, HeaderName, HeaderValue, Uri, header};
use axum::response::Response;
use axum::routing::any;

use crate::config::Config;
use crate::error::ServerError;

struct Proxy {
    http: reqwest::Client,
    upstream: reqwest::Url,
    public: reqwest::Url,
    host: HeaderValue,
    trusted: Vec<IpAddr>,
}

fn refused(reason: &str) -> ServerError {
    ServerError::SignInProvidersUnavailable { reason: reason.to_owned() }
}

fn owns(path: &str) -> bool {
    path == "/auth/v1" || path.starts_with("/auth/v1/")
}

fn decoded_path(path: &str) -> Result<Vec<u8>, ServerError> {
    let mut decoded = Vec::with_capacity(path.len());
    let mut bytes = path.as_bytes().iter().copied();
    while let Some(byte) = bytes.next() {
        if byte == b'%' {
            let high = bytes.next().and_then(|byte| char::from(byte).to_digit(16));
            let low = bytes.next().and_then(|byte| char::from(byte).to_digit(16));
            let value = high.zip(low).and_then(|(high, low)| u8::try_from(high * 16 + low).ok())
                .ok_or_else(|| refused("invalid encoded issuer path"))?;
            decoded.push(value);
        } else {
            decoded.push(byte);
        }
    }
    Ok(decoded)
}

fn target(upstream: &reqwest::Url, uri: &Uri) -> Result<reqwest::Url, ServerError> {
    let path = uri.path();
    let decoded = decoded_path(path)?;
    let canonical = decoded.split(|byte| *byte == b'/').filter(|part| !part.is_empty()).collect::<Vec<_>>();
    if !owns(path) || canonical.starts_with(&[b"auth".as_slice(), b"v1", b"providers", b"callback"]) {
        return Err(refused("the requested route belongs to Lys, not the native issuer proxy"));
    }
    if decoded.contains(&b'\\') || canonical.iter().any(|part| matches!(*part, b"." | b"..")) {
        return Err(refused("the native issuer path contains a traversal"));
    }
    let mut target = upstream.clone();
    let suffix = path.strip_prefix("/auth/v1").ok_or_else(|| refused("invalid native issuer route"))?;
    let native_path = format!("{}{suffix}", upstream.path().trim_end_matches('/'));
    let native_path = if native_path.is_empty() { "/".to_owned() } else { native_path };
    target.set_path(&native_path);
    target.set_query(uri.query());
    if target.path() != native_path {
        return Err(refused("the native issuer path changes when normalized"));
    }
    Ok(target)
}

fn response_headers(headers: &HeaderMap, upstream: &reqwest::Url, public: &reqwest::Url) -> Result<HeaderMap, ServerError> {
    let mut carried = end_to_end(headers)?;
    if let Some(location) = carried.get(header::LOCATION) {
        let location = location.to_str().map_err(|error| refused(&format!("invalid native issuer redirect: {error}")))?;
        if let Ok(native) = reqwest::Url::parse(location)
            && native.origin() == upstream.origin()
            && let Some(suffix) = native.path().strip_prefix(upstream.path().trim_end_matches('/'))
            && (suffix.is_empty() || suffix.starts_with('/')) {
            let mut redirected = public.clone();
            redirected.set_path(&format!("/auth/v1{suffix}"));
            redirected.set_query(native.query());
            redirected.set_fragment(native.fragment());
            carried.insert(header::LOCATION, HeaderValue::from_str(redirected.as_str())
                .map_err(|error| refused(&format!("invalid public issuer redirect: {error}")))?);
        }
    }
    Ok(carried)
}

fn end_to_end(headers: &HeaderMap) -> Result<HeaderMap, ServerError> {
    let mut carried = headers.clone();
    for connection in headers.get_all(header::CONNECTION).iter() {
        for name in connection.to_str().map_err(|error| refused(&format!("invalid connection header: {error}")))?.split(',') {
            let name = HeaderName::from_bytes(name.trim().as_bytes())
                .map_err(|error| refused(&format!("invalid connection header name: {error}")))?;
            carried.remove(name);
        }
    }
    for name in ["connection", "keep-alive", "proxy-authenticate", "proxy-authorization", "te", "trailer", "transfer-encoding", "upgrade", "content-length"] {
        carried.remove(name);
    }
    Ok(carried)
}

fn request_headers(headers: &HeaderMap) -> Result<HeaderMap, ServerError> {
    let mut carried = end_to_end(headers)?;
    for name in ["host", "forwarded", "x-forwarded-for", "x-forwarded-host", "x-forwarded-proto", "x-forwarded-port", "x-forwarded-prefix", "x-forwarded-server", "x-real-ip", "lys-operator", "x-lys-operator", "lys-import", "x-api-key"] {
        carried.remove(name);
    }
    let cookies = carried.get_all(header::COOKIE).iter().map(|value|
        value.to_str().map_err(|error| refused(&format!("invalid issuer cookie header: {error}"))))
        .collect::<Result<Vec<_>, _>>()?;
    let cookies = cookies.iter().flat_map(|line| line.split(';')).map(str::trim)
        .filter(|pair| !pair.split_once('=').is_some_and(|(name, _)|
            matches!(name, "lys_directory_session" | "lys_provider")))
        .filter(|pair| !pair.is_empty()).collect::<Vec<_>>().join("; ");
    carried.remove(header::COOKIE);
    if !cookies.is_empty() {
        carried.insert(header::COOKIE, HeaderValue::from_str(&cookies)
            .map_err(|error| refused(&format!("invalid issuer cookie header: {error}")))?);
    }
    Ok(carried)
}

pub(super) fn routes(config: &Config) -> Result<Router, ServerError> {
    let upstream = reqwest::Url::parse(&config.sign_in_api())
        .map_err(|error| refused(&format!("invalid native issuer address: {error}")))?;
    let public = reqwest::Url::parse(&crate::sign_in::lys_origin(config)?)
        .map_err(|error| refused(&format!("invalid public issuer origin: {error}")))?;
    if !matches!(upstream.scheme(), "http" | "https") || upstream.host_str().is_none()
        || !upstream.username().is_empty() || upstream.password().is_some()
        || upstream.query().is_some() || upstream.fragment().is_some() {
        return Err(refused("the native issuer address must name its auth API without credentials or query"));
    }
    let http = reqwest::Client::builder().redirect(reqwest::redirect::Policy::none())
        .no_proxy().no_gzip().no_brotli().no_deflate().no_zstd()
        .user_agent("lys-issuer-admin").build()
        .map_err(|error| refused(&format!("the native issuer HTTP client could not be built: {}", error.without_url())))?;
    let origin = public.origin().ascii_serialization();
    let host = origin.split_once("://").map(|(_, host)| host)
        .ok_or_else(|| refused("the public issuer has no HTTP authority"))?;
    let host = HeaderValue::from_str(host).map_err(|error| refused(&format!("invalid public issuer host: {error}")))?;
    let proxy = Arc::new(Proxy { http, upstream, public, host, trusted: config.trusted_proxies.clone() });
    Ok(Router::new().route("/auth/v1", any(carry))
        .route("/auth/v1/{*path}", any(carry)).with_state(proxy))
}

async fn carry(State(proxy): State<Arc<Proxy>>, request: Request) -> Result<Response, ServerError> {
    let target = target(&proxy.upstream, request.uri())?;
    let peer = request.extensions().get::<ConnectInfo<SocketAddr>>()
        .ok_or_else(|| refused("the native issuer request has no peer address"))?.0.ip();
    let address = crate::sign_in_address::resolve(peer, request.headers(), &proxy.trusted)?;
    let headers = request_headers(request.headers())?;
    let (parts, body) = request.into_parts();
    let answer = proxy.http.request(parts.method, target).headers(headers)
        .header(header::HOST, proxy.host.clone())
        .header("x-forwarded-host", proxy.host.clone())
        .header("x-forwarded-proto", proxy.public.scheme())
        .header("x-forwarded-for", address.to_string())
        .body(reqwest::Body::wrap_stream(body.into_data_stream()))
        .send().await.map_err(|error| refused(&format!("the native issuer could not answer the request: {}", error.without_url())))?;
    let status = answer.status();
    let headers = response_headers(answer.headers(), &proxy.upstream, &proxy.public)?;
    let mut response = Response::new(Body::new(reqwest::Body::from(answer)));
    *response.status_mut() = status;
    *response.headers_mut() = headers;
    Ok(response)
}

#[cfg(test)]
#[path = "issuer_admin_tests.rs"]
mod tests;
