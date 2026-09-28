//! The provisioning routes: what an agent is set up with, as it is recorded.
//! The models it may use, its tools, skills and MCP servers, and its
//! instructions.
//!
//! The administrator sets a profile, each change a new version over the one
//! it saw. The administrator, the person responsible for the agent and the
//! agent itself read it; to anyone else the agent's profile is not visible.
//!
//! The person responsible for the agent, or the administrator, reviews a
//! version under an operation id; an agent is started only from a reviewed
//! version, and the answer says when the reviewer is the person who set it.
//!
//! No runtime applies a profile yet. The answer says so in `enforced`, and
//! never shows a profile as applied.

use std::str::FromStr;
use std::sync::{Arc, PoisonError};

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::routing::{get, post};
use axum::{Json, Router};
use lys_identity::{AgentId, IdentityId, OperationId};
use reqwest::Url;
use serde::{Deserialize, Serialize};

use crate::error::ServerError;
use crate::grants::caller;
use crate::provisioning_store::{McpServer, Profile, ProvisioningStore, Review, Settings, Version};
use crate::read_api::own_person;
use crate::routes::{AppState, signed_in, with_directory};
use crate::session::now;

/// The most characters a name carries.
const NAME_MAX: usize = 100;
/// The most names one list carries.
const LIST_MAX: usize = 64;
/// The most characters the instructions carry.
const INSTRUCTIONS_MAX: usize = 20_000;
/// The most characters a note carries.
const NOTE_MAX: usize = 500;

/// One version of a profile, in full.
#[derive(Debug, Clone, Serialize)]
pub struct VersionView {
    /// Its number, from 1.
    pub version: u32,
    /// The operation id it was set with.
    pub operation: String,
    /// The models the agent may use.
    pub model_access: Vec<String>,
    /// The tools it is given.
    pub tools: Vec<String>,
    /// The skills it is given.
    pub skills: Vec<String>,
    /// The MCP servers it is given.
    pub mcp_servers: Vec<McpServer>,
    /// Its instructions.
    pub instructions: String,
    /// Why this version was set.
    pub note: String,
    /// The person who set it.
    pub set_by: String,
    /// When it was set, in seconds since the Unix epoch.
    pub set_at: u64,
    /// The person who reviewed it, null until it is reviewed; an agent is
    /// started only from a reviewed version.
    pub reviewed_by: Option<String>,
    /// When it was reviewed, null until it is.
    pub reviewed_at: Option<u64>,
    /// Whether the person who reviewed it is the person who set it.
    pub self_reviewed: bool,
}

/// One version of a profile, as the history lists it.
#[derive(Debug, Clone, Serialize)]
pub struct VersionLine {
    /// Its number.
    pub version: u32,
    /// The person who set it.
    pub set_by: String,
    /// When it was set, in seconds since the Unix epoch.
    pub set_at: u64,
    /// Why it was set.
    pub note: String,
}

/// The answer of the provisioning routes.
#[derive(Debug, Clone, Serialize)]
pub struct ProvisioningView {
    /// The agent.
    pub agent: String,
    /// The latest version, null while nothing was set.
    pub profile: Option<VersionView>,
    /// Every version, in order.
    pub versions: Vec<VersionLine>,
    /// Whether a runtime applies the profile. While none does, the profile
    /// is recorded and not applied.
    pub enforced: bool,
}

/// A profile to set. Every member is required.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SetBody {
    operation: String,
    from_version: u32,
    model_access: Vec<String>,
    tools: Vec<String>,
    skills: Vec<String>,
    mcp_servers: Vec<McpServer>,
    instructions: String,
    note: String,
}

/// The provisioning routes.
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/agents/{id}/provisioning", get(read).post(set))
        .route("/agents/{id}/provisioning/{version}/review", post(review))
}

fn malformed(reason: impl Into<String>) -> ServerError {
    ServerError::RequestMalformed {
        reason: reason.into(),
    }
}

fn text(name: &str, text: &str, most: usize) -> Result<String, ServerError> {
    let text = text.trim();
    if text.chars().count() > most {
        return Err(malformed(format!(
            "{name} is longer than {most} characters"
        )));
    }
    Ok(text.to_owned())
}

fn named(name: &str, given: &str) -> Result<String, ServerError> {
    let given = text(name, given, NAME_MAX)?;
    if given.is_empty() {
        return Err(malformed(format!("a name in {name} is empty")));
    }
    Ok(given)
}

fn names(name: &str, given: &[String]) -> Result<Vec<String>, ServerError> {
    if given.len() > LIST_MAX {
        return Err(malformed(format!(
            "{name} holds more than {LIST_MAX} names"
        )));
    }
    let mut kept = Vec::new();
    for one in given {
        let one = named(name, one)?;
        if !kept.contains(&one) {
            kept.push(one);
        }
    }
    Ok(kept)
}

fn servers(given: &[McpServer]) -> Result<Vec<McpServer>, ServerError> {
    if given.len() > LIST_MAX {
        return Err(malformed(format!(
            "mcp_servers holds more than {LIST_MAX} servers"
        )));
    }
    let mut kept: Vec<McpServer> = Vec::new();
    for server in given {
        let name = named("mcp_servers", &server.name)?;
        let url = Url::parse(server.url.trim()).map_err(|error| {
            malformed(format!(
                "the URL of MCP server `{name}` does not read: {error}"
            ))
        })?;
        if !matches!(url.scheme(), "http" | "https") {
            return Err(malformed(format!(
                "the URL of MCP server `{name}` is not http or https"
            )));
        }
        if kept.iter().any(|other| other.name == name) {
            return Err(malformed(format!("MCP server `{name}` is named twice")));
        }
        kept.push(McpServer {
            name,
            url: url.to_string(),
        });
    }
    Ok(kept)
}

