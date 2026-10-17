//! The start-command route asks the chosen machine's recorded runner to
//! start the agent and returns the runner's answer with the rendered command.
//!
//! The command is rendered from the agent's kept provisioning profile, as a
//! launch template the home keeps by hash, and names the agent's identity,
//! the session the started agent reports under, the machine and the ids of
//! the handles it holds. A credential's value never reaches it: the broker
//! lists handles without their token, and only their ids are named.
//!
//! A start is admitted once, under the caller's operation id, and kept in
//! the runtime reports' log as the session's `starting` report before the
//! command is answered; the session is that operation id. The same request
//! sent again uses the start kept whole in that report, provided the machine
//! still names a runner; the same operation id naming
//! any other report is refused. Only an active agent is started, only on a
//! machine that lists the agent or a role it holds and whose egress list
//! names every host its profile's servers are reached at, and only from a
//! profile version someone answering for the agent reviewed.
//!
//! The administrator and the person responsible for the agent are given
//! the command. Each refusal is by name: an agent the directory does not
//! hold, a caller who does not answer for it, an agent with no profile, a
//! machine unknown, retired, without a runtime or not listing the agent, and
//! a launch that names no working folder when the profile has no default.
//!
//! Runs go where the launch says they go (Tom, 3 October 2026): the launch
//! may name the folder the harness starts in; else the agent's own default
//! from its profile is used; else the start is refused by name. There is no
//! folder for every agent and the runner's home is never used. The folder
//! is kept on the start's record and handed to the runner as the directory
//! the harness is started in.
//!
//! On a machine whose record names a runner, the kept start is then run by
//! that runner, using signed process inputs and config files, and the answer carries the
//! runner's word beside the command: the session is kept running once the
//! runner says its process is up. A machine without a recorded runner is
//! refused before a new start is kept. Existing starts remain intact when
//! their runner is removed. The service asks the runner over its socket.

mod kept;
pub use kept::kept_launch;

pub(crate) mod ordinary;
mod runner;
pub(crate) use runner::enforced;
#[cfg(test)]
use runner::keep_with_runner;
pub(crate) use runner::{keep_start, run};

use std::str::FromStr;
use std::sync::Arc;

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::routing::post;
use axum::{Json, Router};
use lys_identity::{Actor, AgentId, IdentityId, OperationId};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::agent_policy_api::with_policies;
use crate::error::ServerError;
use crate::grants::caller;
use crate::launch_harness::skill_files;
use crate::launch_template::{HandleName, Start, render};
use crate::network_api::with_network;
use crate::provisioning_api::with_provisioning;
use crate::provisioning_store::Version;
use crate::routes::{AppState, signed_in, with_directory};
use crate::runtime_api::with_runtime;
use crate::runtime_state::{Report, Reported};
use crate::session::now;
pub(crate) use crate::start_checks::placed;
use crate::start_checks::{active, handles, reaches, reviewed};

/// The answer of the start-command route.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct StartCommandView {
    /// The agent.
    pub agent: String,
    /// The machine it starts on.
    pub machine: String,
    /// The runtime installed on that machine.
    pub runtime: String,
    /// The session the started agent reports under.
    pub session: String,
    /// The profile version it starts from.
    pub provisioning_version: u32,
    /// The harness the template renders for.
    pub harness: String,
    /// The handles the command names, by id.
    pub handles: Vec<HandleName>,
    /// The launch template, exactly the bytes its hash is of.
    pub template: String,
    /// The template's SHA-256.
    pub template_sha256: String,
    /// The command, as text.
    pub command: String,
    /// The folder the harness is started in: the launch's, else the
    /// profile's default.
    pub directory: String,
    /// What of the profile the template does not carry.
    pub left_out: Vec<String>,
    /// Whether the service ran it: never. A runner may have; its word is
    /// the answer's `runner` member.
    pub executed: bool,
    /// An existing harness conversation selected by a seat start.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conversation: Option<String>,
    /// The native terminal conversation selected before launch, when known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub native_conversation: Option<String>,
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = LaunchBody)]
pub(crate) struct Launch {
    machine: String,
    operation: String,
    /// Explicitly begin a new conversation instead of resuming a known one.
    #[serde(default)]
    fresh: bool,
    /// The folder the harness starts in for this run; absent, the profile's
    /// own default, and with neither the start is refused by name.
    #[serde(default)]
    directory: Option<String>,
}

/// The start-command route.
pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route(
        "/agents/{id}/start-command",
        post(start_command).get(ordinary::preview),
    )
}

async fn start_command(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    body: Result<Json<Launch>, JsonRejection>,
) -> Result<Json<Value>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    let agent = AgentId::from_str(&id).map_err(|_unread| ServerError::AgentNotVisible)?;
    let Json(Launch {
        machine,
        operation,
        directory,
        fresh,
    }) = body.map_err(|refused| ServerError::RequestMalformed {
        reason: refused.body_text(),
    })?;
    let directory = directory
        .map(|given| crate::provisioning_api::folder("directory", given))
        .transpose()?;
    OperationId::from_str(&operation)?;
    start_caller(&state, &headers, &actor, &agent.to_string())?;
    if let Some(start) = crate::seats_run::start_from_agent(
        &state,
        &headers,
        &agent.to_string(),
        &machine,
        &operation,
        directory.as_deref(),
        fresh,
    )
    .await?
    {
        return Ok(Json(start));
    }
    let chosen = ordinary::chosen(
        &state,
        &agent.to_string(),
        &machine,
        &operation,
        directory,
        fresh,
    )
    .await?;
    start_profile(
        &state, &headers, &actor, agent, &machine, &operation, chosen,
    )
    .await
}

