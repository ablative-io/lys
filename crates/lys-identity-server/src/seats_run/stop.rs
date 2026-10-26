//! Native session observations and stop requests use the runner's answer.

use lys_runner::operations::{Operation, OperationRequest};
use lys_runner::{RunnerError, harness_control::Transport};

use super::{
    Act, Answer, AppState, Arc, Line, Seat, SeatError, ServerError, Started, act, kind,
    live_session, now, with_seats,
};
use crate::seats_state::Stopped;

struct EndRecord {
    lines: Vec<Line>,
    refusal: Option<ServerError>,
}

pub(super) async fn remember_end(
    state: &Arc<AppState>,
    held: &Seat,
    caller: &str,
    operation: &str,
    force: bool,
    notes: &mut Vec<String>,
) -> Result<(), ServerError> {
    let session = held
        .session
        .as_deref()
        .ok_or_else(|| SeatError::NotRunning {
            name: held.added.name.clone(),
        })?;
    let observed = if held.transport.as_deref() == Some("pty") {
        harness_session(state, &held.added.machine, session, notes).await
    } else {
        None
    };
    let refusal = with_seats(state, |store| {
        let current = store.named(&held.added.name)?;
        if current.session.as_deref() != Some(session) {
            return Err(RunnerError::refused(
                "seat_session_changed",
                "the seat selected another Lys session while its ended session was observed",
            )
            .into());
        }
        let record = end_record(
            &current,
            observed.as_deref(),
            (caller, operation, force, now()),
        )?;
        store.keep_all(record.lines)?;
        Ok(record.refusal)
    })?;
    if let Some(refusal) = refusal {
        notes.push(format!(
            "the stopped state was kept, but its native conversation was refused: {refusal}"
        ));
        return Err(refusal);
    }
    Ok(())
}

fn end_record(
    held: &Seat,
    observed: Option<&str>,
    (caller, operation, forced, at): (&str, &str, bool, u64),
) -> Result<EndRecord, ServerError> {
    let session = held.session.as_ref().ok_or_else(|| SeatError::NotRunning {
        name: held.added.name.clone(),
    })?;
    let stopped = Line::Stopped(Stopped {
        operation: operation.to_owned(),
        name: held.added.name.clone(),
        session: session.clone(),
        forced,
        by: caller.to_owned(),
        at,
    });
    let mut record = EndRecord {
        lines: vec![stopped],
        refusal: None,
    };
    if held.transport.as_deref() != Some("pty") {
        return Ok(record);
    }
    if let Some(conversation) = observed.or(held.harness_session.as_deref())
        && let Err(error) = Transport::Codex.validate_resume(conversation)
    {
        record.refusal = Some(
                RunnerError::refused(
                    &error.name(),
                    format!(
                        "the ended terminal's native conversation is not a UUID: {error}; its stopped state was kept; choose Start again"
                    ),
                )
                .into(),
            );
        return Ok(record);
    }
    if let Some(observed) = observed
        && held.harness_session.as_deref() != Some(observed)
    {
        // Started restates the same Lys session to retain its newly observed
        // native identity; Stopped closes that session in the same batch.
        record.lines.insert(
            0,
            Line::Started(Started {
                qualification: held.qualification.clone(),
                transport: "pty".to_owned(),
                operation: super::part(operation, "native-conversation"),
                name: held.added.name.clone(),
                session: session.clone(),
                harness_session: Some(observed.to_owned()),
                by: caller.to_owned(),
                at,
            }),
        );
    }
    Ok(record)
}

pub(super) async fn request_end(
    state: &Arc<AppState>,
    driven: &crate::runner_sessions::Driven,
    (name, session, caller, operation): (&str, &str, &str, &str),
    notes: &mut Vec<String>,
) -> Result<bool, ServerError> {
    let mut ended = false;
    let live = live_session(state, &driven.machine, session).await?;
    match &live {
        Some(live) if live.alive && !live.ended => {
            if live.turn_active {
                return Err(SeatError::TurnInProgress {
                    name: name.to_owned(),
                    session: session.to_owned(),
                }
                .into());
            }
            if live.harness.managed {
                let stop = Act::Operate {
                    operation: Operation {
                        operation: operation.to_owned(),
                        session: session.to_owned(),
                        request: OperationRequest::Stop,
                    },
                };
                let (answer, _) = act(state, (driven, caller, "seat_stop"), None, stop).await?;
                if let Answer::Operation { outcome } = &answer
                    && let Some(reaped) = &outcome.ended
                {
                    reaped.require_reaped(session)?;
                    ended = true;
                    notes.push("the harness's own end request ended the session".to_owned());
                } else {
                    notes.push(format!(
                        "the harness was asked to end the session ({})",
                        kind(&answer)
                    ));
                    ended = crate::seats_monitor::exited(state, driven).await?;
                }
            } else {
                let (exited, receipt) =
                    crate::seats_monitor::stop_pty(state, driven, caller, operation).await?;
                ended = exited;
                notes.extend(receipt);
            }
        }
        Some(_) | None => {
            ended = true;
            notes.push("the runner already holds the session ended".to_owned());
        }
    }
    Ok(ended)
}

pub(super) async fn harness_session(
    state: &Arc<AppState>,
    machine: &str,
    session: &str,
    notes: &mut Vec<String>,
) -> Option<String> {
    match live_session(state, machine, session).await {
        Ok(Some(live)) => {
            notes.push(format!(
                "the runner reports transport={} for session {session}",
                if live.harness.managed {
                    "managed"
                } else {
                    "pty"
                }
            ));
            if live.harness_session.is_none() {
                notes.push(format!(
                    "the runner has not yet seen the harness name its own session for {session}"
                ));
            }
            live.harness_session
        }
        Ok(None) => {
            notes.push(format!("the runner holds no session {session} to read"));
            None
        }
        Err(refused) => {
            notes.push(format!(
                "the harness's own session id for {session} is not known: {refused}"
            ));
            None
        }
    }
}

#[cfg(test)]
mod tests;
