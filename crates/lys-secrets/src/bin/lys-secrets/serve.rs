//! The proxy. A holder sends its request here with its handle and a signed
//! presentation; the broker admits the use, the credential is written into
//! the one header its route names, and the request goes to the one upstream
//! the secret is bound to. The credential never reaches the holder: every
//! occurrence of it in the upstream's answer is replaced before it returns.
//!
//! No async worker waits on the broker. Its work, which takes its lock and
//! writes its audit log, runs on the blocking pool ([`on_broker`]). The
//! permission source, which may be a remote engine, is asked before the
//! broker is taken: a use's questions are read under the lock, the lock is
//! let go, the source answers them holding nothing, and the answers go to
//! the admission and, asked again, to the forward boundary. So a slow answer
//! holds up its own call and no other route.

use std::sync::{Arc, Mutex};

use axum::body::{Body, Bytes};
use axum::extract::{Request, State};
use axum::http::{HeaderMap, HeaderName, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use lys_secrets::{
    Admitted, Ask, Broker, Checked, HandleToken, PermissionCheck, Presentation, Secret,
    SecretsError, Settled, from_hex, request_digest,
};

use crate::files::{Layout, Route};
use crate::redact::{Redactor, redacted_answer};
use crate::spice::Grants;

/// Transport guard: the most bytes of a request body read from the socket
/// before its signature names the caller (the signature covers the body).
pub(crate) const MAX_BODY: usize = 16 * 1024 * 1024;

/// A refusal and the status it answers with.
type Refused = (StatusCode, SecretsError);

/// A permission source asked with no lock held.
pub(crate) type Permissions = Arc<dyn PermissionCheck + Send + Sync>;

pub(crate) struct Shared {
    pub(crate) broker: Mutex<Broker<Grants>>,
    pub(crate) layout: Layout,
    pub(crate) client: reqwest::Client,
    pub(crate) window: Mutex<lys_secrets::ServiceWindow>,
    /// The permission source the broker holds, asked before it is taken.
    pub(crate) permissions: Permissions,
}

pub fn serve(mut broker: Broker<Grants>, layout: Layout, listen: &str) -> Result<(), SecretsError> {
    println!("lys-secrets audit log {}", broker.start());
    if let Some(failure) = broker.snapshot_failure() {
        println!("lys-secrets {failure}");
    }
    broker.report_snapshots_to(Box::new(|failure| println!("lys-secrets {failure}")));
    if let Err(error) = layout.routes() {
        println!("lys-secrets routes: {error}");
    }
    if let Err(error) = layout.services() {
        println!("lys-secrets services: {error}");
    }
    let permissions: Permissions = Arc::new(broker.permissions().clone());
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
        permissions,
    });
    runtime.block_on(async move {
        let listener = tokio::net::TcpListener::bind(listen)
            .await
            .map_err(|source| SecretsError::Io {
                context: format!("listening on {listen}"),
                source,
            })?;
        println!("lys-secrets proxy listening on {listen}");
        let app = crate::router::routes(shared);
        axum::serve(listener, app)
            .await
            .map_err(|source| SecretsError::Io {
                context: "serving the proxy".to_owned(),
                source,
            })
    })
}

/// Runs `work` on the broker on the blocking pool, so no async worker waits
/// on the broker's lock or its disk.
pub(crate) async fn on_broker<T, F>(shared: &Arc<Shared>, work: F) -> Result<T, SecretsError>
where
    T: Send + 'static,
    F: FnOnce(&mut Broker<Grants>) -> T + Send + 'static,
{
    let shared = Arc::clone(shared);
    blocking(move || {
        let mut broker = shared
            .broker
            .lock()
            .map_err(|error| SecretsError::StatePoisoned {
                reason: error.to_string(),
            })?;
        Ok(work(&mut broker))
    })
    .await?
}

/// Runs `work` on the blocking pool.
async fn blocking<T, F>(work: F) -> Result<T, SecretsError>
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    tokio::task::spawn_blocking(work)
        .await
        .map_err(|error| SecretsError::Io {
            context: "running broker work off the async workers".to_owned(),
            source: std::io::Error::other(error.to_string()),
        })
}

