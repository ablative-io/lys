//! What the runner routes share: who may operate an agent, which runner a
//! session is driven through, how a session is started on it, and how what
//! the runner saw is kept in the runtime reports.
//!
//! A session is shown running only once its runner said its process is up,
//! and stopped only once its runner said the process ended: each is kept as
//! the runtime report the runner's answer confirms, never inferred.

mod controls;
pub use controls::run_on_runner;
mod starting;
pub(crate) use starting::{launch_session, run_resuming};
#[path = "runner_sessions_refused.rs"]
mod refused;
use refused::{record_refused, refused_before, started_nothing};

use std::str::FromStr;
use std::sync::Arc;

use axum::http::HeaderMap;
use lys_identity::grants::{Action, ExerciseRequest, Resource, Route};
use lys_identity::start::{Given, LaunchRecord};
use lys_identity::{AgentId, IdentityId};
use lys_runner::{Act, Answer, Ended, EndedHow, Launch};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::error::ServerError;
use crate::grants::caller;
use crate::network_api::with_network;
use crate::routes::start::{LaunchFuture, Launcher};
use crate::routes::{AppState, hex, signed_in, with_directory};
use crate::runner_acts::{ActReceipt, RunnerAct};
use crate::runner_client::RunnerRecord;
use crate::runtime_api::with_runtime;
use crate::runtime_state::{Report, Reported};
use crate::session::now;

/// The action a grant gives to drive an agent's sessions.
pub const OPERATE: &str = "operate";

/// A started session's terminal width in columns.
pub const COLUMNS: u16 = 120;

/// A started session's terminal height in rows.
pub const ROWS: u16 = 40;

/// A session driven through its machine's runner.
#[derive(Debug, Clone)]
pub struct Driven {
    /// The session.
    pub session: String,
    /// The agent it is held under.
    pub agent: String,
    /// The machine it runs on.
    pub machine: String,
    /// The machine's runner.
    pub runner: RunnerRecord,
}

/// The runner `machine`'s record names; none when it names none, or when
/// the configuration keeps no machines.
///
/// # Errors
/// Returns network configuration and store read failures.
pub fn machine_runner(
    state: &AppState,
    machine: &str,
) -> Result<Option<RunnerRecord>, ServerError> {
    if state.network.is_none() {
        return Ok(None);
    }
    with_network(state, |store| Ok(store.runner(machine).cloned()))
}

/// The agent's session `session`, as its runner drives it: a found
/// session, or one on a machine naming no runner, is refused by name.
///
/// # Errors
/// Refuses an unknown runtime session, an absent runner, or unavailable runtime and network stores.
pub fn driven(state: &AppState, session: &str) -> Result<Driven, ServerError> {
    let tracked = with_runtime(state, |store| {
        store
            .session(session)
            .cloned()
            .ok_or(ServerError::RuntimeSessionUnknown)
    })?;
    let agent = tracked
        .agent
        .clone()
        .ok_or(ServerError::RuntimeSessionUnknown)?;
    let runner =
        machine_runner(state, &tracked.machine)?.ok_or_else(|| ServerError::RunnerAbsent {
            machine: tracked.machine.clone(),
        })?;
    Ok(Driven {
        session: tracked.session,
        agent,
        machine: tracked.machine,
        runner,
    })
}

/// The agent a tracked session belongs to, before any runner is looked up.
pub(crate) fn session_agent(state: &AppState, session: &str) -> Result<String, ServerError> {
    with_runtime(state, |store| {
        store
            .session(session)
            .and_then(|tracked| tracked.agent.clone())
            .ok_or(ServerError::RuntimeSessionUnknown)
    })
}

/// Refuse a usage target outside the named agent before recording or acting.
pub(crate) fn usage_session(
    state: &AppState,
    agent: &str,
    session: &str,
) -> Result<(), ServerError> {
    let belongs = with_runtime(state, |store| {
        Ok(store
            .session(session)
            .is_some_and(|tracked| tracked.agent.as_deref() == Some(agent)))
    })?;
    if belongs {
        Ok(())
    } else {
        Err(ServerError::RequestMalformed {
            reason: format!("usage session `{session}` is not tracked under agent `{agent}`"),
        })
    }
}