/// What a start may choose beyond its agent and machine: an exact reviewed
/// profile, and the folder the harness starts in for this run.
#[derive(Debug, Default)]
pub(crate) struct Chosen {
    /// The exact profile version; absent, the latest reviewed one.
    pub(crate) profile: Option<Version>,
    /// The run's folder; absent, the profile's own default.
    pub(crate) directory: Option<String>,
    /// Existing conversation to resume; absent, begin a fresh one.
    pub(crate) conversation: Option<String>,
}

/// Start through the same admission and launch path, from the exact profile
/// and in the folder `chosen` names when it names them.
pub(crate) async fn start_profile(
    state: &Arc<AppState>,
    headers: &HeaderMap,
    actor: &Actor,
    agent: AgentId,
    machine: &str,
    operation: &str,
    chosen: Chosen,
) -> Result<Json<Value>, ServerError> {
    Box::pin(start_as(
        state,
        headers,
        Who::Caller(actor),
        agent,
        (machine, operation),
        chosen,
    ))
    .await
}

/// Who a start is for: a caller the start command admits only as the
/// administrator or the person responsible for the agent, or one a seat's
/// own act already admitted under its rights (AGENTS-002 R3), named as the
/// start's records name it.
pub(crate) enum Who<'a> {
    /// A caller still to be admitted to the start command.
    Caller(&'a Actor),
    /// A caller already admitted, by its identity.
    Admitted(String),
}

/// The one start path every start takes: the same checks, kept report and
/// runner call, for `who`.
pub(crate) async fn start_as(
    state: &Arc<AppState>,
    headers: &HeaderMap,
    who: Who<'_>,
    agent: AgentId,
    (machine, operation): (&str, &str),
    chosen: Chosen,
) -> Result<Json<Value>, ServerError> {
    let Chosen {
        profile,
        directory,
        conversation,
    } = chosen;
    // The body's shape is refused before any record is consulted, as a start
    // naming no machine already is: a folder that is not an absolute, plain
    // path is RequestMalformed whatever the machine's or the agent's state.
    let named = directory
        .map(|given| crate::provisioning_api::folder("directory", given))
        .transpose()?;
    let session = OperationId::from_str(operation)?.to_string();
    let agent = agent.to_string();
    let record_seat = matches!(&who, Who::Caller(_));
    let admitted_by = match who {
        Who::Caller(actor) => start_caller(state, headers, actor, &agent)?,
        Who::Admitted(by) => by,
    };
    let admission = admit_start(state, (&agent, &session, machine), profile, admitted_by)?;
    let (version, runtime, admitted_by) = match admission {
        Admission::Kept(kept, admitted_by) => {
            if kept.get("conversation").and_then(Value::as_str) != conversation.as_deref() {
                return Err(ServerError::RuntimeReportReused { operation: session });
            }
            return run(state, kept, &admitted_by, record_seat).await;
        }
        Admission::New(version, runtime, admitted_by) => (version, runtime, admitted_by),
    };
    validate_conversation(&version, conversation.as_deref())?;
    let directory = match named {
        Some(given) => given,
        None => version
            .settings
            .working_folder
            .clone()
            .ok_or(ServerError::WorkingFolderUnnamed)?,
    };
    let handles = handles(state, headers, &agent).await?;
    let skills = with_provisioning(state, |store| skill_files(store, &version))?;
    let policy = match state.policies {
        Some(_) => with_policies(state, |store| Ok(store.held().latest(&agent).cloned()))?,
        None => None,
    };
    let rendered = render(
        &Start {
            agent: &agent,
            session: &session,
            machine,
            runtime: &runtime,
            version: &version,
            skills: &skills,
            policy: policy.as_ref(),
            model_proxy: crate::model_proxy_route::for_machine(state, machine)?,
        },
        &handles,
    )?;
    let native_conversation = native_for_start(&version, conversation.as_deref(), &session)?;
    let view = serde_json::to_value(StartCommandView {
        agent: agent.clone(),
        machine: machine.to_owned(),
        runtime,
        session: session.clone(),
        provisioning_version: version.number,
        harness: rendered.harness,
        handles,
        template: rendered.template,
        template_sha256: rendered.template_sha256.clone(),
        command: rendered.command,
        directory,
        left_out: Vec::new(),
        executed: false,
        conversation,
        native_conversation,
    })
    .map_err(|error| ServerError::LaunchUnrenderable {
        reason: error.to_string(),
    })?;
    let pending = state.runners.begin_start(machine, &session)?;
    let kept = runner::keep_start(
        state,
        Report {
            operation: session.clone(),
            session: session.clone(),
            agent: Some(agent.clone()),
            machine: machine.to_owned(),
            state: Reported::Starting,
            what: format!("start admitted, template {}", rendered.template_sha256),
            confirmation: String::new(),
            reported_by: admitted_by.clone(),
            at: now(),
            launch: Some(view),
        },
    )?;
    let kept = kept
        .first()
        .and_then(|first| first.launch.clone())
        .ok_or(ServerError::RuntimeReportReused { operation: session })?;
    let result = run(state, kept, &admitted_by, record_seat).await;
    drop(pending);
    result
}

