//! What pulling the cord does, in order, each step kept before the next:
//!
//! 1. the pull is kept, so every start is refused from here on;
//! 2. every tracked session that has not ended is recorded asked to stop,
//!    all in one durable write, naming who pulled the cord and why;
//! 3. the runner of every computer holding such a session is asked to stop
//!    everything, all at once: the local runner, a socket runner, and each
//!    dialled runner that is dialled in now. A runner not dialled in, naming
//!    no runner, or not answering is named unreached with its refusal, and
//!    its sessions stay recorded as asked, never as stopped;
//! 4. every session a runner names as ended by this pull is recorded
//!    stopped, all in one durable write, with who and why, and its run's
//!    pass and context are closed as any confirmed end closes them;
//! 5. the credential handles of every agent asked about are ended at the
//!    broker, each refusal named;
//! 6. what the pull did is kept whole and answered. Sent again under the
//!    same operation id it answers that and does nothing twice.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use axum::http::HeaderMap;
use lys_runner::{Act, Answer};
use sha2::{Digest, Sha256};

use crate::cord_api::with_cord;
use crate::cord_store::{CordStore, HandlesRefused, Named, Pull, PullResult, Pulling, Unreached};
use crate::error::ServerError;
use crate::network_api::with_network;
use crate::routes::{AppState, hex, with_directory};
use crate::runner_client::RunnerRecord;
use crate::runtime_api::with_runtime;
use crate::runtime_state::{Report, Reported};
use crate::session::now;

/// A tracked session that had not ended when the cord was pulled.
struct Open {
    session: String,
    agent: Option<String>,
    machine: String,
}

/// What one computer's runner answered: the sessions it ended and those
/// still running, or why it could not be asked.
type Asked = Result<(Vec<String>, Vec<String>), (String, String)>;

#[derive(Clone, Copy)]
enum Caller<'a> {
    Session(&'a HeaderMap),
    Console,
}

/// An operation id for one part of the pull, the same each time the same
/// pull is sent, so each part is kept once.
fn part(operation: &str, kind: &str, key: &str) -> String {
    let digest = Sha256::digest(format!("lys-identity/cord/v1\n{operation}\n{kind}\n{key}"));
    format!("op-{}", &hex(&digest)[..32])
}

/// Pull the cord as `pull` says, answering what it did.
pub(crate) async fn pull(
    state: &Arc<AppState>,
    headers: &HeaderMap,
    pull: Pull,
) -> Result<PullResult, ServerError> {
    pulling(state, Caller::Session(headers), pull).await
}

/// Pull from the console, ending each agent's handles for its responsible person.
pub(crate) async fn console(state: &Arc<AppState>, pull: Pull) -> Result<PullResult, ServerError> {
    pulling(state, Caller::Console, pull).await
}

async fn pulling(
    state: &Arc<AppState>,
    caller: Caller<'_>,
    pull: Pull,
) -> Result<PullResult, ServerError> {
    let pull = match with_cord(state, |store| match caller {
        Caller::Session(_) => store.pull(pull),
        Caller::Console => store.console_pull(pull),
    })? {
        Pulling::Answered(result) => return Ok(*result),
        Pulling::Go(pull) => pull,
    };
    let open = ask_sessions(state, &pull)?;
    // Every computer that names a runner is asked, whether or not Lys holds a
    // session open on it: a session Lys does not track, or one started as the
    // cord was pulled, is stopped with the rest.
    let mut machines: BTreeSet<String> = open.iter().map(|open| open.machine.clone()).collect();
    machines.extend(runner_machines(state)?);
    let mut asking = tokio::task::JoinSet::new();
    for machine in machines {
        let (state, pull) = (Arc::clone(state), pull.clone());
        asking.spawn(async move {
            let answer = ask_runner(&state, &machine, &pull, Asking::Pull).await;
            (machine, answer)
        });
    }
    // The runners are already being asked; each agent's credentials end now,
    // not after the slowest runner has answered.
    let agents: BTreeSet<String> = open.iter().filter_map(|open| open.agent.clone()).collect();
    let (handles_ended, handles_refused) = end_handles(state, caller, &pull, &agents).await;
    let mut ended: Vec<(String, String)> = Vec::new();
    let mut running: Vec<(String, String)> = Vec::new();
    let mut unreached: Vec<(String, String, String)> = Vec::new();
    while let Some(joined) = asking.join_next().await {
        let (machine, answer) = joined.map_err(|failed| ServerError::Runner {
            refusal: "runner_unreachable".to_owned(),
            words: format!("asking a runner to stop everything ended abnormally: {failed}"),
        })?;
        match answer {
            Ok((stopped, still)) => {
                ended.extend(
                    stopped
                        .into_iter()
                        .map(|session| (machine.clone(), session)),
                );
                running.extend(still.into_iter().map(|session| (machine.clone(), session)));
            }
            Err((refusal, reason)) => unreached.push((machine, refusal, reason)),
        }
    }
    record_stopped(state, &pull, &ended)?;
    // A session is confirmed stopped only by the runner of the computer it
    // is tracked on: another computer's runner naming it confirms nothing.
    let ended: BTreeSet<(String, String)> = ended.into_iter().collect();
    let mut still: BTreeSet<(String, String)> = open
        .iter()
        .map(|open| (open.machine.clone(), open.session.clone()))
        .filter(|held| !ended.contains(held))
        .collect();
    still.extend(running.into_iter().filter(|held| !ended.contains(held)));
    let names = Names::read(state, &open, ended.iter().chain(&still), &unreached)?;
    let result = PullResult {
        operation: pull.operation.clone(),
        stopped: ended
            .iter()
            .map(|(machine, session)| names.session(machine, session))
            .collect(),
        still_running: still
            .iter()
            .map(|(machine, session)| names.session(machine, session))
            .collect(),
        unreached: unreached
            .into_iter()
            .map(|(machine, refusal, reason)| Unreached {
                machine_name: names.machine(&machine),
                machine,
                refusal,
                reason,
            })
            .collect(),
        handles_ended,
        handles_refused,
        pulled: pull,
    };
    with_cord(state, |store| store.finish(result))
}