fn settings(body: &SetBody) -> Result<Settings, ServerError> {
    Ok(Settings {
        model_access: names("model_access", &body.model_access)?,
        tools: names("tools", &body.tools)?,
        skills: names("skills", &body.skills)?,
        mcp_servers: servers(&body.mcp_servers)?,
        instructions: text("instructions", &body.instructions, INSTRUCTIONS_MAX)?,
        note: text("note", &body.note, NOTE_MAX)?,
    })
}

pub(crate) fn with_provisioning<T>(
    state: &AppState,
    act: impl FnOnce(&mut ProvisioningStore) -> Result<T, ServerError>,
) -> Result<T, ServerError> {
    let store =
        state
            .provisioning
            .as_ref()
            .ok_or_else(|| ServerError::ProvisioningUnavailable {
                reason: "the configuration names no provisioning_file".to_owned(),
            })?;
    let mut store = store.lock().unwrap_or_else(PoisonError::into_inner);
    store.settle()?;
    act(&mut store)
}

fn view(agent: &str, profile: Option<&Profile>) -> ProvisioningView {
    let versions = profile.map_or(&[][..], |profile| profile.versions.as_slice());
    ProvisioningView {
        agent: agent.to_owned(),
        profile: versions.last().map(|version| VersionView {
            version: version.number,
            operation: version.operation.clone(),
            model_access: version.settings.model_access.clone(),
            tools: version.settings.tools.clone(),
            skills: version.settings.skills.clone(),
            mcp_servers: version.settings.mcp_servers.clone(),
            instructions: version.settings.instructions.clone(),
            note: version.settings.note.clone(),
            set_by: version.set_by.clone(),
            set_at: version.set_at,
            reviewed_by: version.reviewed.as_ref().map(|review| review.by.clone()),
            reviewed_at: version.reviewed.as_ref().map(|review| review.at),
            self_reviewed: version
                .reviewed
                .as_ref()
                .is_some_and(|review| review.by == version.set_by),
        }),
        versions: versions
            .iter()
            .map(|version| VersionLine {
                version: version.number,
                set_by: version.set_by.clone(),
                set_at: version.set_at,
                note: version.settings.note.clone(),
            })
            .collect(),
        enforced: false,
    }
}

async fn read(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<ProvisioningView>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    let agent = AgentId::from_str(&id).map_err(|_unread| ServerError::AgentNotVisible)?;
    with_directory(&state, |directory| {
        let directory = directory.projection()?;
        let asker = caller(&state, &headers, directory)?;
        let record = directory
            .record(IdentityId::Agent(agent))
            .ok_or(ServerError::AgentNotVisible)?;
        let sees = state.admission.administrator(&actor).is_ok()
            || match asker {
                IdentityId::Agent(own) => own == agent,
                IdentityId::Person(person) => record.responsible() == Some(person),
            };
        if !sees {
            return Err(ServerError::AgentNotVisible);
        }
        let agent = agent.to_string();
        with_provisioning(&state, |store| {
            Ok(Json(view(&agent, store.profile(&agent))))
        })
    })
}

async fn set(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    body: Result<Json<SetBody>, JsonRejection>,
) -> Result<Json<ProvisioningView>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    state.admission.administrator(&actor)?;
    let Json(body) = body.map_err(|refused| malformed(refused.body_text()))?;
    let agent = AgentId::from_str(&id).map_err(|_unread| ServerError::AgentNotVisible)?;
    with_directory(&state, |directory| {
        let directory = directory.projection()?;
        if directory.record(IdentityId::Agent(agent)).is_none() {
            return Err(ServerError::AgentNotVisible);
        }
        let version = Version {
            number: 0,
            operation: OperationId::from_str(&body.operation)?.to_string(),
            settings: settings(&body)?,
            set_by: own_person(directory, &actor)?.to_string(),
            set_at: now(),
            reviewed: None,
        };
        let agent = agent.to_string();
        with_provisioning(&state, |store| {
            store.set(&agent, body.from_version, version)?;
            Ok(Json(view(&agent, store.profile(&agent))))
        })
    })
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReviewBody {
    operation: String,
}

async fn review(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((id, number)): Path<(String, u32)>,
    body: Result<Json<ReviewBody>, JsonRejection>,
) -> Result<Json<ProvisioningView>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    let Json(body) = body.map_err(|refused| malformed(refused.body_text()))?;
    let agent = AgentId::from_str(&id).map_err(|_unread| ServerError::AgentNotVisible)?;
    with_directory(&state, |directory| {
        let directory = directory.projection()?;
        let record = directory
            .record(IdentityId::Agent(agent))
            .ok_or(ServerError::AgentNotVisible)?;
        let person = own_person(directory, &actor)?;
        let answers =
            state.admission.administrator(&actor).is_ok() || record.responsible() == Some(person);
        if !answers {
            return Err(ServerError::NotAdmitted {
                reason: "only the person responsible for the agent or the administrator reviews its profile",
            });
        }
        let review = Review {
            operation: OperationId::from_str(&body.operation)?.to_string(),
            by: person.to_string(),
            at: now(),
        };
        let agent = agent.to_string();
        with_provisioning(&state, |store| {
            store.review(&agent, number, review)?;
            Ok(Json(view(&agent, store.profile(&agent))))
        })
    })
}
