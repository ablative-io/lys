use super::{DialHub, Hook, LeaveOnDrop, Replied, RunnerRecord};
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
///
/// # Errors
/// Returns runner, owner discovery, runtime store and credential revocation refusals.
pub async fn ask(
    state: &Arc<crate::routes::AppState>,
    machine: &str,
    record: RunnerRecord,
    act: Act,
) -> Result<Answer, ServerError> {
    let session = act.session().map(str::to_owned);
    ask_for(state, machine, record, act, session.as_deref()).await
}

pub(crate) async fn outcome(
    state: &Arc<crate::routes::AppState>,
    machine: &str,
    record: RunnerRecord,
    session: &str,
    operation: &str,
) -> Result<Answer, ServerError> {
    ask_for(
        state,
        machine,
        record,
        Act::Outcome {
            operation: operation.to_owned(),
        },
        Some(session),
    )
    .await
}

async fn ask_for(
    state: &Arc<crate::routes::AppState>,
    machine: &str,
    record: RunnerRecord,
    act: Act,
    session: Option<&str>,
) -> Result<Answer, ServerError> {
    if let Some(session) = session
        && never_held(state, machine, session)?
    {
        return Err(ServerError::Runner {
            refusal: "session_unknown".to_owned(),
            words: "runner never held it".to_owned(),
        });
    }
    let answered = match holder_of(state, machine, &record, session).await {
        Ok(Some(owner)) => ask_exactly(state, machine, owner, act, false).await,
        Ok(None) => {
            let answered = ask_exactly(state, machine, record, act, true).await;
            // Only the machine runner's own answer proves it has no owner.
            // A failed connection to an owner's socket proves nothing.
            match answered {
                Err(ServerError::Runner { refusal, .. })
                    if session.is_some() && refusal == "runner_owner_unreachable" =>
                {
                    Err(unknown())
                }
                other => other,
            }
        }
        Err(error) => Err(error),
    };
    if let Some(session) = session
        && matches!(&answered, Err(ServerError::Runner { refusal, .. }) if refusal == "session_unknown")
    {
        record_unknown(state, machine, session)?;
    }
    answered
}