/// Record every tracked session that has not ended as asked to stop, in one
/// durable write, answering them.
fn ask_sessions(state: &AppState, pull: &Pull) -> Result<Vec<Open>, ServerError> {
    if state.runtime.is_none() {
        return Ok(Vec::new());
    }
    with_runtime(state, |store| {
        let open: Vec<Open> = store
            .sessions()
            .iter()
            .filter(|tracked| !tracked.stopped())
            .map(|tracked| Open {
                session: tracked.session.clone(),
                agent: tracked.agent.clone(),
                machine: tracked.machine.clone(),
            })
            .collect();
        let reports = open
            .iter()
            .map(|open| Report {
                operation: part(&pull.operation, "asked", &open.session),
                session: open.session.clone(),
                agent: open.agent.clone(),
                machine: open.machine.clone(),
                state: Reported::StopAsked,
                what: format!("everything stopped by {}: {}", pull.by, pull.reason),
                confirmation: String::new(),
                reported_by: pull.by.clone(),
                at: now(),
                launch: None,
            })
            .collect();
        store.report_all(reports)?;
        Ok(open)
    })
}

/// Who an ask of a runner is for.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Asking {
    /// A person pulled the cord: asked again while what an earlier ask
    /// stopped still runs, the runner ends it at once.
    Pull,
    /// The server is settling a start that crossed the pull in force: a
    /// session the pull already asked is left as it is, never killed for a
    /// second ask no person made.
    Settling,
}

/// Ask `machine`'s runner to stop everything it holds, answering what it
/// ended and what still runs, or the refusal and its words.
async fn ask_runner(state: &Arc<AppState>, machine: &str, pull: &Pull, asking: Asking) -> Asked {
    let named = |error: &ServerError| match error {
        ServerError::Runner { refusal, words } => (refusal.clone(), words.clone()),
        other => (other.name(), other.to_string()),
    };
    let runner = match crate::runner_sessions::machine_runner(state, machine) {
        Ok(Some(runner)) => runner,
        Ok(None) => {
            return Err((
                "runner_absent".to_owned(),
                "this computer names no runner, so Lys cannot reach what runs on it".to_owned(),
            ));
        }
        Err(error) => return Err(named(&error)),
    };
    if matches!(runner, RunnerRecord::Dialled { .. }) {
        match state.runners.hub().bridged(machine) {
            Ok(true) => {}
            Ok(false) => {
                return Err((
                    "runner_not_dialled_in".to_owned(),
                    "this computer's runner is not connected to Lys right now".to_owned(),
                ));
            }
            Err(error) => return Err(named(&ServerError::from(error))),
        }
    }
    let act = Act::StopEverything {
        by: pull.by.clone(),
        reason: pull.reason.clone(),
        kill: pull.kill,
        settling: asking == Asking::Settling,
    };
    match crate::runner_client::ask(state, machine, runner, act).await {
        Ok(Answer::StoppedEverything { sessions, running }) => Ok((sessions, running)),
        Ok(other) => Err((
            "runner_reply_malformed".to_owned(),
            format!(
                "stopping everything was answered {}",
                crate::runner_sessions::kind(&other)
            ),
        )),
        Err(error) => Err(named(&error)),
    }
}

