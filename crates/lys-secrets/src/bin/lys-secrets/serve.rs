//! The proxy. A holder sends its request here with its handle and a signed
//! presentation; the broker admits the use, the credential is written into
//! the one header its route names, and the request goes to the one upstream
//! the secret is bound to. The credential never reaches the holder: every
//! occurrence of it in the upstream's answer is replaced before it returns.

use std::sync::{Arc, Mutex, PoisonError};

use axum::Router;
use axum::body::{Body, Bytes};
use axum::extract::{Request, State};
use axum::http::{HeaderMap, HeaderName, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use lys_secrets::{Broker, HandleToken, Presentation, Secret, SecretsError, Used, from_hex};

use crate::files::Layout;
use crate::spice::Grants;

const MAX_BODY: usize = 16 * 1024 * 1024;
const REDACTED: &[u8] = b"[redacted]";

pub(crate) struct Shared {
    pub(crate) broker: Mutex<Broker<Grants>>,
    pub(crate) layout: Layout,
    client: reqwest::Client,
}

pub fn serve(broker: Broker<Grants>, layout: Layout, listen: &str) -> Result<(), SecretsError> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|source| SecretsError::Io {
            context: "starting the proxy runtime".to_owned(),
            source,
        })?;
    let shared = Arc::new(Shared {
        broker: Mutex::new(broker),
        layout,
        client: reqwest::Client::new(),
    });
    runtime.block_on(async move {
        let listener = tokio::net::TcpListener::bind(listen)
            .await
            .map_err(|source| SecretsError::Io {
                context: format!("listening on {listen}"),
                source,
            })?;
        println!("lys-secrets proxy listening on {listen}");
        let app = Router::new()
            .route("/_lys/secrets", axum::routing::get(crate::view::secrets))
            .route("/_lys/audit", axum::routing::get(crate::view::audit))
            .route("/_lys/grants", axum::routing::get(crate::view::grants))
            .fallback(proxy)
            .with_state(shared);
        axum::serve(listener, app)
            .await
            .map_err(|source| SecretsError::Io {
                context: "serving the proxy".to_owned(),
                source,
            })
    })
}

fn refusal(status: StatusCode, error: &SecretsError) -> Response {
    (status, format!("{error}\n")).into_response()
}

fn header<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers.get(name).and_then(|value| value.to_str().ok())
}

async fn proxy(State(shared): State<Arc<Shared>>, request: Request) -> Response {
    match forward(&shared, request).await {
        Ok(response) => response,
        Err((status, error)) => refusal(status, &error),
    }
}

