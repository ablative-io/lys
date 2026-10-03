//! The door to the haematite stores: `GET /haem` names the stores the
//! configuration lists, and `POST /haem/{store}` carries one verb to that
//! store's `haem serve` over its Unix socket and answers what it answered.
//!
//! `haem serve` asks nobody who is calling, so this door is where the caller
//! is known. Only an administrator is admitted to either route: there is no
//! grant per store, collection or field yet, and until there is, nobody else
//! reaches a store through Lys.
//!
//! The service's wire is one frame per message, four bytes of length, most
//! significant first, then JSON. The door sets no frame size of its own: the
//! service states its bound in its greeting, and the verb's answer is held
//! to that. A connection greets with `hello` before any
//! other verb, so each request here is two exchanges on one connection: the
//! greeting, then the verb. The verb's `result` is answered as it came. The
//! service's own refusal is `HaemRefused`, carrying its code and its words:
//! a definite answer. A socket that cannot be reached, or that closes or
//! breaks its framing before the answer, is `HaemUnreachable`, never a
//! refusal: a change that was sent may have landed, and the caller settles
//! it by asking for its receipt.
//!
//! The caller is admitted before the body is read. A body that is not the
//! members the route asks for and no others is refused `RequestMalformed`.

use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::{Path as Named, State};
use axum::http::HeaderMap;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::error::ServerError;
use crate::error_haem::HaemError;
use crate::routes::{AppState, administrator, signed_in};

/// The door's routes.
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/haem", get(stores))
        .route("/haem/{store}", post(carry))
}

/// One store this door reaches.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct HaemStore {
    /// The name the configuration gives it.
    pub name: String,
}

/// The stores this door reaches.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct HaemStores {
    /// Every store, in name order.
    pub stores: Vec<HaemStore>,
}

/// One verb for a store's service.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct HaemVerb {
    /// The verb, as `haem serve` names it.
    pub method: String,
    /// The verb's parameters, when it takes any.
    #[serde(default)]
    #[schema(value_type = Option<Object>)]
    pub params: Option<Value>,
}

fn admitted(state: &AppState, headers: &HeaderMap) -> Result<(), ServerError> {
    let actor = signed_in(state, headers)?;
    administrator(state, &actor)
}

async fn stores(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<HaemStores>, ServerError> {
    admitted(&state, &headers)?;
    Ok(Json(HaemStores {
        stores: state
            .haem_stores
            .keys()
            .map(|name| HaemStore { name: name.clone() })
            .collect(),
    }))
}

async fn carry(
    State(state): State<Arc<AppState>>,
    Named(store): Named<String>,
    headers: HeaderMap,
    bytes: Bytes,
) -> Result<Json<Value>, ServerError> {
    admitted(&state, &headers)?;
    let Some(socket) = state.haem_stores.get(&store).cloned() else {
        return Err(HaemError::StoreUnknown { store }.into());
    };
    let verb: HaemVerb =
        serde_json::from_slice(&bytes).map_err(|error| ServerError::RequestMalformed {
            reason: format!("the body is not one haem verb: {error}"),
        })?;
    let named = store.clone();
    let answered = tokio::task::spawn_blocking(move || exchange(&socket, &verb))
        .await
        .map_err(|error| HaemError::Unreachable {
            store: named,
            reason: format!("the exchange did not finish: {error}"),
        })?;
    match answered {
        Ok(result) => Ok(Json(result)),
        Err(Exchange::Refused { code, message }) => {
            Err(HaemError::Refused { code, message }.into())
        }
        Err(Exchange::Broken(reason)) => Err(HaemError::Unreachable { store, reason }.into()),
    }
}

/// How an exchange with a store's service ends other than with a result.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Exchange {
    /// The service refused, by its own name and words.
    Refused { code: String, message: String },
    /// The wire failed before an answer came.
    Broken(String),
}

fn broken(what: &str, error: &impl std::fmt::Display) -> Exchange {
    Exchange::Broken(format!("{what}: {error}"))
}

