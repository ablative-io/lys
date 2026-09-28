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
use lys_secrets::{
    Admitted, Broker, HandleToken, Presentation, Secret, SecretsError, from_hex, request_digest,
};

use crate::files::{Layout, Route};
use crate::spice::Grants;

pub(crate) const MAX_BODY: usize = 16 * 1024 * 1024;
const REDACTED: &[u8] = b"[redacted]";

pub(crate) struct Shared {
    pub(crate) broker: Mutex<Broker<Grants>>,
    pub(crate) layout: Layout,
    pub(crate) client: reqwest::Client,
    pub(crate) window: Mutex<lys_secrets::ServiceWindow>,
}

pub fn serve(mut broker: Broker<Grants>, layout: Layout, listen: &str) -> Result<(), SecretsError> {
    println!("lys-secrets audit log {}", broker.start());
    if let Some(failure) = broker.snapshot_failure() {
        println!("lys-secrets {failure}");
    }
    broker.report_snapshots_to(Box::new(|failure| println!("lys-secrets {failure}")));
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
        window: Mutex::new(lys_secrets::ServiceWindow::new()),
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
            .route("/_lys/handles", axum::routing::get(crate::view::handles))
            .route("/_lys/next-account", axum::routing::post(next_account))
            .route("/_lys/scope", axum::routing::post(crate::manage::scope))
            .route(
                "/_lys/recipients",
                axum::routing::post(crate::manage::recipients),
            )
            .route(
                "/_lys/settings",
                axum::routing::get(crate::manage::settings),
            )
            .route(
                "/_lys/revocation",
                axum::routing::get(crate::manage::revocation),
            )
            .route(
                "/_lys/drop",
                axum::routing::post(crate::manage::drop_handle),
            )
            .route(
                "/_lys/leases/{lease_id}",
                axum::routing::get(crate::manage::lease),
            )
            .route(
                "/_lys/leases/{lease_id}/revoke",
                axum::routing::post(crate::manage::revoke),
            )
            .route(
                "/_lys/leases/{lease_id}/relinquish",
                axum::routing::post(crate::manage::relinquish),
            )
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

/// The handle and the presentation a request carries, the presentation
/// bound to the digest of this very request. A request with no
/// `lys-presentation` header carries an unsigned presentation, which the
/// broker refuses as `PresentationUnsigned` once it has found the handle.
pub(crate) fn signed(
    parts: &axum::http::request::Parts,
    body: &[u8],
) -> Result<(HandleToken, Presentation), (StatusCode, SecretsError)> {
    let bad = |error: SecretsError| (StatusCode::BAD_REQUEST, error);
    let request = request_digest(
        parts.method.as_str(),
        parts
            .uri
            .path_and_query()
            .map_or("/", |whole| whole.as_str()),
        body,
    )
    .map_err(bad)?;
    let malformed = || SecretsError::PresentationInvalid {
        handle: String::new(),
    };
    let token = header(&parts.headers, "lys-handle")
        .and_then(from_hex)
        .ok_or_else(|| bad(SecretsError::HandleUnknown))?;
    let presentation = Presentation::from_wire(
        header(&parts.headers, "lys-handle-id").ok_or_else(|| bad(malformed()))?,
        header(&parts.headers, "lys-operation").ok_or_else(|| bad(malformed()))?,
        header(&parts.headers, "lys-signed-at").ok_or_else(|| bad(malformed()))?,
        header(&parts.headers, "lys-presentation"),
        request,
    )
    .map_err(bad)?;
    Ok((HandleToken::from_bytes(&token), presentation))
}

/// The holder of a handle asks for the secret's next account, for example
/// at the current one's usage limit. The answer names the account, never
/// its value.
async fn next_account(State(shared): State<Arc<Shared>>, request: Request) -> Response {
    let (parts, body) = request.into_parts();
    let asked = async {
        let body = axum::body::to_bytes(body, MAX_BODY)
            .await
            .map_err(|error| {
                (
                    StatusCode::BAD_REQUEST,
                    SecretsError::Encoding {
                        context: "request body",
                        reason: error.to_string(),
                    },
                )
            })?;
        let (token, presentation) = signed(&parts, &body)?;
        let mut broker = shared.broker.lock().unwrap_or_else(PoisonError::into_inner);
        broker
            .next_account_for(&token, &presentation)
            .map_err(|error| (StatusCode::FORBIDDEN, error))
    };
    match asked.await {
        Ok(account) => (StatusCode::OK, format!("now {account}\n")).into_response(),
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
        Some(text) => text.parse::<u64>().map_err(|_number| {
            bad(SecretsError::Encoding {
                context: "lys-reserve",
                reason: "not a whole number".to_owned(),
            })
        })?,
    };
    let admitted = {
        let mut broker = shared.broker.lock().unwrap_or_else(PoisonError::into_inner);
        broker.admit_use_for(&token, &presentation, secret, reserve)
    }
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
    let ticket = {
        let mut broker = shared.broker.lock().unwrap_or_else(PoisonError::into_inner);
        broker.at_forward_boundary(ticket)
    }
    .map_err(|error| (StatusCode::FORBIDDEN, error))?;
    let rest = rest.to_owned();
    let called = match ticket.oauth() {
        Err(error) => Err((StatusCode::INTERNAL_SERVER_ERROR, error)),
        Ok(None) => {
            let credential = ticket.credential();
            call_upstream(
                shared,
                &route,
                parts,
                &rest,
                body,
                credential,
                &[credential],
            )
            .await
        }
        Ok(Some(grant)) => match crate::oauth_proxy::live(shared, &ticket, grant).await {
            Err(error) => Err(error),
            Ok((grant, retired)) => {
                let mut tokens = grant.tokens();
                tokens.extend(retired.iter());
                call_upstream(
                    shared,
                    &route,
                    parts,
                    &rest,
                    body,
                    grant.access_token(),
                    &tokens,
                )
                .await
            }
        },
    };
    let reserved = ticket.reserved().unwrap_or(0);
    {
        let mut broker = shared.broker.lock().unwrap_or_else(PoisonError::into_inner);
        match &called {
            Ok((_, Some(reported))) => broker.settle(ticket, *reported),
            Ok((_, None)) => broker.settle_unmetered(ticket),
            Err(_) => broker.settle_failed(ticket, reserved),
        }
    }
    .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error))?;
    called.map(|(response, _reported)| response)
}