/// The permission source's answers to `asks`, asked on the blocking pool
/// with no lock held.
pub(crate) async fn ask(shared: &Shared, asks: &[Ask]) -> Result<Checked, SecretsError> {
    let permissions = Arc::clone(&shared.permissions);
    let asks = asks.to_vec();
    let answered = blocking(move || Checked::answer(&asks, permissions.as_ref()));
    answered.await
}

/// The header the broker names its own refusal in, so a caller tells the
/// broker's refusal from an answer the upstream gave.
pub(crate) const REFUSAL_HEADER: &str = "lys-refusal";

fn refusal(status: StatusCode, error: &SecretsError) -> Response {
    (
        status,
        [(REFUSAL_HEADER, error.name())],
        format!("{error}\n"),
    )
        .into_response()
}

fn header<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers.get(name).and_then(|value| value.to_str().ok())
}

pub(crate) async fn proxy(State(shared): State<Arc<Shared>>, request: Request) -> Response {
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
    signed_for(parts, request)
}

pub(crate) fn signed_for(
    parts: &axum::http::request::Parts,
    request: [u8; 32],
) -> Result<(HandleToken, Presentation), (StatusCode, SecretsError)> {
    let bad = |error: SecretsError| (StatusCode::BAD_REQUEST, error);
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
pub(crate) async fn next_account(State(shared): State<Arc<Shared>>, request: Request) -> Response {
    let (parts, body) = request.into_parts();
    let asked = async {
        // Guarded: read before the caller is known, as the caller's signature covers it.
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
        on_broker(&shared, move |broker| {
            broker.next_account_for(&token, &presentation)
        })
        .await
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error))?
        .map_err(|error| (StatusCode::FORBIDDEN, error))
    };
    match asked.await {
        Ok(account) => (StatusCode::OK, format!("now {account}\n")).into_response(),
        Err((status, error)) => refusal(status, &error),
    }
}

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
        Some(text) => text.parse::<u64>().map_err(|_number| {
            bad(SecretsError::Encoding {
                context: "lys-reserve",
                reason: "not a whole number".to_owned(),
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
    let oauth = ticket.oauth();
    let (ticket, called) = match oauth {
        Err(error) => (Some(ticket), Err(internal(error))),
        Ok(None) => {
            let credential = ticket.credential();
            let called = call_upstream(
                shared,
                &route,
                parts,
                &rest,
                body,
                credential,
                &[credential],
            )
            .await;
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
                        &route,
                        parts,
                        &rest,
                        body,
                        grant.access_token(),
                        &tokens,
                    )
                    .await
                }
            };
            (ticket, called)
        }
    };
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

/// Sends the request upstream with the credential in the route's header,
/// and answers the redacted response and the spend the upstream reported in
/// the route's spend header, when it names one. The answer is read whole,
/// at any size, and redacted whole.
async fn call_upstream(
    shared: &Shared,
    route: &Route,
    parts: axum::http::request::Parts,
    rest: &str,
    body: Bytes,
    credential: &Secret,
    hidden: &[&Secret],
) -> Result<(Response, Option<u64>), Refused> {
    let redactor = Redactor::new(hidden);
    let mut header_value = route.prefix.as_bytes().to_vec();
    header_value.extend_from_slice(credential.expose());
    let header_value = Secret::new(header_value);
    let query = parts
        .uri
        .query()
        .map(|query| format!("?{query}"))
        .unwrap_or_default();
    let url = format!("{}/{rest}{query}", route.upstream.trim_end_matches('/'));
    let carried = route.header.to_ascii_lowercase();
    let mut outgoing = HeaderMap::new();
    for (name, value) in &parts.headers {
        let lowered = name.as_str();
        if lowered.starts_with("lys-") || lowered == "host" || lowered == carried {
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
                    reason: redactor.redact_text(&error.to_string()),
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
        if redactor.contains(value.as_bytes()) {
            continue;
        }
        headers.insert(name.clone(), value.clone());
    }
    headers.remove("content-length");
    headers.remove("transfer-encoding");
    let answer = redacted_answer(upstream, &redactor)
        .await
        .map_err(|error| (StatusCode::BAD_GATEWAY, error))?;
    let mut response = Response::new(Body::from(answer));
    *response.status_mut() = status;
    *response.headers_mut() = headers;
    Ok((response, reported))
}
