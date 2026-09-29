//! Cambium alone judges message visibility; this bridge additionally requires explicit Lys identity bindings.

use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

use axum::{
    Json, Router,
    extract::{Query, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use lys_identity::IdentityId;
use serde::{Deserialize, Serialize};

use crate::{
    config::Config,
    error::ServerError,
    routes::{AppState, signed_in, with_directory},
};

pub use lys_identity::cambium_messages::{Binding, Settings};

/// Opaque page request passed to Cambium, whose configured page bound applies.
#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct EdgeQuery {
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Cambium stream id, absent when listing places.
    pub stream: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Thread root id, absent when listing root posts.
    pub root: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Opaque cursor within the requested scope.
    pub cursor: Option<String>,
}

/// Addressed message metadata; no message body or credential crosses into the graph.
#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub struct MessageEdge {
    /// Recorded Cambium post id.
    pub message: String,
    /// Cambium stream containing this post.
    pub stream: String,
    /// Explicitly mapped Lys author identity.
    pub source: String,
    /// Explicitly mapped, caller-visible addressed identities.
    pub recipients: Vec<String>,
    /// Direct address or explicit mention, never proof of consumption.
    pub addressing: String,
    /// Cambium creation timestamp in seconds.
    pub at: u64,
}

#[derive(Deserialize)]
struct UpstreamRefusal {
    kind: String,
    reason: String,
}

#[derive(Deserialize)]
struct UpstreamPage {
    caller: String,
    places: Vec<String>,
    messages: Vec<MessageEdge>,
    roots: Vec<String>,
    next: Option<String>,
}

/// One Cambium-filtered page, then restricted to identities visible in Lys.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct EdgePage {
    /// Caller-visible places to page.
    pub places: Vec<String>,
    /// Addressing evidence after both services apply visibility.
    pub messages: Vec<MessageEdge>,
    /// Thread roots whose replies require separate paging.
    pub roots: Vec<String>,
    /// Continuation for this scope, absent at its end.
    pub next: Option<String>,
    /// Caller-visible Cambium identities that have no explicit Lys binding.
    pub unmapped: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum EdgeError {
    #[error(transparent)]
    Identity(ServerError),
    #[error("MessageEdgesUnavailable: {reason}")]
    Refused { status: StatusCode, reason: String },
}

impl From<ServerError> for EdgeError {
    fn from(error: ServerError) -> Self {
        Self::Identity(error)
    }
}
impl IntoResponse for EdgeError {
    fn into_response(self) -> Response {
        match self {
            Self::Identity(error) => error.into_response(),
            Self::Refused { status, reason } => (status, Json(serde_json::json!({ "refusal": "MessageEdgesUnavailable", "reason": reason, "fields": {} }))).into_response(),
        }
    }
}
fn unavailable(reason: impl Into<String>) -> EdgeError {
    EdgeError::Refused {
        status: StatusCode::SERVICE_UNAVAILABLE,
        reason: reason.into(),
    }
}

struct Bridge {
    url: reqwest::Url,
    bindings: BTreeMap<String, String>,
    client: reqwest::Client,
}

/// Validate the trusted endpoint and require a one-to-one identity mapping.
///
/// # Errors
/// Refuses an invalid endpoint, malformed identity or ambiguous binding.
pub fn validate(settings: &Settings) -> Result<(), ServerError> {
    bridge(settings).map(|_| ())
}

fn bridge(settings: &Settings) -> Result<Bridge, ServerError> {
    let invalid = |reason: String| ServerError::ConfigInvalid {
        reason: format!("cambium_messages: {reason}"),
    };
    let checked = settings.check().map_err(invalid)?;
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|error| invalid(error.to_string()))?;
    Ok(Bridge {
        url: checked.url,
        bindings: checked.bindings,
        client,
    })
}

/// Build this optional integration without making any network request at startup.
///
/// # Errors
/// Refuses invalid integration configuration.
pub fn routes(config: &Config) -> Result<Router<Arc<AppState>>, ServerError> {
    let bridge = Arc::new(config.cambium_messages.as_ref().map(bridge).transpose()?);
    Ok(Router::new().route(
        "/runtime/message-edges",
        get(
            move |State(state): State<Arc<AppState>>,
                  headers: HeaderMap,
                  Query(query): Query<EdgeQuery>| {
                let bridge = Arc::clone(&bridge);
                async move { read(&state, &headers, query, bridge.as_ref().as_ref()).await }
            },
        ),
    ))
}

fn cookie(headers: &HeaderMap) -> Result<String, EdgeError> {
    let mut values = Vec::new();
    for value in headers.get_all(header::COOKIE) {
        let text = value.to_str().map_err(|error| {
            unavailable(format!("Cambium cookie header is unreadable: {error}"))
        })?;
        for part in text.split(';') {
            if let Some((name, value)) = part.trim().split_once('=') {
                if name == "cambium_session" {
                    values.push(value);
                }
            }
        }
    }
    if values.len() != 1 || values[0].is_empty() {
        return Err(EdgeError::Refused { status: StatusCode::UNAUTHORIZED, reason: "Sign into Cambium on this host before reading message connections; exactly one Cambium session is required".to_owned() });
    }
    Ok(format!("cambium_session={}", values[0]))
}