/// Sends the request upstream with the credential in the route's header,
/// and answers the redacted response and the spend the upstream reported in
/// the route's spend header, when it names one.
async fn call_upstream(
    shared: &Shared,
    route: &Route,
    parts: axum::http::request::Parts,
    rest: &str,
    body: Bytes,
    credential: &Secret,
    hidden: &[&Secret],
) -> Result<(Response, Option<u64>), (StatusCode, SecretsError)> {
    let mut header_value = route.prefix.as_bytes().to_vec();
    header_value.extend_from_slice(credential.expose());
    let header_value = Secret::new(header_value);
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
                    reason: redact_text(&error.to_string(), hidden),
                },
            )
        })?;
    let status = upstream.status();
    let reported = route
        .spend_header
        .as_deref()
        .and_then(|name| upstream.headers().get(name))
        .and_then(|value| value.to_str().ok())
        .and_then(|text| text.trim().parse::<u64>().ok());
    let mut headers = HeaderMap::new();
    for (name, value) in upstream.headers() {
        if hidden
            .iter()
            .any(|secret| contains(value.as_bytes(), secret.expose()))
        {
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
                reason: redact_text(&error.to_string(), hidden),
            },
        )
    })?;
    let answer = hidden.iter().fold(answer.to_vec(), |text, secret| {
        redact(&text, secret.expose())
    });
    let mut response = Response::new(Body::from(answer));
    *response.status_mut() = status;
    *response.headers_mut() = headers;
    Ok((response, reported))
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

fn redact_text(text: &str, hidden: &[&Secret]) -> String {
    let redacted = hidden
        .iter()
        .fold(text.as_bytes().to_vec(), |text, secret| {
            redact(&text, secret.expose())
        });
    String::from_utf8_lossy(&redacted).into_owned()
}