/// Authenticate the caller and require the existing start authority.
pub(crate) fn start_caller(
    state: &AppState,
    headers: &HeaderMap,
    actor: &Actor,
    agent: &str,
) -> Result<String, ServerError> {
    with_directory(state, |directory| {
        let directory = directory.projection()?;
        let admitted_by = caller(state, headers, directory)?.to_string();
        let parsed = AgentId::from_str(agent)?;
        let record = directory
            .record(IdentityId::Agent(parsed))
            .ok_or(ServerError::AgentNotVisible)?;
        let answers = state.admission.is_administrator(directory, actor)?
            || directory
                .person_for(actor.binding())
                .is_some_and(|person| record.responsible() == Some(person));
        if !answers {
            return Err(ServerError::NotAdmitted {
                reason: "only the administrator or the person responsible for the agent is given its start command",
            });
        }
        Ok(admitted_by)
    })
}

/// A start already admitted, or what a new one is rendered from.
enum Admission {
    Kept(Value, String),
    New(Box<Version>, String, String),
}

/// The start kept under `session` for `agent`, answered exactly as it was
/// first; none when nothing is kept under it, and refused when the id names
/// any other report.
pub(crate) fn admitted(
    state: &AppState,
    session: &str,
    agent: &str,
) -> Result<Option<Value>, ServerError> {
    with_runtime(state, |store| {
        let Some(tracked) = store.session(session) else {
            return Ok(None);
        };
        tracked
            .first()
            .filter(|first| first.operation == session && tracked.agent.as_deref() == Some(agent))
            .and_then(|first| first.launch.clone())
            .map(Some)
            .ok_or_else(|| ServerError::RuntimeReportReused {
                operation: session.to_owned(),
            })
    })
}

fn validate_conversation(version: &Version, conversation: Option<&str>) -> Result<(), ServerError> {
    if let Some(conversation) = conversation {
        let harness = version
            .settings
            .harness
            .as_ref()
            .and_then(crate::launch_template::runner_harness);
        let transport = match harness {
            Some(lys_runner::tracking::Harness::ClaudeCode) => {
                lys_runner::harness_control::Transport::Claude
            }
            Some(lys_runner::tracking::Harness::Codex) => {
                lys_runner::harness_control::Transport::Codex
            }
            None => lys_runner::harness_control::Transport::Pty,
        };
        transport.validate_resume(conversation)?;
    }
    Ok(())
}

fn admit_start(
    state: &AppState,
    (agent, session, machine): (&str, &str, &str),
    profile: Option<Version>,
    admitted_by: String,
) -> Result<Admission, ServerError> {
    with_directory(state, |directory| {
        let directory = directory.projection()?;
        let parsed = AgentId::from_str(agent)?;
        let record = directory
            .record(IdentityId::Agent(parsed))
            .ok_or(ServerError::AgentNotVisible)?;
        if let Some(kept) = admitted(state, session, agent)? {
            return Ok(Admission::Kept(kept, admitted_by));
        }
        active(record.state())?;
        let version = match profile {
            Some(version) => version,
            None => with_provisioning(state, |store| {
                store
                    .profile(agent)
                    .and_then(|profile| profile.versions.last().cloned())
                    .ok_or(ServerError::LaunchRecordMissing)
            })?,
        };
        reviewed(&version)?;
        let held = crate::roles_api::held_roles(state, agent, now())?;
        let runtime = with_network(state, |store| {
            let machine = placed(store, machine, (agent, &held))?;
            reaches(machine, &version)?;
            machine
                .runtime
                .clone()
                .ok_or(ServerError::MachineWithoutRuntime)
        })?;
        Ok(Admission::New(Box::new(version), runtime, admitted_by))
    })
}

#[cfg(test)]
#[path = "launch_api/admission_tests.rs"]
mod tests;

fn native_for_start(
    version: &crate::provisioning_store::Version,
    conversation: Option<&str>,
    session: &str,
) -> Result<Option<String>, ServerError> {
    Ok(if crate::launch_template::requires_controls(version) {
        None
    } else {
        match version
            .settings
            .harness
            .as_ref()
            .and_then(crate::launch_template::runner_harness)
        {
            Some(lys_runner::tracking::Harness::ClaudeCode) => Some(match conversation {
                Some(conversation) => conversation.to_owned(),
                None => lys_runner::pty_conversation::conversation_uuid(session)?,
            }),
            Some(lys_runner::tracking::Harness::Codex) => conversation.map(str::to_owned),
            None => None,
        }
    })
}