fn write_frame(stream: &mut impl Write, value: &Value) -> Result<(), Exchange> {
    let body = serde_json::to_vec(value).map_err(|error| broken("the request", &error))?;
    let length = u32::try_from(body.len()).map_err(|error| broken("the request", &error))?;
    stream
        .write_all(&length.to_be_bytes())
        .and_then(|()| stream.write_all(&body))
        .map_err(|error| broken("writing to the socket", &error))
}

/// Read one frame. The door sets no size of its own: `stated` is the frame
/// bound the service gave in its greeting, and an answer that claims more
/// than the service said its frames hold is a broken wire. The greeting
/// itself is read before any bound is known, so its body is taken as it
/// arrives and never reserved from the length it claims.
fn read_frame(stream: &mut impl Read, stated: Option<u64>) -> Result<Value, Exchange> {
    let mut header = [0_u8; 4];
    stream
        .read_exact(&mut header)
        .map_err(|error| broken("reading the answer's length", &error))?;
    let length = u64::from(u32::from_be_bytes(header));
    if let Some(bound) = stated.filter(|bound| length > *bound) {
        return Err(Exchange::Broken(format!(
            "the answer claims {length} bytes and the service said its frames hold {bound}"
        )));
    }
    let mut body = Vec::new();
    let read = stream
        .take(length)
        .read_to_end(&mut body)
        .map_err(|error| broken("reading the answer", &error))?;
    if u64::try_from(read).ok() != Some(length) {
        return Err(Exchange::Broken(format!(
            "the answer claims {length} bytes and the connection closed after {read}"
        )));
    }
    serde_json::from_slice(&body).map_err(|error| broken("the answer is not JSON", &error))
}

/// One request and its answer: the `result`, or the service's refusal.
fn asked(
    stream: &mut (impl Read + Write),
    id: &str,
    operation: &Value,
    stated: Option<u64>,
) -> Result<Value, Exchange> {
    write_frame(stream, &json!({ "id": id, "operation": operation }))?;
    let mut answer = read_frame(stream, stated)?;
    if answer["id"] != id {
        return Err(Exchange::Broken(format!(
            "the answer is to {}, not to {id}",
            answer["id"]
        )));
    }
    if let Some(error) = answer.get("error") {
        return Err(Exchange::Refused {
            code: error["code"].as_str().unwrap_or("unnamed").to_owned(),
            message: error["message"].as_str().unwrap_or_default().to_owned(),
        });
    }
    match answer.get_mut("result") {
        Some(result) => Ok(result.take()),
        None => Err(Exchange::Broken(
            "the answer carries neither a result nor an error".to_owned(),
        )),
    }
}

/// Greet the service, then ask the verb, on one connection.
pub(crate) fn exchange(socket: &Path, verb: &HaemVerb) -> Result<Value, Exchange> {
    let mut stream =
        UnixStream::connect(socket).map_err(|error| broken("connecting to the socket", &error))?;
    let greeting = asked(&mut stream, "hello", &json!({ "method": "hello" }), None)?;
    let stated = greeting["max_frame"].as_u64();
    if verb.method == "hello" {
        return Ok(greeting);
    }
    let operation = match &verb.params {
        Some(params) => json!({ "method": verb.method, "params": params }),
        None => json!({ "method": verb.method }),
    };
    asked(&mut stream, "verb", &operation, stated)
}

/// The configured stores, each by an absolute socket path, or the entry at fault.
pub(crate) fn validate(stores: &std::collections::BTreeMap<String, PathBuf>) -> Result<(), String> {
    for (name, socket) in stores {
        if name.is_empty() || name.contains('/') {
            return Err(format!("haem_stores: `{name}` is not a store name"));
        }
        if !socket.is_absolute() {
            return Err(format!(
                "haem_stores: the socket of `{name}` must be an absolute path"
            ));
        }
    }
    Ok(())
}
