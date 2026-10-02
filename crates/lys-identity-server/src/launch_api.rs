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
//! hold, a caller who does not answer for it, an agent with no profile, and
//! a machine unknown, retired, without a runtime or not listing the agent.
//!
//! On a machine whose record names a runner, the kept start is then run by
//! that runner, using signed process inputs and config files, and the answer carries the
//! runner's word beside the command: the session is kept running once the
//! runner says its process is up. A machine without a recorded runner is
//! refused before a new start is kept. Existing starts remain intact when
//! their runner is removed. The service asks the runner over its socket.

use std::str::FromStr;
use std::sync::{Arc, Mutex};

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
use crate::network_store::NetworkStore;
use crate::provisioning_api::with_provisioning;
use crate::provisioning_store::Version;
use crate::routes::{AppState, signed_in, with_directory};
use crate::runtime_api::with_runtime;
use crate::runtime_state::{Report, Reported, Tracked};
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
    start_for(&state, &headers, &actor, agent, &machine, &operation).await
}

/// Give every admitted start the same checks, kept report and runner call.
pub(crate) async fn start_for(
    state: &Arc<AppState>,
    headers: &HeaderMap,
    actor: &Actor,
    agent: AgentId,
    machine: &str,
    operation: &str,
) -> Result<Json<Value>, ServerError> {
    start_profile(state, headers, actor, agent, machine, operation, None).await
}

/// Start an exact reviewed profile through the same admission and launch path.
pub(crate) async fn start_profile(
    state: &Arc<AppState>,
    headers: &HeaderMap,
    actor: &Actor,
    agent: AgentId,
    machine: &str,
    operation: &str,
    profile: Option<Version>,
) -> Result<Json<Value>, ServerError> {
    let session = OperationId::from_str(operation)?.to_string();
    let agent = agent.to_string();
    let admitted_by = start_caller(state, headers, actor, &agent)?;
    let admission = with_directory(state, |directory| {
        let directory = directory.projection()?;
        let parsed = AgentId::from_str(&agent)?;
        let record = directory
            .record(IdentityId::Agent(parsed))
            .ok_or(ServerError::AgentNotVisible)?;
        if let Some(kept) = admitted(state, &session, &agent)? {
            return Ok(Admission::Kept(kept, admitted_by));
        }
        active(record.state())?;
        let version = match profile {
            Some(version) => version,
            None => with_provisioning(state, |store| {
                store
                    .profile(&agent)
                    .and_then(|profile| profile.versions.last().cloned())
                    .ok_or(ServerError::LaunchRecordMissing)
            })?,
        };
        reviewed(&version)?;
        let held = crate::roles_api::held_roles(state, &agent, now())?;
        let runtime = with_network(state, |store| {
            let machine = placed(store, machine, (&agent, &held))?;
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
            return run(state, kept, &admitted_by).await;
        }
        Admission::New(version, runtime, admitted_by) => (version, runtime, admitted_by),
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
        },
        &handles,
    )?;
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
        left_out: Vec::new(),
        executed: false,
    })
    .map_err(|error| ServerError::LaunchUnrenderable {
        reason: error.to_string(),
    })?;
    let network = state
        .network
        .as_ref()
        .ok_or(ServerError::MachineWithoutRunner)?;
    // Session reads take runtime before network; admission must use the same order.
    let kept = with_runtime(state, |store| {
        keep_with_runner(network, machine, || {
            store.report(Report {
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
            })
        })
    })?;
    let kept = kept
        .first()
        .and_then(|first| first.launch.clone())
        .ok_or(ServerError::RuntimeReportReused { operation: session })?;
    run(state, kept, &admitted_by).await
}