/// The caller, admitted to operate `agent` for `act`: the administrator,
/// the person responsible for the agent, or the holder of a grant of
/// `operate` on it. Anyone else is refused `not_permitted`, by name.
///
/// # Errors
/// Refuses an unsigned, invisible or unpermitted caller, or an unavailable permission or directory store.
pub fn operator(
    state: &AppState,
    headers: &HeaderMap,
    agent: &str,
    act: &str,
) -> Result<String, ServerError> {
    let actor = signed_in(state, headers)?;
    let id = AgentId::from_str(agent).map_err(|_unread| ServerError::AgentNotVisible)?;
    let administrator = crate::routes::is_administrator(state, &actor)?;
    let (asker, answers) = with_directory(state, |directory| {
        let projection = directory.projection()?;
        let asker = caller(state, headers, projection)?;
        let record = projection
            .record(IdentityId::Agent(id))
            .ok_or(ServerError::AgentNotVisible)?;
        let responsible =
            matches!(asker, IdentityId::Person(person) if record.responsible() == Some(person));
        Ok((asker, administrator || responsible))
    })?;
    if answers {
        return Ok(asker.to_string());
    }
    let granted = crate::grants_reads::with_read_grants(
        state,
        None,
        crate::session::now(),
        |_| {
            let request = ExerciseRequest {
                caller: asker,
                route: Route::Api,
                resource: Resource::new("agent", agent)?,
                action: Action::new(OPERATE)?,
            };
            Ok(vec![request])
        },
        |judged| {
            let request = ExerciseRequest {
                caller: asker,
                route: Route::Api,
                resource: Resource::new("agent", agent)?,
                action: Action::new(OPERATE)?,
            };
            Ok(crate::grants::allowed(judged.grants.explain(
                judged.directory,
                &request,
                now(),
                None,
            ))?)
        },
    )?;
    if granted {
        return Ok(asker.to_string());
    }
    Err(ServerError::NotPermitted {
        reason: format!(
            "{asker} does not hold {OPERATE} on {agent}, so it may not {act} its sessions"
        ),
    })
}

/// An operation id for the runner's report `what` on `session`, the same
/// each time, so the report is kept once.
fn report_id(session: &str, what: &str) -> String {
    let digest = Sha256::digest(format!("lys-identity/runner-report/v1\n{session}\n{what}"));
    format!("op-{}", &hex(&digest)[..32])
}

fn runner_report(driven: &Driven, state: Reported, what: String, confirmation: String) -> Report {
    Report {
        operation: report_id(&driven.session, state.name()),
        session: driven.session.clone(),
        agent: Some(driven.agent.clone()),
        machine: driven.machine.clone(),
        state,
        what,
        confirmation,
        reported_by: format!("the runner of machine {}", driven.machine),
        at: now(),
        launch: None,
    }
}

/// Keep that the runner has `driven`'s process `pid` up, once.
///
/// # Errors
/// Returns runtime store read or durable report failures.
pub fn record_running(state: &AppState, driven: &Driven, pid: u32) -> Result<(), ServerError> {
    if state.runtime.is_none() {
        return Ok(());
    }
    with_runtime(state, |store| {
        let latest = store
            .session(&driven.session)
            .and_then(|tracked| tracked.latest().map(|report| report.state));
        if latest != Some(Reported::Starting) {
            return Ok(());
        }
        let what = format!("the runner started process {pid}");
        store
            .report(runner_report(
                driven,
                Reported::Running,
                what,
                String::new(),
            ))
            .map(drop)
    })
}

/// The runner's words for `ended`.
#[must_use]
pub fn confirmation(ended: &Ended) -> String {
    let how = match ended.how {
        EndedHow::Exited => "the process exited",
        EndedHow::EndedByRunnerRestart => {
            "ended_by_runner_restart: the runner restarted and found it gone"
        }
        EndedHow::AccountsExhausted => {
            "accounts_exhausted: a usage limit on the last account of its list"
        }
        EndedHow::Stopped => "stopped: everything was stopped, or the runner was",
    };
    let status = match (&ended.status, &ended.signal) {
        (Some(status), _) => format!("exit status {status}"),
        (None, Some(signal)) => format!("signal {signal}"),
        (None, None) => "no exit status".to_owned(),
    };
    format!("{how}, {status}, at {} ms since the Unix epoch", ended.at)
}