/// Every computer in use that names a runner. A retired computer is asked
/// only when Lys still holds a session open on it.
fn runner_machines(state: &AppState) -> Result<BTreeSet<String>, ServerError> {
    if state.network.is_none() {
        return Ok(BTreeSet::new());
    }
    with_network(state, |store| {
        Ok(store
            .machines()
            .iter()
            .filter(|machine| machine.retired.is_none() && store.runner(&machine.id).is_some())
            .map(|machine| machine.id.clone())
            .collect())
    })
}

/// A start that crossed a pull: its session was tracked after the pull had
/// asked its computer, so nothing ended it. That computer is asked again
/// under the pull in force, and what it ends is recorded as that pull's
/// own. Nothing is asked when no pull is in force.
pub(crate) async fn end_late_start(
    state: &Arc<AppState>,
    machine: &str,
) -> Result<(), ServerError> {
    let Some(pull) = with_cord(state, CordStore::standing)? else {
        return Ok(());
    };
    let (stopped, _) = ask_runner(state, machine, &pull, Asking::Settling)
        .await
        .map_err(|(refusal, words)| ServerError::Runner { refusal, words })?;
    let ended: Vec<(String, String)> = stopped
        .into_iter()
        .map(|session| (machine.to_owned(), session))
        .collect();
    record_stopped(state, &pull, &ended)
}

/// Settle a session kept as starting whose start was refused because
/// everything is stopped. Its computer is asked under the pull in force: a
/// session the runner ended is recorded as that pull's; one the runner
/// still runs stays open; one the runner does not hold never ran, and is
/// kept as stopped in the runner's own words. A session not kept open is
/// left as it is and no runner is asked.
pub(crate) async fn settle_refused_start(
    state: &Arc<AppState>,
    machine: &str,
    session: &str,
    agent: &str,
) -> Result<(), ServerError> {
    if state.runtime.is_none() {
        return Ok(());
    }
    let open = with_runtime(state, |store| {
        Ok(store
            .session(session)
            .is_some_and(|tracked| !tracked.stopped()))
    })?;
    let Some(pull) = with_cord(state, CordStore::standing)? else {
        return Ok(());
    };
    if !open {
        return Ok(());
    }
    let (stopped, running) = ask_runner(state, machine, &pull, Asking::Settling)
        .await
        .map_err(|(refusal, words)| ServerError::Runner { refusal, words })?;
    let held = stopped.iter().chain(&running).any(|each| each == session);
    let ended: Vec<(String, String)> = stopped
        .into_iter()
        .map(|each| (machine.to_owned(), each))
        .collect();
    record_stopped(state, &pull, &ended)?;
    if held {
        return Ok(());
    }
    with_runtime(state, |store| {
        if store
            .session(session)
            .is_none_or(crate::runtime_state::Tracked::stopped)
        {
            return Ok(());
        }
        store
            .report(Report {
                operation: part(&pull.operation, "never-started", session),
                session: session.to_owned(),
                agent: Some(agent.to_owned()),
                machine: machine.to_owned(),
                state: Reported::Stopped,
                what: format!("everything stopped by {}: {}", pull.by, pull.reason),
                confirmation: format!(
                    "the runner of machine {machine}, asked when everything was stopped, held no such session"
                ),
                reported_by: format!("the runner of machine {machine}"),
                at: now(),
                launch: None,
            })
            .map(drop)
    })
}

