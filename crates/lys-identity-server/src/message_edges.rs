//! Cambium alone judges message visibility; this bridge additionally requires explicit Lys identity bindings.

use std::{
    collections::{BTreeMap, BTreeSet},
    str::FromStr,
    sync::Arc,
};

use axum::{
    Json, Router,
    extract::{Query, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use lys_identity::{AgentId, IdentityId, PersonId};
use serde::{Deserialize, Serialize};

use crate::{
    config::Config,
    error::ServerError,
    routes::{AppState, signed_in, with_directory},
};

/// Administrator-declared identity bridge; no secret or service credential is stored here.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    /// Cambium's trusted absolute base URL; HTTPS, or HTTP on loopback.
    pub url: String,
    /// Explicit bindings, never guessed from matching display names.
    pub bindings: Vec<Binding>,
}

/// One Cambium registry identity and the Lys directory identity it represents.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    /// Cambium participant registry id.
    pub participant: String,
    /// Enduring Lys person or agent id.
    pub identity: String,
}

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

fn parse_identity(value: &str) -> Result<IdentityId, lys_identity::IdentityError> {
    if value.starts_with("person-") {
        PersonId::from_str(value).map(IdentityId::Person)
    } else {
        AgentId::from_str(value).map(IdentityId::Agent)
    }
}

impl Settings {
    /// Validate the trusted endpoint and require a one-to-one identity mapping.
    ///
    /// # Errors
    /// Refuses an invalid endpoint, malformed identity or ambiguous binding.
    pub fn validate(&self) -> Result<(), ServerError> {
        self.bridge().map(|_| ())
    }

    fn bridge(&self) -> Result<Bridge, ServerError> {
        let invalid = |reason: String| ServerError::ConfigInvalid {
            reason: format!("cambium_messages: {reason}"),
        };
        let mut url = reqwest::Url::parse(&self.url).map_err(|error| invalid(error.to_string()))?;
        let loopback = url.host_str().is_some_and(|host| {
            host == "localhost"
                || host
                    .trim_matches(['[', ']'])
                    .parse::<std::net::IpAddr>()
                    .is_ok_and(|ip| ip.is_loopback())
        });
        if (url.scheme() != "https" && !(url.scheme() == "http" && loopback))
            || !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
        {
            return Err(invalid(
                "use HTTPS or loopback HTTP, without credentials, query or fragment".to_owned(),
            ));
        }
        if !url.path().ends_with('/') {
            url.set_path(&format!("{}/", url.path()));
        }
        let mut bindings = BTreeMap::new();
        let mut identities = BTreeSet::new();
        for binding in &self.bindings {
            parse_identity(&binding.identity)
                .map_err(|error| invalid(format!("identity {}: {error}", binding.identity)))?;
            if binding.participant.is_empty()
                || !binding
                    .participant
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
            {
                return Err(invalid(
                    "participant must be a Cambium registry id".to_owned(),
                ));
            }
            if bindings
                .insert(binding.participant.clone(), binding.identity.clone())
                .is_some()
                || !identities.insert(binding.identity.clone())
            {
                return Err(invalid(format!(
                    "ambiguous binding for {}",
                    binding.participant
                )));
            }
        }
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|error| invalid(error.to_string()))?;
        Ok(Bridge {
            url,
            bindings,
            client,
        })
    }
}

/// Build this optional integration without making any network request at startup.
///
/// # Errors
/// Refuses invalid integration configuration.
pub fn routes(config: &Config) -> Result<Router<Arc<AppState>>, ServerError> {
    let bridge = Arc::new(
        config
            .cambium_messages
            .as_ref()
            .map(Settings::bridge)
            .transpose()?,
    );
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
        let text = value
            .to_str()
            .map_err(|_| unavailable("Cambium cookie header is unreadable"))?;
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
            .filter_map(|(id, record)| {
                (administrator || *id == caller || record.responsible() == Some(person))
                    .then(|| id.to_string())
            })
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
        let refusal = response.json::<UpstreamRefusal>().await.map_err(|error| {
            unavailable(format!(
                "Cambium answered HTTP {status} with an unreadable refusal: {error}"
            ))
        })?;
        return Err(EdgeError::Refused {
            status,
            reason: format!("Cambium {}: {}", refusal.kind, refusal.reason),
        });
    }
    let page: UpstreamPage = response
        .json()
        .await
        .map_err(|error| unavailable(format!("Cambium message page is unreadable: {error}")))?;
    map_page(page, bridge, &caller, &visible).map(Json)
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