async fn forward(
    shared: &Shared,
    request: Request,
) -> Result<Response, (StatusCode, SecretsError)> {
    let bad = |error: SecretsError| (StatusCode::BAD_REQUEST, error);
    let (parts, body) = request.into_parts();
    let path = parts.uri.path().trim_start_matches('/');
    let (secret, rest) = path.split_once('/').unwrap_or((path, ""));
    let routes = shared
        .layout
        .routes()
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error))?;
    let route = routes.get(secret).cloned().ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            SecretsError::SecretUnknown {
                name: secret.to_owned(),
            },
        )
    })?;
    let unsigned = || SecretsError::PresentationInvalid {
        handle: String::new(),
    };
    let token = header(&parts.headers, "lys-handle")
        .and_then(from_hex)
        .ok_or_else(|| bad(SecretsError::HandleUnknown))?;
    let presentation = Presentation::from_wire(
        header(&parts.headers, "lys-handle-id").ok_or_else(|| bad(unsigned()))?,
        header(&parts.headers, "lys-operation").ok_or_else(|| bad(unsigned()))?,
        header(&parts.headers, "lys-signed-at").ok_or_else(|| bad(unsigned()))?,
        header(&parts.headers, "lys-presentation").ok_or_else(|| bad(unsigned()))?,
    )
    .map_err(bad)?;
    let used = {
        let mut broker = shared.broker.lock().unwrap_or_else(PoisonError::into_inner);
        broker.use_handle(
            &HandleToken::from_bytes(&token),
            &presentation,
            |credential| {
                let mut value = route.prefix.as_bytes().to_vec();
                value.extend_from_slice(credential.expose());
                (Secret::new(value), Secret::from_slice(credential.expose()))
            },
        )
    }
    .map_err(|error| (StatusCode::FORBIDDEN, error))?;
    let (header_value, credential) = match used {
        Used::Forwarded { answer, .. } => answer,
        Used::Retried { outcome } => {
            return Ok((
                StatusCode::CONFLICT,
                format!("already presented; first outcome: {outcome}\n"),
            )
                .into_response());
        }
    };
    let body = axum::body::to_bytes(body, MAX_BODY)
        .await
        .map_err(|error| {
            bad(SecretsError::Encoding {
                context: "request body",
                reason: error.to_string(),
            })
        })?;
    let query = parts
        .uri
        .query()
        .map(|query| format!("?{query}"))
        .unwrap_or_default();
    let url = format!("{}/{rest}{query}", route.upstream.trim_end_matches('/'));
    let mut outgoing = HeaderMap::new();
    for (name, value) in &parts.headers {
        let lowered = name.as_str();
        if lowered.starts_with("lys-")
            || lowered == "host"
            || lowered == route.header.to_ascii_lowercase()
        {
            continue;
        }
        outgoing.insert(name.clone(), value.clone());
    }
    let name = HeaderName::from_bytes(route.header.as_bytes()).map_err(|error| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            SecretsError::Encoding {
                context: "route header",
                reason: error.to_string(),
            },
        )
    })?;
    let mut value = HeaderValue::from_bytes(header_value.expose()).map_err(|_value| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            SecretsError::Encoding {
                context: "credential header",
                reason: "not a header value".to_owned(),
            },
        )
    })?;
    value.set_sensitive(true);
    outgoing.insert(name, value);
    let upstream = shared
        .client
        .request(parts.method, &url)
        .headers(outgoing)
        .body(body)
        .send()
        .await
        .map_err(|error| {
            (
                StatusCode::BAD_GATEWAY,
                SecretsError::Encoding {
                    context: "upstream call",
                    reason: redact_text(&error.to_string(), &credential),
                },
            )
        })?;
    let status = upstream.status();
    let mut headers = HeaderMap::new();
    for (name, value) in upstream.headers() {
        if contains(value.as_bytes(), credential.expose()) {
            continue;
        }
        headers.insert(name.clone(), value.clone());
    }
    headers.remove("content-length");
    headers.remove("transfer-encoding");
    let answer: Bytes = upstream.bytes().await.map_err(|error| {
        (
            StatusCode::BAD_GATEWAY,
            SecretsError::Encoding {
                context: "upstream answer",
                reason: redact_text(&error.to_string(), &credential),
            },
        )
    })?;
    let answer = redact(&answer, credential.expose());
    let mut response = Response::new(Body::from(answer));
    *response.status_mut() = status;
    *response.headers_mut() = headers;
    Ok(response)
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    !needle.is_empty()
        && haystack
            .windows(needle.len())
            .any(|window| window == needle)
}

fn redact(haystack: &[u8], needle: &[u8]) -> Vec<u8> {
    if needle.is_empty() {
        return haystack.to_vec();
    }
    let mut out = Vec::with_capacity(haystack.len());
    let mut at = 0;
    while at < haystack.len() {
        if haystack[at..].starts_with(needle) {
            out.extend_from_slice(REDACTED);
            at += needle.len();
        } else {
            out.push(haystack[at]);
            at += 1;
        }
    }
    out
}

fn redact_text(text: &str, credential: &Secret) -> String {
    String::from_utf8_lossy(&redact(text.as_bytes(), credential.expose())).into_owned()
}
