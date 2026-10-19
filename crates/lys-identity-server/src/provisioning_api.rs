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
//! A version without its own review is started when that person or the
//! administrator reviewed the same settings on another agent of theirs, and
//! the history says per version which review stands: its own, one carried by
//! content (naming it), or neither (AGENTS-017 R3).
//!
//! A profile is applied only when its complete native start was reported
//! running by its machine's runner. A newer version remains recorded until
//! that version runs.

use std::str::FromStr;
use std::sync::Arc;

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::routing::{get, post};
use axum::{Json, Router};
use lys_home::harness::launch_fields::{DeclaredHarness, InstructionsMode};
use lys_identity::{AgentId, IdentityId, OperationId};
use serde::{Deserialize, Serialize};

use crate::agent_sight::seen_agent;
use crate::error::ServerError;
use crate::launch_permissions::Permissions;
#[path = "provisioning_review.rs"]
mod reviews;
mod session;
mod validation;
use crate::provisioning_store::{
    McpServer, Profile, ProvisioningStore, Review, SessionSettings, SkillPin, Version,
};
use crate::read_api::own_person;
use crate::routes::{AppState, signed_in, with_directory};
use crate::session::now;
pub(crate) use reviews::{ReviewBody, SeatProgramReviewBody};
pub(crate) use validation::folder;
use validation::settings;

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
    /// The harness plugins its settings turn on, each `name@marketplace`.
    pub plugins: Vec<String>,
    /// Commands appended to each declared Claude hook event.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<Object>)]
    pub hooks: Option<
        std::collections::BTreeMap<String, Vec<lys_home::harness::launch_fields::HookCommand>>,
    >,
    /// The seat's own status command, preserved as declared.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_line: Option<String>,
    /// Extra native settings without the separately governed keys.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<Object>)]
    pub settings: Option<serde_json::Map<String, serde_json::Value>>,
    /// The existing conversation explicitly selected for resume.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resume: Option<String>,
    /// The context window explicitly declared for this seat, in tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_window: Option<u32>,
    /// The absolute harness config directory explicitly chosen by a person.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_dir: Option<String>,
    /// Its instructions.
    pub instructions: String,
    /// How instructions affect the program's prompt: keep, append or replace.
    #[schema(schema_with = instructions_mode_schema)]
    pub instructions_mode: InstructionsMode,
    /// How the profile drives its sessions, null when none was recorded.
    pub session: Option<SessionSettings>,
    /// Why this version was set.
    pub note: String,
    /// Each named skill's text as this version was recorded with it.
    pub skill_pins: Vec<SkillPin>,
    /// The harness build it is started with, null when none is declared.
    #[schema(value_type = Option<Object>)]
    pub harness: Option<DeclaredHarness>,
    /// The permissions its settings file carries, null when none are set.
    pub permissions: Option<Permissions>,
    /// The reviewed default machine, absent when none is declared.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runs_on: Option<String>,
    /// The reviewed writable folder, absent for an unconfined launch.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub writable: Option<String>,
    /// The folder this agent's runs start in when a launch names none;
    /// absent when every launch must name one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub working_folder: Option<String>,
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
    /// The SHA-256 of its settings serialised canonically, computed from the
    /// settings as kept; null when they cannot be written as JSON.
    pub settings_sha256: Option<String>,
    /// Which review stands for it: `itself`, `carried` (the same settings
    /// reviewed on another agent of the same person) or `neither`.
    pub reviewed: String,
    /// The carried review, when `reviewed` is `carried`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub carried_review: Option<crate::start_checks::CarriedReview>,
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

/// A profile to set, including optional seat declarations.
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
    /// Harness plugins its settings turn on, each `name@marketplace`.
    #[serde(default)]
    plugins: Vec<String>,
    /// Commands appended to each declared Claude hook event.
    #[serde(default)]
    #[schema(value_type = Option<Object>)]
    hooks: Option<
        std::collections::BTreeMap<String, Vec<lys_home::harness::launch_fields::HookCommand>>,
    >,
    /// The seat's own status command, preserved as declared.
    #[serde(default)]
    status_line: Option<String>,
    /// Extra native settings without the separately governed keys.
    #[serde(default)]
    #[schema(value_type = Option<Object>)]
    settings: Option<serde_json::Map<String, serde_json::Value>>,
    /// The existing conversation explicitly selected for resume.
    #[serde(default)]
    resume: Option<String>,
    /// The context window explicitly declared for this seat, in tokens.
    #[serde(default)]
    context_window: Option<u32>,
    /// The absolute harness config directory explicitly chosen by a person.
    #[serde(default)]
    config_dir: Option<String>,
    instructions: String,
    #[serde(default)]
    #[schema(schema_with = instructions_mode_schema)]
    instructions_mode: InstructionsMode,
    note: String,
    #[serde(default)]
    #[schema(value_type = Option<SessionSettings>)]
    session: Option<session::IncomingSession>,
    #[serde(default)]
    #[schema(value_type = Option<Object>)]
    harness: Option<DeclaredHarness>,
    #[serde(default)]
    permissions: Option<Permissions>,
    #[serde(default)]
    runs_on: Option<String>,
    #[serde(default)]
    writable: Option<String>,
    #[serde(default)]
    working_folder: Option<String>,
}

fn instructions_mode_schema() -> utoipa::openapi::schema::Object {
    utoipa::openapi::schema::ObjectBuilder::new()
        .schema_type(utoipa::openapi::schema::Type::String)
        .enum_values(Some(["keep", "append", "replace"]))
        .default(Some(serde_json::json!("append")))
        .build()
}