/// Keep `driven`'s end as its runner saw it, once.
///
/// # Errors
/// Returns runtime report, pass revocation or budget context failures.
pub fn record_end(state: &AppState, driven: &Driven, ended: &Ended) -> Result<(), ServerError> {
    if state.runtime.is_none() {
        return crate::agent_pass::end_session(state, &driven.session);
    }
    with_runtime(state, |store| {
        if store
            .session(&driven.session)
            .is_none_or(crate::runtime_state::Tracked::stopped)
        {
            return Ok(());
        }
        let report = runner_report(
            driven,
            Reported::Stopped,
            String::new(),
            confirmation(ended),
        );
        store.report(report).map(drop)
    })?;
    crate::agent_pass::end_session(state, &driven.session)?;
    crate::budgets_context::finish(state, &driven.agent, &driven.session)
}

/// The end an answer carries, when it carries one.
#[must_use]
pub fn ended_in(answer: &Answer) -> Option<&Ended> {
    match answer {
        Answer::Ended { ended, .. } => Some(ended),
        Answer::Output { output } => output.ended.as_ref(),
        Answer::Bytes { output } => output.ended.as_ref(),
        Answer::Status { status } => status
            .sessions
            .first()
            .and_then(|session| session.ended.as_ref()),
        _ => None,
    }
}

/// An answer's kind, as a receipt names the outcome.
#[must_use]
pub fn kind(answer: &Answer) -> &'static str {
    match answer {
        Answer::Programs { .. } => "programs",
        Answer::Started { .. } => "started",
        Answer::Delivered { .. } => "delivered",
        Answer::Output { .. } => "output",
        Answer::Bytes { .. } => "bytes",
        Answer::Matched { .. } => "matched",
        Answer::Ended { .. } => "ended",
        Answer::Status { .. } => "status",
        Answer::Judged { .. } => "judged",
        Answer::Collected { .. } => "collected",
        Answer::Operation { .. } => "operation",
        Answer::ControlStatus { .. } => "control_status",
        Answer::ControlReceipt { .. } => "control_receipt",
        Answer::ControlReceipts { .. } => "control_receipts",
        Answer::Feed { .. } => "feed",
        Answer::Folders { .. } => "folders",
        Answer::FolderChosen { .. } => "folder_chosen",
        Answer::GrantChannel => "grant_channel",
        Answer::StoppedEverything { .. } => "stopped_everything",
        Answer::Refused { .. } => "refused",
        Answer::Liveness { .. } => "liveness",
        Answer::AttachLines { .. } => "attach_lines",
        Answer::Owner { .. } => "owner",
        Answer::Owned { .. } => "owned",
        Answer::Plugins { .. } => "plugins",
        Answer::UpgradeSessions { .. } => "upgrade_sessions",
        Answer::UpgradeCancelled => "upgrade_cancelled",
    }
}

/// Keep `act`, answering its receipt.
///
/// # Errors
/// Returns an unavailable act store or a durable receipt failure.
pub fn keep_act(state: &AppState, act: RunnerAct) -> Result<ActReceipt, ServerError> {
    let mut acts = state
        .acts
        .lock()
        .map_err(|error| ServerError::RuntimeUnavailable {
            reason: format!("runner act store unavailable: {error}"),
        })?;
    acts.keep(act)
}

/// The directory service's launcher: a given start is run by the runner its
/// machine's record names, moving between the accounts the agent's profile
/// names at a usage limit, and nothing is run on a machine that names none.
pub struct DirectoryLauncher(pub Arc<AppState>);

/// Replay only the full launch that this exact signed record admitted.
fn retained_record(
    state: &AppState,
    record: &LaunchRecord,
    session: &str,
) -> Result<Option<Value>, ServerError> {
    with_runtime(state, |store| {
        let Some(tracked) = store.session(session) else {
            return Ok(None);
        };
        let first = tracked
            .first()
            .ok_or_else(|| ServerError::RuntimeReportReused {
                operation: session.to_owned(),
            })?;
        if tracked.agent.as_deref() != Some(record.agent.as_str())
            || tracked.machine != record.machine
            || first.operation != session
        {
            return Err(ServerError::RuntimeReportReused {
                operation: session.to_owned(),
            });
        }
        let view = first.launch.clone().ok_or_else(|| {
            starting::record_unproved("the prior start has no saved launch template")
        })?;
        if !starting::matches_record(&view, record) {
            return Err(ServerError::RuntimeReportReused {
                operation: session.to_owned(),
            });
        }
        Ok(Some(view))
    })
}