fn keep_with_runner(
    network: &Mutex<NetworkStore>,
    machine: &str,
    keep: impl FnOnce() -> Result<Tracked, ServerError>,
) -> Result<Tracked, ServerError> {
    let mut network = network
        .lock()
        .map_err(|error| ServerError::NetworkUnavailable {
            reason: format!("the network lock is poisoned: {error}"),
        })?;
    network.settle()?;
    network
        .runner(machine)
        .ok_or(ServerError::MachineWithoutRunner)?;
    // Removal is ordered after the append, including an uncertain append result.
    let kept = keep();
    drop(network);
    kept
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

/// A replay requires the recorded runner even though its launch is already kept.
async fn run(
    state: &Arc<AppState>,
    view: Value,
    admitted_by: &str,
) -> Result<Json<Value>, ServerError> {
    let member = |name: &str| {
        view.get(name)
            .and_then(Value::as_str)
            .map(str::to_owned)
            .ok_or_else(|| ServerError::LaunchUnrenderable {
                reason: format!("the kept start names no {name}"),
            })
    };
    let (agent, machine) = (member("agent")?, member("machine")?);
    crate::runner_sessions::machine_runner(state, &machine)?
        .ok_or(ServerError::MachineWithoutRunner)?;
    let mut launch = with_provisioning(state, |store| kept_launch(store, &view))?;
    launch.policy = crate::agent_policy_api::launch_policy(state, &agent)?;
    let ran = crate::runner_sessions::run_on_runner(state, (&agent, &machine, admitted_by), launch)
        .await?;
    let mut view = view;
    view["runner"] = ran;
    Ok(Json(view))
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

/// Build the signed launch using only the kept template and its exact version.
///
/// # Errors
/// Refuses unreadable starts, missing kept versions and invalid template bytes.
pub fn kept_launch(
    store: &crate::provisioning_store::ProvisioningStore,
    view: &Value,
) -> Result<lys_runner::Launch, ServerError> {
    let text = |member: &str| {
        view.get(member)
            .and_then(Value::as_str)
            .ok_or_else(|| ServerError::LaunchUnrenderable {
                reason: format!("the kept start names no {member}"),
            })
    };
    let number = view
        .get("provisioning_version")
        .and_then(Value::as_u64)
        .and_then(|number| u32::try_from(number).ok())
        .ok_or_else(|| ServerError::LaunchUnrenderable {
            reason: "the kept start names no profile version".to_owned(),
        })?;
    let version = store
        .profile(text("agent")?)
        .and_then(|profile| {
            profile
                .versions
                .iter()
                .find(|version| version.number == number)
        })
        .ok_or(ServerError::ProfileVersionUnknown { version: number })?;
    reviewed(version)?;
    let mut native = crate::launch_template::from_template(
        version,
        text("template")?,
        text("template_sha256")?,
    )?;
    let handles: Vec<HandleName> =
        serde_json::from_value(view.get("handles").cloned().ok_or_else(|| {
            ServerError::LaunchUnrenderable {
                reason: "the kept start names no handles".to_owned(),
            }
        })?)
        .map_err(|error| ServerError::LaunchUnrenderable {
            reason: format!("the kept handles do not read: {error}"),
        })?;
    native.environment.insert(
        "LYS_LAUNCH_TEMPLATE".to_owned(),
        text("template_sha256")?.to_owned(),
    );
    native.environment.insert(
        "LYS_HANDLES".to_owned(),
        handles
            .iter()
            .map(|handle| handle.id.as_str())
            .collect::<Vec<_>>()
            .join(","),
    );
    let files = native
        .files
        .into_iter()
        .map(|file| lys_runner::launch_config::File {
            path: file.path,
            text: file.text,
            sha256: file.sha256,
        })
        .collect();
    Ok(lys_runner::Launch {
        session: text("session")?.to_owned(),
        program: native.program,
        arguments: native.arguments,
        directory: String::new(),
        environment: native.environment,
        config: Some(lys_runner::launch_config::Config {
            files,
            argument_files: native.argument_files,
            environment_paths: native.environment_paths,
            working_directory: true,
        }),
        columns: crate::runner_sessions::COLUMNS,
        rows: crate::runner_sessions::ROWS,
        rotation: version
            .settings
            .session
            .as_ref()
            .and_then(|session| session.accounts.clone()),
        policy: None,
    })
}

/// A latest reviewed profile is applied only after its whole native launch ran.
pub(crate) fn enforced(
    state: &AppState,
    agent: &str,
    version: Option<&Version>,
) -> Result<bool, ServerError> {
    let Some(version) = version.filter(|version| version.reviewed.is_some()) else {
        return Ok(false);
    };
    if state.runtime.is_none() {
        return Ok(false);
    }
    with_runtime(state, |store| {
        for tracked in store
            .sessions()
            .iter()
            .filter(|tracked| tracked.agent.as_deref() == Some(agent))
        {
            let Some(latest) = tracked.latest() else {
                continue;
            };
            if latest.state != Reported::Running
                || latest.reported_by != format!("the runner of machine {}", tracked.machine)
                || !latest.what.starts_with("the runner started process ")
            {
                continue;
            }
            let Some(view) = tracked.first().and_then(|first| first.launch.as_ref()) else {
                continue;
            };
            if view.get("provisioning_version").and_then(Value::as_u64)
                != Some(u64::from(version.number))
                || !view
                    .get("left_out")
                    .and_then(Value::as_array)
                    .is_some_and(Vec::is_empty)
            {
                continue;
            }
            let template = view
                .get("template")
                .and_then(Value::as_str)
                .ok_or_else(|| ServerError::LaunchUnrenderable {
                    reason: "the running start names no template".to_owned(),
                })?;
            let digest = view
                .get("template_sha256")
                .and_then(Value::as_str)
                .ok_or_else(|| ServerError::LaunchUnrenderable {
                    reason: "the running start names no template digest".to_owned(),
                })?;
            let native = crate::launch_template::from_template(version, template, digest)?;
            let handles: Vec<HandleName> =
                serde_json::from_value(view.get("handles").cloned().ok_or_else(|| {
                    ServerError::LaunchUnrenderable {
                        reason: "the running start names no handles".to_owned(),
                    }
                })?)
                .map_err(|error| ServerError::LaunchUnrenderable {
                    reason: error.to_string(),
                })?;
            if view.get("command").and_then(Value::as_str)
                == Some(crate::launch_template::command(&native, digest, &handles).as_str())
            {
                return Ok(true);
            }
        }
        Ok(false)
    })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::error::Error;
    use std::path::{Path, PathBuf};
    use std::sync::{Arc, Mutex, TryLockError};

    use lys_core::Ed25519Identity;
    use serde_json::json;

    use super::keep_with_runner;
    use crate::error::ServerError;
    use crate::network_store::{Machine, NetworkStore};
    use crate::runner_client::RunnerRecord;
    use crate::runtime_state::{Report, Reported};
    use crate::runtime_store::RuntimeStore;

    fn bytes(root: &Path) -> Result<BTreeMap<PathBuf, Vec<u8>>, Box<dyn Error>> {
        let mut kept = BTreeMap::new();
        for entry in std::fs::read_dir(root)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                kept.extend(bytes(&entry.path())?);
            } else {
                kept.insert(entry.path(), std::fs::read(entry.path())?);
            }
        }
        Ok(kept)
    }

    fn remove_at_append(network: &Mutex<NetworkStore>, machine: &str) -> Result<bool, ServerError> {
        match network.try_lock() {
            Ok(mut store) => {
                store.name_runner(machine, None)?;
                Ok(true)
            }
            Err(TryLockError::WouldBlock) => Ok(false),
            Err(TryLockError::Poisoned(error)) => Err(ServerError::NetworkUnavailable {
                reason: format!("the network lock is poisoned: {error}"),
            }),
        }
    }

    #[test]
    fn runner_removal_is_ordered_before_or_after_the_starting_append() -> Result<(), Box<dyn Error>>
    {
        let dir = tempfile::tempdir()?;
        let runtime_path = dir.path().join("runtime");
        let key = Arc::new(Ed25519Identity::load_or_generate(&dir.path().join("key"))?);
        let mut runtime = RuntimeStore::open(&runtime_path, key)?;
        let mut network = NetworkStore::open(&dir.path().join("network.json"))?;
        let machine = "op-00000000000000000000000000000001";
        network.name(Machine {
            id: machine.to_owned(),
            name: "Computer".to_owned(),
            kind: "computer".to_owned(),
            runtime: Some("sh".to_owned()),
            slots: 1,
            may_run: vec!["agent".to_owned()],
            may_run_roles: Vec::new(),
            may_reach: Vec::new(),
            named_by: "person".to_owned(),
            named_at: 1,
            retired: None,
            team: None,
            creation_team: None,
        })?;
        network.name_runner(machine, Some(RunnerRecord::Lys))?;
        network.name_runner(machine, None)?;
        let network = Mutex::new(network);
        let report = Report {
            operation: "start".to_owned(),
            session: "start".to_owned(),
            agent: Some("agent".to_owned()),
            machine: machine.to_owned(),
            state: Reported::Starting,
            what: "start admitted".to_owned(),
            confirmation: String::new(),
            reported_by: "person".to_owned(),
            at: 1,
            launch: Some(json!({"session": "start"})),
        };
        let before = bytes(&runtime_path)?;
        let mut called = false;
        let refused = keep_with_runner(&network, machine, || {
            called = true;
            runtime.report(report.clone())
        });
        assert!(
            matches!(refused, Err(ServerError::MachineWithoutRunner)),
            "{refused:?}"
        );
        assert!(!called, "removal before admission must prevent the append");
        assert!(runtime.sessions().is_empty());
        assert_eq!(bytes(&runtime_path)?, before);

        network
            .lock()
            .map_err(|error| error.to_string())?
            .name_runner(machine, Some(RunnerRecord::Lys))?;
        let mut removed = None;
        let held = keep_with_runner(&network, machine, || {
            removed = Some(remove_at_append(&network, machine)?);
            runtime.report(report.clone())
        })?;
        assert_eq!(
            removed,
            Some(false),
            "runner removal completed between admission and the Starting append"
        );
        assert_eq!(held.first(), Some(&report));
        assert_eq!(held.reports.len(), 1);
        assert_eq!(runtime.sessions().len(), 1);
        assert_eq!(
            network
                .lock()
                .map_err(|error| error.to_string())?
                .runner(machine),
            Some(&RunnerRecord::Lys)
        );
        let appended = bytes(&runtime_path)?;
        assert_ne!(appended, before);
        assert!(
            remove_at_append(&network, machine)?,
            "the append must release the network lock"
        );
        assert!(
            network
                .lock()
                .map_err(|error| error.to_string())?
                .runner(machine)
                .is_none()
        );
        assert_eq!(runtime.session("start"), Some(&held));
        assert_eq!(bytes(&runtime_path)?, appended);

        network
            .lock()
            .map_err(|error| error.to_string())?
            .name_runner(machine, Some(RunnerRecord::Lys))?;
        let failed = keep_with_runner(&network, machine, || {
            Err(ServerError::RuntimeUnavailable {
                reason: "append refused".to_owned(),
            })
        });
        assert!(
            matches!(failed, Err(ServerError::RuntimeUnavailable { ref reason }) if reason == "append refused")
        );
        assert_eq!(bytes(&runtime_path)?, appended);
        assert!(
            remove_at_append(&network, machine)?,
            "a refused append must release the network lock"
        );
        assert_eq!(runtime.session("start"), Some(&held));
        Ok(())
    }
}
