//! The start-command route: the product starts an agent by giving the
//! command that starts it on a chosen machine, never by running it.
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
//! sent again answers the start exactly as it was first answered, kept whole
//! in that report, whatever has changed since; the same operation id naming
//! any other report is refused. Only an active agent is started, only on a
//! machine that lists the agent or a role it holds and whose egress list
//! names every host its profile's servers are reached at, and only from a
//! profile version someone answering for the agent reviewed.
//!
//! The administrator and the person responsible for the agent are given
//! the command. Each refusal is by name: an agent the directory does not
//! hold, a caller who does not answer for it, an agent with no profile, and
//! a machine unknown, retired, without a runtime or not listing the agent.
//!
//! On a machine whose record names a runner, the kept start is then run by
//! that runner, as `/bin/sh -c` and the command, and the answer carries the
//! runner's word beside the command: the session is kept running once the
//! runner says its process is up. On a machine that names none, the command
//! is answered as it always was, and nothing runs. The service itself
//! never runs anything: it asks the runner over its socket.

use std::collections::BTreeSet;
use std::str::FromStr;
use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, Method};
use axum::routing::post;
use axum::{Json, Router};
use lys_identity::LifecycleState;
use lys_identity::{AgentId, IdentityId, OperationId};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::ServerError;
use crate::grants::caller;
use crate::launch_harness::skill_files;
use crate::launch_template::{HandleName, Start, handle_variable, render};
use crate::network_api::with_network;
use crate::network_store::{Machine, NetworkStore};
use crate::provisioning_api::with_provisioning;
use crate::provisioning_store::Version;
use crate::routes::{AppState, signed_in, with_directory};
use crate::runtime_api::with_runtime;
use crate::runtime_state::{Report, Reported};
use crate::session::now;

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
    pub harness: &'static str,
    /// The handles the command names, by id.
    pub handles: Vec<HandleName>,
    /// The launch template, exactly the bytes its hash is of.
    pub template: String,
    /// The template's SHA-256.
    pub template_sha256: String,
    /// The command, as text.
    pub command: String,
    /// What of the profile the template does not carry.
    pub left_out: Vec<String>,
    /// Whether the service ran it: never. A runner may have; its word is
    /// the answer's `runner` member.
    pub executed: bool,
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = LaunchBody)]
pub(crate) struct Launch {
    machine: String,
    operation: String,
}

/// The start-command route.
pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/agents/{id}/start-command", post(start_command))
}

/// The machine `id`, when it takes `agent`: known, in use, with a runtime,
/// and listing the agent among those that may run on it.
pub(crate) fn placed<'a>(
    store: &'a NetworkStore,
    id: &str,
    (agent, held): (&str, &[String]),
) -> Result<&'a Machine, ServerError> {
    let machine = store.machine(id).ok_or(ServerError::MachineUnknown)?;
    if machine.retired.is_some() {
        return Err(ServerError::MachineRetired);
    }
    if machine.runtime.is_none() {
        return Err(ServerError::MachineWithoutRuntime);
    }
    let by_role = machine.may_run_roles.iter().any(|role| held.contains(role));
    if !by_role && !machine.may_run.iter().any(|named| named == agent) {
        return Err(ServerError::MachineNotForAgent);
    }
    Ok(machine)
}

/// The handles `agent` holds that are not dropped, as the broker lists them
/// to the signed-in person, each with the variable the launch sets.
async fn handles(
    state: &AppState,
    headers: &HeaderMap,
    agent: &str,
) -> Result<Vec<HandleName>, ServerError> {
    let path = format!("/_lys/handles?holder={agent}");
    let answer = crate::secrets_api::ask(state, headers, Method::GET, &path, Bytes::new()).await?;
    let unread = |reason: &str| ServerError::SecretsUnavailable {
        reason: format!("the broker's handle list does not read: {reason}"),
    };
    let listed = answer
        .get("handles")
        .and_then(Value::as_array)
        .ok_or_else(|| unread("it holds no handles list"))?;
    let mut held: Vec<(String, String)> = Vec::new();
    for handle in listed {
        let dropped = handle
            .get("dropped")
            .and_then(Value::as_bool)
            .ok_or_else(|| unread("a handle does not say whether it was dropped"))?;
        if dropped {
            continue;
        }
        let text = |name: &str| {
            handle
                .get(name)
                .and_then(Value::as_str)
                .map(str::to_owned)
                .ok_or_else(|| unread(&format!("a handle has no {name}")))
        };
        held.push((text("id")?, text("secret")?));
    }
    held.sort();
    let mut taken = BTreeSet::new();
    Ok(held
        .into_iter()
        .map(|(id, secret)| HandleName {
            env: handle_variable(&secret, &mut taken),
            id,
            secret,
        })
        .collect())
}