async fn read(
    state: &AppState,
    headers: &HeaderMap,
    query: EdgeQuery,
    bridge: Option<&Bridge>,
) -> Result<Json<EdgePage>, EdgeError> {
    let actor = signed_in(state, headers)?;
    let (caller, visible) = with_directory(state, |store| {
        let directory = store.projection()?;
        let person = crate::read_api::own_person(directory, &actor)?;
        let caller = IdentityId::Person(person);
        let administrator = state.admission.administrator(&actor).is_ok();
        let visible: BTreeSet<String> = directory
            .records()
            .filter(|(id, record)| {
                administrator || **id == caller || record.responsible() == Some(person)
            })
            .map(|(id, _)| id.to_string())
            .collect();
        Ok((caller.to_string(), visible))
    })?;
    let bridge = bridge.ok_or_else(|| {
        unavailable("cambium_messages endpoint and explicit identity bindings are not configured")
    })?;
    let url = bridge
        .url
        .join("conversation/message-edges")
        .map_err(|error| unavailable(error.to_string()))?;
    let response = bridge
        .client
        .get(url)
        .query(&query)
        .header(header::COOKIE, cookie(headers)?)
        .send()
        .await
        .map_err(|error| unavailable(format!("Cambium message read failed: {error}")))?;
    if !response.status().is_success() {
        let status = response.status();
        let refusal = read_json::<UpstreamRefusal>(response)
            .await
            .map_err(|error| {
                unavailable(format!(
                    "Cambium answered HTTP {status} with an unreadable refusal: {error}"
                ))
            })?;
        return Err(EdgeError::Refused {
            status,
            reason: format!("Cambium {}: {}", refusal.kind, refusal.reason),
        });
    }
    let page: UpstreamPage = read_json(response)
        .await
        .map_err(|error| unavailable(format!("Cambium message page is unreadable: {error}")))?;
    map_page(page, bridge, &caller, &visible).map(Json)
}

/// The body of `response`, read whole, as `T`.
async fn read_json<T: serde::de::DeserializeOwned>(
    response: reqwest::Response,
) -> Result<T, String> {
    let bytes = response.bytes().await.map_err(|error| error.to_string())?;
    serde_json::from_slice(&bytes).map_err(|error| error.to_string())
}

fn map_page(
    page: UpstreamPage,
    bridge: &Bridge,
    caller: &str,
    visible: &BTreeSet<String>,
) -> Result<EdgePage, EdgeError> {
    if bridge.bindings.get(&page.caller).map(String::as_str) != Some(caller) {
        return Err(EdgeError::Refused {
            status: StatusCode::FORBIDDEN,
            reason: "The Cambium session is not bound to this signed-in Lys identity".to_owned(),
        });
    }
    let mut unmapped = BTreeSet::new();
    let mut messages = Vec::new();
    for mut message in page.messages {
        if !matches!(message.addressing.as_str(), "direct" | "mentioned") {
            return Err(unavailable(format!(
                "Cambium message {} has an unknown addressing kind",
                message.message
            )));
        }
        let Some(source) = bridge.bindings.get(&message.source) else {
            unmapped.insert(message.source);
            continue;
        };
        if !visible.contains(source) {
            continue;
        }
        message.source.clone_from(source);
        message.recipients = message
            .recipients
            .into_iter()
            .filter_map(|recipient| match bridge.bindings.get(&recipient) {
                Some(identity) if visible.contains(identity) => Some(identity.clone()),
                Some(_) => None,
                None => {
                    unmapped.insert(recipient);
                    None
                }
            })
            .collect();
        if !message.recipients.is_empty() {
            messages.push(message);
        }
    }
    Ok(EdgePage {
        places: page.places,
        messages,
        roots: page.roots,
        next: page.next,
        unmapped: unmapped.into_iter().collect(),
    })
}

#[cfg(test)]
#[path = "message_edges_tests.rs"]
mod tests;

/// Describe the derived GET members as query parameters, never a GET request body.
pub(crate) fn document_query(document: &mut serde_json::Value) -> Result<(), ServerError> {
    let invalid = || ServerError::ConfigInvalid {
        reason: "message-edge query schema or operation missing from OpenAPI".to_owned(),
    };
    let operation_path = "/paths/~1runtime~1message-edges/get";
    let reference = document
        .pointer(&format!(
            "{operation_path}/requestBody/content/application~1json/schema/$ref"
        ))
        .and_then(serde_json::Value::as_str)
        .ok_or_else(invalid)?
        .to_owned();
    let pointer = reference.strip_prefix('#').ok_or_else(invalid)?;
    let properties = document
        .pointer(pointer)
        .and_then(|schema| schema.get("properties"))
        .and_then(serde_json::Value::as_object)
        .ok_or_else(invalid)?;
    let parameters: Vec<serde_json::Value> = properties
        .iter()
        .map(|(name, schema)| {
            serde_json::json!({
                "name": name, "in": "query", "required": false, "schema": schema,
            })
        })
        .collect();
    let operation = document
        .pointer_mut(operation_path)
        .and_then(serde_json::Value::as_object_mut)
        .ok_or_else(invalid)?;
    operation.remove("requestBody");
    operation.insert(
        "parameters".to_owned(),
        serde_json::Value::Array(parameters),
    );
    Ok(())
}