fn unknown() -> ServerError {
    ServerError::Runner {
        refusal: "session_unknown".to_owned(),
        words: "runner never held it".to_owned(),
    }
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

pub(super) fn record_unknown(
    state: &crate::routes::AppState,
    machine: &str,
    session: &str,
) -> Result<(), ServerError> {
    if state.runtime.is_none() {
        return Ok(());
    }
    let (pending, agent) = state
        .runners
        .with_pending_start(machine, session, |dispatching| {
            crate::runtime_api::with_runtime(state, |store| {
                let Some(tracked) = store.session(session) else {
                    return Ok((false, None));
                };
                if tracked.machine != machine || tracked.stopped() {
                    return Ok((false, None));
                }
                // Only an active dispatch can explain absence before the runner starts.
                if dispatching
                    && tracked.latest().is_some_and(|report| {
                        report.state == crate::runtime_state::Reported::Starting
                    })
                {
                    return Ok((true, None));
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
                Ok((false, agent))
            })
        })?;
    if pending {
        return Ok(());
    }
    crate::agent_pass::end_session(state, session)?;
    if let Some(agent) = agent {
        crate::budgets_context::finish(state, &agent, session)?;
    }
    state.changes.signal()?;
    Ok(())
}

/// The runner of the seat owner holding the bound session, when the
/// machine's runner lists one: asked as `Act::Owned`. A dialled machine's
/// owners are not reached by a socket here, and an act naming no session,
/// a start among them, is the machine runner's own.
async fn holder_of(
    state: &Arc<crate::routes::AppState>,
    machine: &str,
    record: &RunnerRecord,
    session: Option<&str>,
) -> Result<Option<RunnerRecord>, ServerError> {
    let Some(session) = session else {
        return Ok(None);
    };
    if matches!(record, RunnerRecord::Dialled { .. }) {
        return Ok(None);
    }
    let answer = match ask_exactly(state, machine, record.clone(), Act::Owned, true).await {
        Err(ServerError::Runner { refusal, .. }) if refusal == "runner_owner_unreachable" => {
            return Err(unknown());
        }
        other => other?,
    };
    match answer {
        Answer::Owned {
            owners,
            unreachable,
        } => {
            if let Some(found) = unreachable
                .iter()
                .find(|found| found.seat.binding.session == session)
            {
                // A proved exit or reused pid means the recorded owner is gone;
                // an unreadable kernel still leaves its authority unknown.
                if matches!(
                    found.unreachable,
                    Some(
                        lys_runner::seat_owner::recovery::Unreachable::Exited
                            | lys_runner::seat_owner::recovery::Unreachable::PidReused { .. }
                    )
                ) {
                    return Err(unknown());
                }
                return Err(ServerError::Runner {
                    refusal: "runner_owner_unreachable".to_owned(),
                    words: format!(
                        "the recorded owner of session {session} could not be proved reachable"
                    ),
                });
            }
            Ok(owners
                .into_iter()
                .find(|seat| seat.binding.session == session)
                .map(|seat| RunnerRecord::Socket {
                    path: seat.endpoint.socket.to_string_lossy().into_owned(),
                }))
        }
        other => Err(ServerError::Runner {
            refusal: "runner_answer_unexpected".to_owned(),
            words: format!(
                "owner discovery answered {}",
                crate::runner_sessions::kind(&other)
            ),
        }),
    }
}

/// [`ask`] of exactly `record`'s runner; `standing` when that is the
/// machine's own runner, whose standing link an answer begins again.
pub(super) async fn ask_exactly(
    state: &Arc<crate::routes::AppState>,
    machine: &str,
    record: RunnerRecord,
    act: Act,
    standing: bool,
) -> Result<Answer, ServerError> {
    let encoded = serde_json::to_value(&act).map_err(|error| ServerError::Runner {
        refusal:"runner_request_malformed".to_owned(), words:error.to_string(),
    })?;
    let peer = match &record {
        RunnerRecord::Socket { path } => path.as_str(),
        RunnerRecord::Lys => "local runner socket",
        RunnerRecord::Dialled { .. } => machine,
    };
    let mut call = crate::waits::Call::begin("runner", peer,
        encoded["act"].as_str().unwrap_or("request"), act.session().unwrap_or(machine))?;
    let leave = call.closer();
    let guard = LeaveOnDrop(Arc::clone(&leave));
    let (shared, asked_machine, asked_record) =
        (Arc::clone(state), machine.to_owned(), record.clone());
    let mut waiting = tokio::task::spawn_blocking(move || {
        shared
            .runners
            .ask(&asked_machine, &asked_record, &act, &leave)
    });
    let joined = match call.until(&mut waiting).await {
        Ok(joined) => joined,
        Err(words) => {
            guard.0.leave()?;
            let outcome = waiting.await.map_err(|error| ServerError::Runner {
                refusal:"runner_unreachable".to_owned(), words:error.to_string(),
            })?;
            if let Err(error) = outcome { tracing::debug!("cancelled_runner_returned: {error}"); }
            return Err(ServerError::Runner {
                refusal: "runner_unreachable".to_owned(),
                words,
            });
        }
    };
    let asked = joined.map_err(|failed| ServerError::Runner {
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

#[cfg(test)]
#[path = "pending_start_tests.rs"]
mod pending_start_tests;

#[cfg(test)]
mod tests {
    use crate::budgets_feed::entry_tests::fixture;
    use crate::runner_client::RunnerRecord;
    use lys_runner::{Act, Answer};
    use std::error::Error;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

    #[tokio::test]
    async fn seatfix_reconcile_socket_down_never_ends_a_session() -> Result<(), Box<dyn Error>> {
        let table = fixture().await?;
        table.track("socket-down", "machine")?;
        let dir = tempfile::tempdir()?;
        let refused = super::ask(
            &table.state,
            "machine",
            RunnerRecord::Socket {
                path: dir
                    .path()
                    .join("absent.sock")
                    .to_string_lossy()
                    .into_owned(),
            },
            Act::Status {
                session: Some("socket-down".to_owned()),
            },
        )
        .await
        .err()
        .ok_or("unreachable socket answered")?;
        assert_eq!(refused.name(), "runner_unreachable");
        crate::runtime_api::with_runtime(&table.state, |store| {
            let tracked = store
                .session("socket-down")
                .ok_or(crate::error::ServerError::RuntimeSessionUnknown)?;
            assert!(!tracked.stopped());
            assert_eq!(tracked.reports.len(), 2);
            Ok(())
        })?;
        table.finish().await
    }

    #[tokio::test]
    async fn seatfix_unread_owner_evidence_never_ends_a_tracked_session()
    -> Result<(), Box<dyn Error>> {
        let table = fixture().await?;
        for session in ["registry-unread", "owner-unproved"] {
            table.track(session, "machine")?;
        }
        let root = table.state.estate_plan_file.parent().ok_or("no root")?;
        let key = lys_core::Ed25519Identity::load(&root.join("key"))?.public_key_bytes();
        let dir = tempfile::tempdir()?;
        let socket = dir.path().join("owners.sock");
        let listener = tokio::net::UnixListener::bind(&socket)?;
        let record = RunnerRecord::Socket {
            path: socket.to_string_lossy().into_owned(),
        };
        let unproved: Answer = serde_json::from_value(serde_json::json!({
            "kind":"owned", "owners":[], "unreachable":[{
                "seat":{"binding":{"seat":"seatfix","session":"owner-unproved","conversation":"","generation":1},
                    "endpoint":{"dir":"/","socket":"/missing","runner":"0123456789abcdef0123456789abcdef",
                        "owner":{"pid":1,"start":"unproved"}},"established_at":1},
                "unreachable":{"why":"unproved","reason":"kernel refused"}
            }]
        }))?;
        let registry = Arc::new(AtomicBool::new(true));
        let mode = Arc::clone(&registry);
        let (stop, mut stopped) = tokio::sync::oneshot::channel();
        let worker = tokio::spawn(async move {
            let mut statuses = 0;
            loop {
                let (stream, _) = tokio::select! { accepted = listener.accept() => accepted?, result = &mut stopped => { result.map_err(std::io::Error::other)?; return Ok::<_, std::io::Error>(statuses); } };
                let greeting =
                    lys_runner::protocol::Greeting::fresh("0123456789abcdef0123456789abcdef");
                let mut stream = BufReader::new(stream);
                stream
                    .get_mut()
                    .write_all(format!("{}\n", greeting.line()).as_bytes())
                    .await?;
                let mut line = String::new();
                stream.read_line(&mut line).await?;
                let act = lys_runner::protocol::verify_request(&line, &key, &greeting)
                    .map_err(std::io::Error::other)?;
                let answer = match act {
                    Act::Owned if mode.load(Ordering::SeqCst) => {
                        Answer::refusal(&lys_runner::RunnerError::refused(
                            "runner_unreachable",
                            "owner registry could not be read",
                        ))
                    }
                    Act::Owned => unproved.clone(),
                    Act::Status { session: Some(_) } => {
                        statuses += 1;
                        Answer::refusal(&lys_runner::RunnerError::refused(
                            "session_unknown",
                            "machine runner holds no owner session",
                        ))
                    }
                    other => return Err(std::io::Error::other(format!("unexpected {other:?}"))),
                };
                stream
                    .get_mut()
                    .write_all(format!("{}\n", lys_runner::protocol::reply_line(answer)).as_bytes())
                    .await?;
            }
        });
        let mut answers = Vec::new();
        for (session, registry_unread) in [("registry-unread", true), ("owner-unproved", false)] {
            registry.store(registry_unread, Ordering::SeqCst);
            let refused = super::ask(
                &table.state,
                "machine",
                record.clone(),
                Act::Status {
                    session: Some(session.to_owned()),
                },
            )
            .await
            .err()
            .ok_or("unread owner evidence accepted")?;
            answers.push(refused.name());
        }
        stop.send(()).map_err(|()| "worker ended")?;
        let statuses = worker.await??;
        let stopped = crate::runtime_api::with_runtime(&table.state, |store| {
            Ok(store
                .sessions()
                .iter()
                .any(crate::runtime_state::Tracked::stopped))
        })?;
        table.finish().await?;
        assert_eq!(answers, ["runner_unreachable", "runner_owner_unreachable"]);
        assert!(
            !stopped,
            "unknown owner authority was recorded as a stopped session"
        );
        assert_eq!(
            statuses, 0,
            "machine runner was asked about an owner session"
        );
        Ok(())
    }
}

#[cfg(test)]
#[path = "read_deadline_tests.rs"]
mod read_deadline_tests;
