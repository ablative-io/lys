//! What the runner routes share: who may operate an agent, which runner a
//! session is driven through, how a session is started on it, and how what
//! the runner saw is kept in the runtime reports.
//!
//! A session is shown running only once its runner said its process is up,
//! and stopped only once its runner said the process ended: each is kept as
//! the runtime report the runner's answer confirms, never inferred.

use std::collections::BTreeMap;
use std::str::FromStr;
use std::sync::Arc;

use axum::http::HeaderMap;
use lys_identity::grants::{Action, ExerciseRequest, Resource, Route};
use lys_identity::start::Given;
use lys_identity::{AgentId, IdentityId};
use lys_runner::{Act, Answer, Ended, EndedHow, Launch};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::error::ServerError;
use crate::grants::{caller, with_grants};
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
    let granted = with_grants(state, |judged| {
        let request = ExerciseRequest {
            caller: asker,
            route: Route::Api,
            resource: Resource::new("agent", agent)?,
            action: Action::new(OPERATE)?,
        };
        Ok(judged
            .grants
            .explain(judged.directory, &request, now(), None)
            .is_ok())
    })?;
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
pub fn confirmation(ended: &Ended) -> String {
    let how = match ended.how {
        EndedHow::Exited => "the process exited",
        EndedHow::EndedByRunnerRestart => {
            "ended_by_runner_restart: the runner restarted and found it gone"
        }
        EndedHow::AccountsExhausted => {
            "accounts_exhausted: a usage limit on the last account of its list"
        }
    };
    let status = match (&ended.status, &ended.signal) {
        (Some(status), _) => format!("exit status {status}"),
        (None, Some(signal)) => format!("signal {signal}"),
        (None, None) => "no exit status".to_owned(),
    };
    format!("{how}, {status}, at {} ms since the Unix epoch", ended.at)
}

/// Keep `driven`'s end as its runner saw it, once.
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
pub fn kind(answer: &Answer) -> &'static str {
    match answer {
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
        Answer::Feed { .. } => "feed",
        Answer::GrantChannel => "grant_channel",
        Answer::Refused { .. } => "refused",
    }
}

/// Keep `act`, answering its receipt.
pub fn keep_act(state: &AppState, act: RunnerAct) -> Result<ActReceipt, ServerError> {
    let mut acts = state
        .acts
        .lock()
        .map_err(|error| ServerError::RuntimeUnavailable {
            reason: format!("runner act store unavailable: {error}"),
        })?;
    acts.keep(act)
}

/// Start `launch` for `agent` on `machine`'s runner and keep it running;
/// refuse when the machine names no runner. A start sent again finds the
/// session the runner already holds.
pub async fn run_on_runner(
    state: &Arc<AppState>,
    (agent, machine, caller): (&str, &str, &str),
    launch: Launch,
) -> Result<Value, ServerError> {
    let runner = machine_runner(state, machine)?.ok_or(ServerError::MachineWithoutRunner)?;
    let driven = Driven {
        session: launch.session.clone(),
        agent: agent.to_owned(),
        machine: machine.to_owned(),
        runner: runner.clone(),
    };
    let session = launch.session.clone();
    let agent_id = AgentId::from_str(agent)?;
    let record = launch
        .environment
        .get("LYS_LAUNCH_RECORD")
        .map_or(session.as_str(), String::as_str)
        .to_owned();
    // The check that no pass is held and the issue take one lock, so two
    // starts of the same session cannot both issue.
    let issued =
        crate::agent_pass::store(state)?.issue_unless_present(agent_id, &record, &session)?;
    let act = match issued {
        None => Act::Status {
            session: Some(session.clone()),
        },
        Some(pass) => Act::Start {
            launch: Box::new(launch),
            lys_mcp: Some(lys_runner::protocol::LysMcp {
                url: format!("{}/api/mcp", state.oidc.public_origin()),
                pass: pass.to_string(),
            }),
        },
    };
    let asked = crate::runner_client::ask(state, machine, runner.clone(), act).await;
    let answer = match asked {
        Err(ServerError::Runner { refusal, .. }) if refusal == "session_exists" => {
            let act = Act::Status {
                session: Some(driven.session.clone()),
            };
            crate::runner_client::ask(state, machine, runner, act).await
        }
        other => other,
    };
    if answer.is_err() {
        crate::agent_pass::end_session(state, &session)?;
    }
    let outcome = answer
        .as_ref()
        .map_or_else(ServerError::name, |answer| kind(answer).to_owned());
    keep_act(
        state,
        RunnerAct {
            act: "start".to_owned(),
            caller: caller.to_owned(),
            session: driven.session.clone(),
            agent: agent.to_owned(),
            machine: machine.to_owned(),
            at: now(),
            text: None,
            keys: Vec::new(),
            outcome,
        },
    )?;
    let (pid, started_at, ended) = match answer? {
        Answer::Started {
            pid, started_at, ..
        } => (Some(pid), started_at, None),
        Answer::Status { status } => {
            let held = status
                .sessions
                .into_iter()
                .next()
                .ok_or(ServerError::RuntimeSessionUnknown)?;
            (held.pid, held.started_at, held.ended)
        }
        other => {
            return Err(ServerError::Runner {
                refusal: "runner_reply_malformed".to_owned(),
                words: format!("a start was answered {}", kind(&other)),
            });
        }
    };
    match (pid, &ended) {
        (Some(pid), _) => record_running(state, &driven, pid)?,
        (None, None) => {
            return Err(ServerError::Runner {
                refusal: "runner_reply_malformed".to_owned(),
                words: format!(
                    "the runner holds session {} running with no process id",
                    driven.session
                ),
            });
        }
        (None, Some(_)) => {}
    }
    if let Some(ended) = &ended {
        record_end(state, &driven, ended)?;
    }
    Ok(json!({
        "session": driven.session,
        "state": if ended.is_some() { "ended" } else { "running" },
        "pid": pid,
        "started_at": started_at,
        "ended": ended,
    }))
}

