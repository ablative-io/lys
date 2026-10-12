use super::{DialHub, Hook, Leave, LeaveOnDrop, Replied, RunnerRecord};
use crate::error::ServerError;
use lys_runner::{Act, Answer};
use std::sync::Arc;
use std::sync::mpsc;

/// Ask `act` of `machine`'s runner off the async workers; a caller that
/// leaves closes the request. An act about a session a seat owner holds is
/// asked of that owner's runner, on the socket the machine's runner lists
/// for it: the machine's runner does not hold an owner's session, and
/// answered every read, input, keys, operation and end on one
/// `session_unknown`.
pub async fn ask(
    state: &Arc<crate::routes::AppState>,
    machine: &str,
    record: RunnerRecord,
    act: Act,
) -> Result<Answer, ServerError> {
    let session = act.session().map(str::to_owned);
    if let Some(session) = &session
        && never_held(state, machine, session)?
    {
        return Err(ServerError::Runner {
            refusal: "session_unknown".to_owned(),
            words: "runner never held it".to_owned(),
        });
    }
    let answered = match holder_of(state, machine, &record, &act).await {
        Some(owner) => ask_exactly(state, machine, owner, act, false).await,
        None => ask_exactly(state, machine, record, act, true).await,
    };
    if let Some(session) = &session
        && matches!(&answered, Err(ServerError::Runner { refusal, .. }) if refusal == "session_unknown")
    {
        record_unknown(state, machine, session)?;
    }
    answered
}

fn never_held(
    state: &crate::routes::AppState,
    machine: &str,
    session: &str,
) -> Result<bool, ServerError> {
    if state.runtime.is_none() {
        return Ok(false);
    }
    crate::runtime_api::with_runtime(state, |store| {
        Ok(store.session(session).is_some_and(|tracked| {
            tracked.machine == machine
                && tracked.stopped()
                && tracked
                    .latest()
                    .is_some_and(|report| report.what == "session_unknown")
        }))
    })
}

fn record_unknown(
    state: &crate::routes::AppState,
    machine: &str,
    session: &str,
) -> Result<(), ServerError> {
    if state.runtime.is_none() {
        return Ok(());
    }
    let agent = crate::runtime_api::with_runtime(state, |store| {
        let Some(tracked) = store.session(session) else {
            return Ok(None);
        };
        if tracked.machine != machine || tracked.stopped() {
            return Ok(None);
        }
        let agent = tracked.agent.clone();
        store.report(crate::runtime_state::Report {
            operation: format!("runner-never-held-{session}"),
            session: session.to_owned(),
            agent: agent.clone(),
            machine: machine.to_owned(),
            state: crate::runtime_state::Reported::Stopped,
            what: "session_unknown".to_owned(),
            confirmation: "runner never held it".to_owned(),
            reported_by: format!("the runner of machine {machine}"),
            at: crate::session::now(),
            launch: None,
        })?;
        Ok(agent)
    })?;
    crate::agent_pass::end_session(state, session)?;
    if let Some(agent) = agent {
        crate::budgets_context::finish(state, &agent, session)?;
    }
    state.changes.signal()?;
    Ok(())
}

/// The runner of the seat owner holding the session `act` names, when the
/// machine's runner lists one: asked as `Act::Owned`. A dialled machine's
/// owners are not reached by a socket here, and an act naming no session,
/// a start among them, is the machine runner's own.
async fn holder_of(
    state: &Arc<crate::routes::AppState>,
    machine: &str,
    record: &RunnerRecord,
    act: &Act,
) -> Option<RunnerRecord> {
    let session = act.session()?.to_owned();
    if matches!(record, RunnerRecord::Dialled { .. }) {
        return None;
    }
    match ask_exactly(state, machine, record.clone(), Act::Owned, true).await {
        Ok(Answer::Owned { owners, .. }) => owners
            .into_iter()
            .find(|seat| seat.binding.session == session)
            .map(|seat| RunnerRecord::Socket {
                path: seat.endpoint.socket.to_string_lossy().into_owned(),
            }),
        // The machine's runner answers the act itself, and names its own
        // refusal there.
        _ => None,
    }
}

/// [`ask`] of exactly `record`'s runner; `standing` when that is the
/// machine's own runner, whose standing link an answer begins again.
async fn ask_exactly(
    state: &Arc<crate::routes::AppState>,
    machine: &str,
    record: RunnerRecord,
    act: Act,
    standing: bool,
) -> Result<Answer, ServerError> {
    let leave = Arc::new(Leave::default());
    let guard = LeaveOnDrop(Arc::clone(&leave));
    let (shared, asked_machine, asked_record) =
        (Arc::clone(state), machine.to_owned(), record.clone());
    let asked = tokio::task::spawn_blocking(move || {
        shared
            .runners
            .ask(&asked_machine, &asked_record, &act, &leave)
    })
    .await
    .map_err(|failed| ServerError::Runner {
        refusal: "runner_unreachable".to_owned(),
        words: format!("the request to the runner ended abnormally: {failed}"),
    })?;
    guard.0.done()?;
    drop(guard);
    let answer = asked?;
    // The runner answered, so it is there: a standing link that lost it is
    // begun again now.
    if standing {
        crate::runner_links::ensure(state, machine, &record);
    }
    Ok(answer)
}
pub(super) fn withdraw_on_leave(
    hub: Arc<DialHub>,
    machine: String,
    ticket: String,
    cancellation: mpsc::Sender<Replied>,
) -> Hook {
    Box::new(move || {
        if let Err(error) = hub.withdraw(&machine, &ticket) {
            tracing::error!("runner request withdrawal refused: {error}");
            if let Err(undelivered) = cancellation.send(Err(error)) {
                tracing::error!(
                    "runner withdrawal refusal could not reach its caller: {undelivered}"
                );
            }
        }
    })
}
