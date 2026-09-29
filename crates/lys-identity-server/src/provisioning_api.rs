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
use lys_home::harness::launch_fields::DeclaredHarness;
use lys_identity::{AgentId, IdentityId, OperationId};
use serde::{Deserialize, Serialize};

use crate::agent_sight::seen_agent;
use crate::error::ServerError;
use crate::launch_permissions::{Permissions, checked};
use crate::provisioning_store::{
    McpServer, Profile, ProvisioningStore, Review, SessionSettings, Settings, SkillPin, Version,
};
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
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[schema(as = ProvisioningVersionView)]
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
    /// Each named skill's text as this version was recorded with it.
    pub skill_pins: Vec<SkillPin>,
    /// The harness build it is started with, null when none is declared.
    #[schema(value_type = Option<Object>)]
    pub harness: Option<DeclaredHarness>,
    /// The permissions its settings file carries, null when none are set.
    pub permissions: Option<Permissions>,
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
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
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
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
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
    /// What this act was recorded as, null on a read. A change sent again
    /// answers the version it was first recorded as, whatever was set
    /// after it, while `profile` is always the latest. A review answers the
    /// version reviewed and the operation of the review that is kept, which
    /// is an earlier one when the version was reviewed already.
    pub recorded: Option<Recorded>,
}

/// The version a change was recorded as.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[schema(as = ProvisioningRecorded)]
pub struct Recorded {
    /// The operation id the change was set with.
    pub operation: String,
    /// The version it was recorded as.
    pub version: u32,
}

/// A profile to set. Every member is required.
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = ProvisioningSetBody)]
pub(crate) struct SetBody {
    operation: String,
    from_version: u32,
    model_access: Vec<String>,
    tools: Vec<String>,
    skills: Vec<String>,
    mcp_servers: Vec<crate::mcp_record::McpServerBody>,
    instructions: String,
    note: String,
    #[serde(default)]
    session: Option<SessionSettings>,
    #[serde(default)]
    #[schema(value_type = Option<Object>)]
    harness: Option<DeclaredHarness>,
    #[serde(default)]
    permissions: Option<Permissions>,
}

/// The provisioning routes.
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .merge(crate::skills_api::routes())
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

fn settings(body: SetBody) -> Result<Settings, ServerError> {
    let declared = body.harness.map(harness).transpose()?;
    let permissions = body
        .permissions
        .map(|given| {
            let contract = declared
                .as_ref()
                .ok_or_else(|| malformed("permissions require harness.description.permissions"))?;
            checked(given, &contract.description.permissions)
        })
        .transpose()?;
    let settings = Settings {
        model_access: names("model_access", &body.model_access)?,
        tools: names("tools", &body.tools)?,
        skills: names("skills", &body.skills)?,
        mcp_servers: crate::mcp_record::servers(body.mcp_servers)?,
        instructions: text("instructions", &body.instructions, INSTRUCTIONS_MAX)?,
        note: text("note", &body.note, NOTE_MAX)?,
        session: body.session.clone().map(session).transpose()?,
        harness: declared,
        skill_pins: Vec::new(),
        permissions,
    };
    if let Some(declared) = &settings.harness {
        crate::launch_fields::models(declared, &settings.model_access)?;
    }
    Ok(settings)
}

/// The declared build, refused when its program is not an absolute path or
/// it names no package to verify the program against.
fn harness(declared: DeclaredHarness) -> Result<DeclaredHarness, ServerError> {
    if !declared.program.starts_with('/') || declared.program.contains('\0') {
        return Err(malformed("harness.program is not an absolute path"));
    }
    if declared.name.trim().is_empty() {
        return Err(malformed("harness.name is empty"));
    }
    if declared.description.rendering_contract.trim().is_empty() {
        return Err(malformed("harness.description.rendering_contract is empty"));
    }
    if declared.package.trim().is_empty() {
        return Err(malformed("harness.package names no package"));
    }
    Ok(declared)
}

/// The session settings, each refused by name when a runner could not use it.
fn session(settings: SessionSettings) -> Result<SessionSettings, ServerError> {
    if settings
        .compact
        .as_deref()
        .is_some_and(|line| line.trim().is_empty())
    {
        return Err(malformed(
            "session.compact is empty: name the command or leave it out",
        ));
    }
    if let Some(accounts) = &settings.accounts {
        lys_runner::rotation::RotationState::new(accounts.clone())
            .map_err(|refused| malformed(format!("session.accounts: {refused}")))?;
    }
    Ok(settings)
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

/// A profile or skill mutation, refused while the previous build may be
/// restored. Check after taking the store lock, immediately before its write.
pub(crate) fn write_provisioning<T>(
    state: &AppState,
    act: impl FnOnce(&mut ProvisioningStore) -> Result<T, ServerError>,
) -> Result<T, ServerError> {
    with_provisioning(state, |store| {
        let pending = crate::operator::upgrade_pending(state).map_err(|error| {
            ServerError::ProvisioningUnavailable {
                reason: format!("the provisioning writer cannot read the upgrade intent: {error}"),
            }
        })?;
        if pending {
            return Err(ServerError::ProvisioningUnavailable {
                reason:
                    "upgrade_pending: provisioning cannot change while the upgrade is reversible"
                        .to_owned(),
            });
        }
        act(store)
    })
}

fn view(agent: &str, profile: Option<&Profile>, recorded: Option<Recorded>) -> ProvisioningView {
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
            skill_pins: version.settings.skill_pins.clone(),
            harness: version.settings.harness.clone(),
            permissions: version.settings.permissions.clone(),
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
        recorded,
    }
}

async fn read(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<ProvisioningView>, ServerError> {
    let agent = seen_agent(&state, &headers, &id)?.agent.to_string();
    with_provisioning(&state, |store| {
        Ok(Json(view(&agent, store.profile(&agent), None)))
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
        let from_version = body.from_version;
        let version = Version {
            number: 0,
            operation: OperationId::from_str(&body.operation)?.to_string(),
            settings: settings(body)?,
            set_by: own_person(directory, &actor)?.to_string(),
            set_at: now(),
            reviewed: None,
        };
        let agent = agent.to_string();
        write_provisioning(&state, |store| {
            let operation = version.operation.clone();
            let mut version = version;
            version.settings.skill_pins = match store.pins_for(&version.operation) {
                Some(recorded) => recorded,
                None => store.pins(&version.settings.skills)?,
            };
            let version = store.set(&agent, from_version, version)?;
            let recorded = Recorded { operation, version };
            Ok(Json(view(&agent, store.profile(&agent), Some(recorded))))
        })
    })
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReviewBody {
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
        write_provisioning(&state, |store| {
            store.review(&agent, number, review)?;
            let recorded = store
                .profile(&agent)
                .and_then(|profile| profile.versions.iter().find(|kept| kept.number == number))
                .and_then(|kept| kept.reviewed.as_ref())
                .map(|kept| Recorded {
                    operation: kept.operation.clone(),
                    version: number,
                });
            Ok(Json(view(&agent, store.profile(&agent), recorded)))
        })
    })
}