async fn start_command(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    body: Result<Json<Launch>, JsonRejection>,
) -> Result<Json<Value>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    let agent = AgentId::from_str(&id).map_err(|_unread| ServerError::AgentNotVisible)?;
    let Json(Launch { machine, operation }) =
        body.map_err(|refused| ServerError::RequestMalformed {
            reason: refused.body_text(),
        })?;
    let session = OperationId::from_str(&operation)?.to_string();
    let agent = agent.to_string();
    let admission = with_directory(&state, |directory| {
        let directory = directory.projection()?;
        let admitted_by = caller(&state, &headers, directory)?.to_string();
        let parsed = AgentId::from_str(&agent)?;
        let record = directory
            .record(IdentityId::Agent(parsed))
            .ok_or(ServerError::AgentNotVisible)?;
        let answers = state.admission.administrator(&actor).is_ok()
            || directory
                .person_for(actor.binding())
                .is_some_and(|person| record.responsible() == Some(person));
        if !answers {
            return Err(ServerError::NotAdmitted {
                reason: "only the administrator or the person responsible for the agent is given its start command",
            });
        }
        if let Some(kept) = admitted(&state, &session, &agent)? {
            return Ok(Admission::Kept(kept, admitted_by));
        }
        if record.state() != LifecycleState::Active {
            return Err(ServerError::AgentNotActive {
                state: record.state().to_string(),
            });
        }
        let version = with_provisioning(&state, |store| {
            store
                .profile(&agent)
                .and_then(|profile| profile.versions.last().cloned())
                .ok_or(ServerError::LaunchRecordMissing)
        })?;
        if version.reviewed.is_none() {
            return Err(ServerError::ProfileNotReviewed {
                version: version.number,
            });
        }
        let held = crate::roles_api::held_roles(&state, &agent, now())?;
        let runtime = with_network(&state, |store| {
            let machine = placed(store, &machine, (&agent, &held))?;
            reaches(machine, &version)?;
            machine
                .runtime
                .clone()
                .ok_or(ServerError::MachineWithoutRuntime)
        })?;
        Ok(Admission::New(Box::new(version), runtime, admitted_by))
    })?;
    let (version, runtime, admitted_by) = match admission {
        Admission::Kept(kept, admitted_by) => {
            let rotation = with_provisioning(&state, |store| {
                Ok(store
                    .profile(&agent)
                    .and_then(|profile| profile.versions.last())
                    .and_then(|version| version.settings.session.as_ref())
                    .and_then(|session| session.accounts.clone()))
            })?;
            return run(&state, kept, &admitted_by, rotation).await;
        }
        Admission::New(version, runtime, admitted_by) => (version, runtime, admitted_by),
    };
    let rotation = version
        .settings
        .session
        .as_ref()
        .and_then(|session| session.accounts.clone());
    let handles = handles(&state, &headers, &agent).await?;
    let skills = with_provisioning(&state, |store| skill_files(store, &version))?;
    let rendered = render(
        &Start {
            agent: &agent,
            session: &session,
            machine: &machine,
            runtime: &runtime,
            version: &version,
            skills: &skills,
        },
        &handles,
    )?;
    let view = serde_json::to_value(StartCommandView {
        agent: agent.clone(),
        machine: machine.clone(),
        runtime,
        session: session.clone(),
        provisioning_version: version.number,
        harness: lys_home::harness::claude_code::HARNESS,
        handles,
        template: rendered.template,
        template_sha256: rendered.template_sha256.clone(),
        command: rendered.command,
        left_out: Vec::new(),
        executed: false,
    })
    .map_err(|error| ServerError::LaunchUnrenderable {
        reason: error.to_string(),
    })?;
    let kept = with_runtime(&state, |store| {
        store.report(Report {
            operation: session.clone(),
            session: session.clone(),
            agent: Some(agent.clone()),
            machine: machine.clone(),
            state: Reported::Starting,
            what: format!("start admitted, template {}", rendered.template_sha256),
            confirmation: String::new(),
            reported_by: admitted_by.clone(),
            at: now(),
            launch: Some(view),
        })
    })?;
    let kept = kept
        .first()
        .and_then(|first| first.launch.clone())
        .ok_or(ServerError::RuntimeReportReused { operation: session })?;
    run(&state, kept, &admitted_by, rotation).await
}