/// Record every session a runner ended by this pull as stopped, in one
/// durable write, then close each run's pass and context as a confirmed end
/// closes them. A session its runner names that Lys does not hold open on
/// that computer is answered stopped and recorded nowhere.
fn record_stopped(
    state: &AppState,
    pull: &Pull,
    ended: &[(String, String)],
) -> Result<(), ServerError> {
    if state.runtime.is_none() {
        for (_, session) in ended {
            crate::agent_pass::end_session(state, session)?;
        }
        return Ok(());
    }
    let closing = with_runtime(state, |store| {
        let mut closing = Vec::new();
        let mut reports = Vec::new();
        for (machine, session) in ended {
            let Some(tracked) = store.session(session) else {
                continue;
            };
            if tracked.stopped() || &tracked.machine != machine {
                continue;
            }
            reports.push(Report {
                operation: part(&pull.operation, "stopped", session),
                session: session.clone(),
                agent: tracked.agent.clone(),
                machine: machine.clone(),
                state: Reported::Stopped,
                what: format!("everything stopped by {}: {}", pull.by, pull.reason),
                confirmation: format!(
                    "the runner of machine {machine} ended it when everything was stopped"
                ),
                reported_by: format!("the runner of machine {machine}"),
                at: now(),
                launch: None,
            });
            closing.push((session.clone(), tracked.agent.clone()));
        }
        store.report_all(reports)?;
        Ok(closing)
    })?;
    for (session, agent) in closing {
        crate::agent_pass::end_session(state, &session)?;
        if let Some(agent) = agent {
            crate::budgets_context::finish(state, &agent, &session)?;
        }
    }
    Ok(())
}

/// End every credential handle of each agent at the broker, naming each
/// agent whose handles the broker did not confirm ended.
async fn end_handles(
    state: &AppState,
    caller: Caller<'_>,
    pull: &Pull,
    agents: &BTreeSet<String>,
) -> (Vec<String>, Vec<HandlesRefused>) {
    let (mut ended, mut refused) = (Vec::new(), Vec::new());
    for agent in agents {
        let answer = match caller {
            Caller::Session(headers) => {
                crate::stop_api::end_handles(state, headers, agent, &pull.operation).await
            }
            Caller::Console => {
                crate::stop_api::end_handles_as_owner(state, agent, &pull.operation).await
            }
        };
        match answer {
            Ok(handles) => ended.extend(handles),
            Err(error) => refused.push(HandlesRefused {
                agent: agent.clone(),
                refusal: error.to_string(),
            }),
        }
    }
    (ended, refused)
}

/// The names of the agents and computers a pull's answer names, read once.
struct Names {
    agents: BTreeMap<String, Option<String>>,
    machines: BTreeMap<String, Option<String>>,
    of_session: BTreeMap<String, Option<String>>,
}

impl Names {
    fn read<'a>(
        state: &AppState,
        open: &[Open],
        sessions: impl Iterator<Item = &'a (String, String)>,
        unreached: &[(String, String, String)],
    ) -> Result<Self, ServerError> {
        let mut of_session: BTreeMap<String, Option<String>> = open
            .iter()
            .map(|open| (open.session.clone(), open.agent.clone()))
            .collect();
        let mut machines: BTreeSet<String> = unreached
            .iter()
            .map(|(machine, ..)| machine.clone())
            .collect();
        let held: Vec<(String, String)> = sessions.cloned().collect();
        if state.runtime.is_some() {
            with_runtime(state, |store| {
                for (_, session) in &held {
                    let agent = store
                        .session(session)
                        .and_then(|tracked| tracked.agent.clone());
                    of_session.entry(session.clone()).or_insert(agent);
                }
                Ok(())
            })?;
        }
        machines.extend(held.iter().map(|(machine, _)| machine.clone()));
        let agents: BTreeSet<String> = of_session.values().flatten().cloned().collect();
        let agents: BTreeMap<String, Option<String>> = with_directory(state, |directory| {
            let projection = directory.projection()?;
            Ok(agents
                .into_iter()
                .map(|agent| {
                    let name = crate::routes::identity_id(&agent)
                        .ok()
                        .and_then(|id| projection.record(id))
                        .map(|record| record.profile().display_name().to_owned());
                    (agent, name)
                })
                .collect())
        })?;
        let machines: BTreeMap<String, Option<String>> = if state.network.is_some() {
            with_network(state, |store| {
                Ok(machines
                    .into_iter()
                    .map(|machine| {
                        let name = store.machine(&machine).map(|kept| kept.name.clone());
                        (machine, name)
                    })
                    .collect())
            })?
        } else {
            machines
                .into_iter()
                .map(|machine| (machine, None))
                .collect()
        };
        Ok(Self {
            agents,
            machines,
            of_session,
        })
    }

    fn machine(&self, machine: &str) -> Option<String> {
        self.machines.get(machine).cloned().flatten()
    }

    fn session(&self, machine: &str, session: &str) -> Named {
        let agent = self.of_session.get(session).cloned().flatten();
        Named {
            session: session.to_owned(),
            agent_name: agent
                .as_ref()
                .and_then(|agent| self.agents.get(agent).cloned().flatten()),
            agent,
            machine: machine.to_owned(),
            machine_name: self.machine(machine),
        }
    }
}