/// The directory service's launcher: a given start is run by the runner its
/// machine's record names, moving between the accounts the agent's profile
/// names at a usage limit, and nothing is run on a machine that names none.
pub struct DirectoryLauncher(pub Arc<AppState>);

/// The session a launch record's start runs as: the same each time the
/// record is run, so a start sent again finds the session already held.
fn launch_session(record: &str) -> String {
    let digest = Sha256::digest(format!("lys-identity/launch-session/v1\n{record}"));
    format!("op-{}", &hex(&digest)[..32])
}

/// Keep that `driven` is starting from launch record `record`, once.
fn record_starting(
    state: &AppState,
    driven: &Driven,
    record: &str,
    caller: &str,
) -> Result<(), ServerError> {
    if state.runtime.is_none() {
        return Ok(());
    }
    with_runtime(state, |store| {
        if store.session(&driven.session).is_some() {
            return Ok(());
        }
        store
            .report(Report {
                operation: driven.session.clone(),
                session: driven.session.clone(),
                agent: Some(driven.agent.clone()),
                machine: driven.machine.clone(),
                state: Reported::Starting,
                what: format!("start given from launch record {record}"),
                confirmation: String::new(),
                reported_by: caller.to_owned(),
                at: now(),
                launch: None,
            })
            .map(drop)
    })
}

impl Launcher for DirectoryLauncher {
    fn launch<'a>(&'a self, given: &'a Given, caller: &'a str) -> LaunchFuture<'a> {
        Box::pin(async move {
            let record = &given.record;
            let runner = match machine_runner(&self.0, &record.machine) {
                Ok(Some(runner)) => runner,
                Ok(None) => return None,
                Err(refused) => return Some(Err(refused)),
            };
            let driven = Driven {
                session: launch_session(&record.id),
                agent: record.agent.clone(),
                machine: record.machine.clone(),
                runner,
            };
            let rotation = match crate::runner_api::session_settings(&self.0, &record.agent) {
                Ok(settings) => settings.and_then(|settings| settings.accounts),
                Err(refused) => return Some(Err(refused)),
            };
            let policy = match crate::agent_policy_api::launch_policy(&self.0, &record.agent) {
                Ok(policy) => policy,
                Err(refused) => return Some(Err(refused)),
            };
            if let Err(refused) = record_starting(&self.0, &driven, &record.id, caller) {
                return Some(Err(refused));
            }
            let environment = BTreeMap::from([
                ("LYS_AGENT".to_owned(), record.agent.clone()),
                ("LYS_SESSION".to_owned(), driven.session.clone()),
                ("LYS_LAUNCH_RECORD".to_owned(), record.id.clone()),
                ("LYS_HANDLES".to_owned(), record.credential_ids.join(",")),
            ]);
            let launch = Launch {
                session: driven.session,
                program: record.executable.clone(),
                arguments: record.arguments.clone(),
                directory: record.working_directory.clone(),
                environment,
                config: None,
                columns: COLUMNS,
                rows: ROWS,
                rotation,
                policy,
            };
            Some(run_on_runner(&self.0, (&record.agent, &record.machine, caller), launch).await)
        })
    }
}