fn record_starting(
    state: &AppState,
    driven: &Driven,
    record: &LaunchRecord,
    caller: &str,
    view: Value,
) -> Result<Value, ServerError> {
    if let Some(kept) = retained_record(state, record, &driven.session)? {
        return Ok(kept);
    }
    let tracked = crate::launch_api::keep_start(
        state,
        Report {
            operation: driven.session.clone(),
            session: driven.session.clone(),
            agent: Some(driven.agent.clone()),
            machine: driven.machine.clone(),
            state: Reported::Starting,
            what: format!("start given from launch record {}", record.id),
            confirmation: String::new(),
            reported_by: caller.to_owned(),
            at: now(),
            launch: Some(view),
        },
    )?;
    tracked
        .first()
        .and_then(|first| first.launch.clone())
        .ok_or_else(|| starting::record_unproved("the admitted start has no saved launch template"))
}

impl Launcher for DirectoryLauncher {
    fn launch<'a>(&'a self, given: &'a Given, caller: &'a str) -> LaunchFuture<'a> {
        Box::pin(starting::launch_given(&self.0, given, caller))
    }
}

/// Only the proxy's per-run key may differ from the signed Codex argument.
fn proxy_argument(given: &str, wanted: &str, proxy: &str, run: &str) -> Result<bool, ServerError> {
    let prefix = "model_providers.lys=";
    let Some(setting) = given.strip_prefix(prefix) else {
        return Ok(false);
    };
    if !wanted.starts_with(prefix) {
        return Ok(false);
    }
    let setting: toml::Table =
        toml::from_str(&format!("provider = {setting}")).map_err(|error| {
            ServerError::LaunchUnrenderable {
                reason: format!("the signed Codex provider setting is invalid TOML: {error}"),
            }
        })?;
    let base = setting
        .get("provider")
        .and_then(|value| value.get("base_url"))
        .and_then(toml::Value::as_str)
        .ok_or_else(|| ServerError::LaunchUnrenderable {
            reason: "the signed Codex provider setting names no base URL".to_owned(),
        })?;
    let Some(old_run) = base
        .split_once("://")
        .and_then(|(_, rest)| rest.split_once('/'))
        .and_then(|(_, path)| path.split('/').next())
        .filter(|run| lys_home::proxy::usage::is_run_key(run))
    else {
        return Ok(false);
    };
    if lys_home::harness::rendering::openai_base(proxy, Some(old_run)).as_deref() != Some(base) {
        return Ok(false);
    }
    let replacement =
        lys_home::harness::rendering::openai_base(proxy, Some(run)).ok_or_else(|| {
            ServerError::LaunchUnrenderable {
                reason: "the Codex model proxy has no scheme and host".to_owned(),
            }
        })?;
    let quote = |value: &str| {
        serde_json::to_string(value).map_err(|error| ServerError::LaunchUnrenderable {
            reason: error.to_string(),
        })
    };
    let original = quote(base)?;
    Ok(given.matches(&original).count() == 1
        && given.replacen(&original, &quote(&replacement)?, 1) == wanted)
}

/// A start owned by the agent's responsible person, so the runner admits
/// input to it only for a verified caller; any other act is sent as it is.
fn owned_start(state: &AppState, agent: &str, act: Act) -> Result<Act, ServerError> {
    if !matches!(act, Act::Start { .. } | Act::StartManaged { .. }) {
        return Ok(act);
    }
    let agent = AgentId::from_str(agent)?;
    let person = crate::routes::with_directory(state, |directory| {
        directory
            .projection()?
            .record(lys_identity::IdentityId::Agent(agent))
            .and_then(lys_identity::projection::Record::responsible)
            .ok_or(ServerError::NoPerson)
    })?;
    Ok(Act::AsCaller {
        caller: person.to_string(),
        done: Box::new(act),
    })
}