/// Run the start `view` answers on its machine's runner, when the machine
/// names one, and answer the view with the runner's word on it beside it;
/// a machine that names none is answered the view as it is, and nothing
/// runs. The service spawns nothing: it asks the runner.
async fn run(
    state: &Arc<AppState>,
    view: Value,
    admitted_by: &str,
    rotation: Option<lys_runner::Rotation>,
) -> Result<Json<Value>, ServerError> {
    let member = |name: &str| {
        view.get(name)
            .and_then(Value::as_str)
            .map(str::to_owned)
            .ok_or_else(|| ServerError::LaunchUnrenderable {
                reason: format!("the kept start names no {name}"),
            })
    };
    let (agent, machine, session, command) = (
        member("agent")?,
        member("machine")?,
        member("session")?,
        member("command")?,
    );
    let policy = crate::agent_policy_api::launch_policy(state, &agent)?;
    let launch = lys_runner::Launch {
        session,
        program: SHELL.to_owned(),
        arguments: vec!["-c".to_owned(), command],
        directory: String::new(),
        environment: std::collections::BTreeMap::new(),
        columns: crate::runner_sessions::COLUMNS,
        rows: crate::runner_sessions::ROWS,
        rotation,
        policy,
    };
    let ran = crate::runner_sessions::run_on_runner(state, (&agent, &machine, admitted_by), launch)
        .await?;
    let mut view = view;
    if let Some(runner) = ran {
        view["runner"] = runner;
    }
    Ok(Json(view))
}

/// Refuse by name the first host a server of `version` is reached at that
/// `machine`'s egress list does not name. A command server is started on
/// the machine and reached over its own streams, so it names no host here.
fn reaches(machine: &Machine, version: &Version) -> Result<(), ServerError> {
    for server in version
        .settings
        .mcp_servers
        .iter()
        .filter(|server| server.command.is_none())
    {
        let host = url_host(&server.url).ok_or_else(|| ServerError::LaunchUnrenderable {
            reason: format!(
                "server `{}` is reached at `{}`, which names no host",
                server.name, server.url
            ),
        })?;
        if !machine.may_reach.contains(&host) {
            return Err(ServerError::MachineCannotReach { host });
        }
    }
    Ok(())
}

/// The host of `url`, lower-cased, without scheme, credentials, port or path.
fn url_host(url: &str) -> Option<String> {
    let (_scheme, rest) = url.split_once("://")?;
    let authority = rest.split(['/', '?', '#']).next()?;
    let located = authority
        .rsplit_once('@')
        .map_or(authority, |(_user, host)| host);
    let host = located.split(':').next()?.to_ascii_lowercase();
    (!host.is_empty()).then_some(host)
}

/// The shell a runner runs a start command with.
const SHELL: &str = "/bin/sh";

/// A start already admitted, or what a new one is rendered from.
enum Admission {
    Kept(Value, String),
    New(Box<Version>, String, String),
}

/// The start kept under `session` for `agent`, answered exactly as it was
/// first; none when nothing is kept under it, and refused when the id names
/// any other report.
fn admitted(state: &AppState, session: &str, agent: &str) -> Result<Option<Value>, ServerError> {
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