/// The provisioning routes.
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .merge(crate::skills_api::routes())
        .route("/agents/{id}/provisioning", get(read).post(set))
        .route(
            "/agents/{id}/provisioning/{version}/review",
            post(reviews::review),
        )
        .route(
            "/agents/{id}/provisioning/{version}/seat-program-review",
            post(reviews::seat_program),
        )
}

fn malformed(reason: impl Into<String>) -> ServerError {
    ServerError::RequestMalformed {
        reason: reason.into(),
    }
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
    let mut store = store
        .lock()
        .map_err(|error| ServerError::ProvisioningUnavailable {
            reason: format!("the provisioning lock is poisoned: {error}"),
        })?;
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

fn view(
    state: &AppState,
    (store, kin): (&ProvisioningStore, &crate::start_checks::Kin),
    agent: &str,
    recorded: Option<Recorded>,
) -> Result<ProvisioningView, ServerError> {
    let profile: Option<&Profile> = store.profile(agent);
    let versions = profile.map_or(&[][..], |profile| profile.versions.as_slice());
    Ok(ProvisioningView {
        agent: agent.to_owned(),
        profile: versions.last().map(|version| VersionView {
            version: version.number,
            operation: version.operation.clone(),
            model_access: version.settings.model_access.clone(),
            tools: version.settings.tools.clone(),
            skills: version.settings.skills.clone(),
            mcp_servers: version.settings.mcp_servers.clone(),
            plugins: version.settings.plugins.clone(),
            hooks: version.settings.hooks.clone(),
            status_line: version.settings.status_line.clone(),
            settings: version.settings.settings.clone(),
            resume: version.settings.resume.clone(),
            context_window: version.settings.context_window,
            config_dir: version.settings.config_dir.clone(),
            instructions: version.settings.instructions.clone(),
            instructions_mode: version.settings.instructions_mode,
            session: version.settings.session.clone(),
            note: version.settings.note.clone(),
            skill_pins: version.settings.skill_pins.clone(),
            harness: version.settings.harness.clone(),
            permissions: version.settings.permissions.clone(),
            runs_on: version.settings.runs_on.clone(),
            writable: version.settings.writable.clone(),
            working_folder: version.settings.working_folder.clone(),
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
            .map(|version| {
                let carried_review = version
                    .reviewed
                    .is_none()
                    .then(|| crate::start_checks::carried(store, kin, version))
                    .flatten();
                VersionLine {
                    version: version.number,
                    set_by: version.set_by.clone(),
                    set_at: version.set_at,
                    note: version.settings.note.clone(),
                    settings_sha256: version.settings.sha256(),
                    reviewed: if version.reviewed.is_some() {
                        "itself"
                    } else if carried_review.is_some() {
                        "carried"
                    } else {
                        "neither"
                    }
                    .to_owned(),
                    carried_review,
                }
            })
            .collect(),
        enforced: crate::launch_api::enforced(state, agent, versions.last())?,
        recorded,
    })
}

async fn read(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<ProvisioningView>, ServerError> {
    let agent = seen_agent(&state, &headers, &id)?.agent.to_string();
    // Read from the directory before the profiles are held (AGENTS-017 R3).
    let kin = crate::start_checks::kin(&state, &agent)?;
    with_provisioning(&state, |store| {
        Ok(Json(view(&state, (store, &kin), &agent, None)?))
    })
}

async fn set(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    body: Result<Json<SetBody>, JsonRejection>,
) -> Result<Json<ProvisioningView>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    // An agent admitted at the door holds agent.provisioning.set on it.
    let acting = crate::agent_acts::admitted().map(|admitted| admitted.agent);
    if acting.is_none() {
        crate::routes::responsible_or_administrator(&state, &headers, &id)?;
    }
    let Json(body) = body.map_err(|refused| malformed(refused.body_text()))?;
    let agent = AgentId::from_str(&id).map_err(|_unread| ServerError::AgentNotVisible)?;
    if let Some(machine) = &body.runs_on
        && body.harness.is_some()
    {
        crate::network_api::computer_owner(&state, &actor, machine)?;
    }
    validation::on_machine(&state, &body).await?;
    with_directory(&state, |directory| {
        let directory = directory.projection()?;
        if directory.record(IdentityId::Agent(agent)).is_none() {
            return Err(ServerError::AgentNotVisible);
        }
        let from_version = body.from_version;
        let migrated = body
            .session
            .as_ref()
            .is_some_and(|session| session.migrated);
        let version = Version {
            number: 0,
            operation: OperationId::from_str(&body.operation)?.to_string(),
            settings: settings(body)?,
            set_by: match acting {
                Some(agent) => agent.to_string(),
                None => own_person(directory, &actor)?.to_string(),
            },
            set_at: now(),
            reviewed: None,
        };
        let agent = agent.to_string();
        let kin = crate::start_checks::kin_in(directory, &agent)?;
        write_provisioning(&state, |store| {
            let operation = version.operation.clone();
            let mut version = version;
            version.settings.skill_pins = match store.pins_for(&version.operation) {
                Some(recorded) => recorded,
                None => store.pins(&version.settings.skills)?,
            };
            let written = !store.profile(&agent).is_some_and(|profile| {
                profile
                    .versions
                    .iter()
                    .any(|kept| kept.operation == operation)
            });
            let version = store.set(&agent, from_version, version)?;
            if migrated && written {
                session::recorded(&agent, version);
            }
            let recorded = Recorded { operation, version };
            Ok(Json(view(&state, (store, &kin), &agent, Some(recorded))?))
        })
    })
}
