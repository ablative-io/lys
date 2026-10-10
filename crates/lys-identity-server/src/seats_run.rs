//! A seat's start, stop and restart (AGENTS-002 R1, R3, R5), each a
//! deliberate act admitted as `seats_acts.rs` says; its messages and
//! attach are `seats_send.rs`'s.
//!
//! A start goes through the server's own start path, the one the start
//! command takes, from the seat's exact profile version, and only when that
//! version's session requires controls, so the runner starts it managed. A
//! seat whose agent has no responsible person is not started, nor one whose
//! latest import is in progress or stopped (`import_incomplete`, refused
//! before anything is stopped or started), nor one the monitor still lists
//! online. A stop is refused while the harness reports a turn
//! in progress unless force is named; between turns a managed session is
//! asked to end through the harness's own end request, and the runner then
//! ends its process. A restart is a stop and then a start, under a new
//! session.

use std::str::FromStr;
use std::sync::Arc;

use axum::Json;
use axum::extract::rejection::JsonRejection;
use axum::extract::{OriginalUri, Path, State};
use axum::http::HeaderMap;
use lys_identity::{AgentId, OperationId};
use lys_runner::operations::{Operation, OperationRequest};
use lys_runner::{Act, Answer};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::error::ServerError;
use crate::error_seat::SeatError;
use crate::launch_api::{Chosen, Who, start_as};
use crate::routes::{AppState, hex};
use crate::runner_sessions::{driven, ended_in, kind};
use crate::seats_acts::{RESTART, START, STOP, Standing, act, admitted, live_session};
use crate::seats_api::{body, seat, shown, with_seats};
use crate::seats_state::{Line, Seat, Started, Stopped};
use crate::seats_views::{
    SeatOperationBody, SeatRestarted, SeatStarted, SeatStopBody, SeatStopped,
};
use crate::session::now;

/// A seat's session started, as the start answers it.
struct Begun {
    session: String,
    harness_session: Option<String>,
    start: Value,
    notes: Vec<String>,
}

/// A seat's session stopped, as the stop answers it; `ended` is the wire
/// field's name.
#[allow(
    clippy::struct_field_names,
    reason = "the field is the answer's own name on the wire"
)]
struct Ended {
    session: String,
    ended: bool,
    notes: Vec<String>,
}

pub(crate) async fn start(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    OriginalUri(uri): OriginalUri,
    Path(name): Path<String>,
    given: Result<Json<SeatOperationBody>, JsonRejection>,
) -> Result<Json<SeatStarted>, ServerError> {
    let given = body(given)?;
    OperationId::from_str(&given.operation)?;
    let held = seat(&state, &name)?;
    let (admission, standing) = admitted(&state, &headers, &held, START, &uri.to_string())?;
    let begun = Box::pin(begin(
        &state,
        &headers,
        (&held, &standing),
        &admission.caller,
        &given.operation,
    ))
    .await?;
    let held = seat(&state, &name)?;
    Ok(Json(SeatStarted {
        seat: shown(&state, &held).await?,
        session: begun.session,
        harness_session: begun.harness_session,
        start: begun.start,
        admission,
        notes: begun.notes,
    }))
}

pub(crate) async fn stop(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    OriginalUri(uri): OriginalUri,
    Path(name): Path<String>,
    given: Result<Json<SeatStopBody>, JsonRejection>,
) -> Result<Json<SeatStopped>, ServerError> {
    let given = body(given)?;
    OperationId::from_str(&given.operation)?;
    let held = seat(&state, &name)?;
    let (admission, _) = admitted(&state, &headers, &held, STOP, &uri.to_string())?;
    let ended = Box::pin(end(
        &state,
        &held,
        &admission.caller,
        &given.operation,
        given.force,
    ))
    .await?;
    let held = seat(&state, &name)?;
    Ok(Json(SeatStopped {
        seat: shown(&state, &held).await?,
        session: ended.session,
        ended: ended.ended,
        admission,
        notes: ended.notes,
    }))
}

pub(crate) async fn restart(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    OriginalUri(uri): OriginalUri,
    Path(name): Path<String>,
    given: Result<Json<SeatStopBody>, JsonRejection>,
) -> Result<Json<SeatRestarted>, ServerError> {
    let given = body(given)?;
    OperationId::from_str(&given.operation)?;
    let held = seat(&state, &name)?;
    let (admission, standing) = admitted(&state, &headers, &held, RESTART, &uri.to_string())?;
    state.seat_imports.require_selectable(&held.added.name)?;
    let mut notes = Vec::new();
    let stopped = if held.running {
        let ended = Box::pin(end(
            &state,
            &held,
            &admission.caller,
            &part(&given.operation, "stop"),
            given.force,
        ))
        .await?;
        notes.extend(ended.notes);
        Some(ended.session)
    } else {
        None
    };
    let held = seat(&state, &name)?;
    let begun = Box::pin(begin(
        &state,
        &headers,
        (&held, &standing),
        &admission.caller,
        &part(&given.operation, "start"),
    ))
    .await?;
    notes.extend(begun.notes);
    let held = seat(&state, &name)?;
    Ok(Json(SeatRestarted {
        seat: shown(&state, &held).await?,
        session: begun.session,
        stopped,
        admission,
        notes,
    }))
}

/// An operation id for one part of the act asked under `operation`, the
/// same each time, so a restart sent again finds its parts already done.
fn part(operation: &str, part: &str) -> String {
    let digest = Sha256::digest(format!("lys-identity/seat-act/v1\n{operation}\n{part}"));
    format!("op-{}", &hex(&digest)[..32])
}

/// Start a session for `seat` under `operation`, for `caller`.
async fn begin(
    state: &Arc<AppState>,
    headers: &HeaderMap,
    (held, standing): (&Seat, &Standing),
    caller: &str,
    operation: &str,
) -> Result<Begun, ServerError> {
    let name = held.added.name.clone();
    if standing.responsible.is_none() {
        return Err(SeatError::NoResponsible {
            name,
            agent: held.added.agent.clone(),
        }
        .into());
    }
    state.seat_imports.require_selectable(&name)?;
    if let (true, Some(session)) = (held.running, &held.session)
        && session != operation
        && live_session(state, &held.added.machine, session)
            .await?
            .is_some_and(|live| live.alive && !live.ended)
    {
        return Err(SeatError::Running {
            name,
            session: session.clone(),
        }
        .into());
    }
    let number = held.added.profile_version;
    let version = crate::provisioning_api::with_provisioning(state, |store| {
        store
            .version(&held.added.agent, number)
            .cloned()
            .ok_or(ServerError::ProfileVersionUnknown { version: number })
    })?;
    if !crate::launch_template::requires_controls(&version) {
        return Err(SeatError::NotManaged {
            agent: held.added.agent.clone(),
            version: number,
        }
        .into());
    }
    let mut notes = vec![crate::seats_monitor::check(&name).await?];
    let chosen = Chosen {
        profile: Some(version),
        directory: held.added.working_folder.clone(),
    };
    let Json(start) = Box::pin(start_as(
        state,
        headers,
        Who::Admitted(caller.to_owned()),
        AgentId::from_str(&held.added.agent)?,
        (&held.added.machine, operation),
        chosen,
    ))
    .await?;
    let session = start
        .get("session")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| ServerError::LaunchUnrenderable {
            reason: "the kept start names no session".to_owned(),
        })?;
    let harness_session = match live_session(state, &held.added.machine, &session).await {
        Ok(Some(live)) if live.harness_session.is_some() => live.harness_session,
        Ok(Some(_)) => {
            notes.push(format!(
                "the runner has not yet seen the harness name its own session for {session}"
            ));
            None
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
    };
    let kept = with_seats(state, |store| {
        store.keep(Line::Started(Started {
            operation: operation.to_owned(),
            name: name.clone(),
            session: session.clone(),
            harness_session: harness_session.clone(),
            by: caller.to_owned(),
            at: now(),
        }))
    })?;
    let harness_session = match kept {
        Line::Started(kept) => kept.harness_session.or(harness_session),
        Line::Added(_) | Line::Stopped(_) | Line::Sent(_) => harness_session,
    };
    Ok(Begun {
        session,
        harness_session,
        start,
        notes,
    })
}

/// Stop `held`'s running session under `operation`, for `caller`.
async fn end(
    state: &Arc<AppState>,
    held: &Seat,
    caller: &str,
    operation: &str,
    force: bool,
) -> Result<Ended, ServerError> {
    let name = held.added.name.clone();
    let recorded = with_seats(state, |store| Ok(store.recorded(operation).cloned()))?;
    if let Some(Line::Stopped(kept)) = recorded
        && kept.name == name
    {
        return Ok(Ended {
            session: kept.session,
            ended: true,
            notes: vec![format!("stopped already under operation {operation}")],
        });
    }
    let running = if held.running {
        held.session.clone()
    } else {
        None
    };
    let session = running.ok_or_else(|| SeatError::NotRunning { name: name.clone() })?;
    let driven = driven(state, &session)?;
    let live = live_session(state, &driven.machine, &session).await?;
    let mut notes = Vec::new();
    let mut ended = false;
    match &live {
        Some(live) if live.alive && !live.ended => {
            if live.turn_active && !force {
                return Err(SeatError::TurnInProgress { name, session }.into());
            }
            if live.managed && !live.turn_active {
                let stop = Act::Operate {
                    operation: Operation {
                        operation: operation.to_owned(),
                        session: session.clone(),
                        request: OperationRequest::Stop,
                    },
                };
                let (answer, _) = act(state, (&driven, caller, "seat_stop"), None, stop).await?;
                if let Answer::Operation { outcome } = &answer
                    && outcome.ended.is_some()
                {
                    ended = true;
                    notes.push("the harness's own end request ended the session".to_owned());
                } else {
                    notes.push(format!(
                        "the harness was asked to end the session ({})",
                        kind(&answer)
                    ));
                }
            } else if live.turn_active {
                notes.push("force was named: the session was ended mid-turn".to_owned());
            }
        }
        Some(_) | None => {
            ended = true;
            notes.push("the runner already holds the session ended".to_owned());
        }
    }
    if !ended {
        let finish = Act::End {
            session: session.clone(),
        };
        match act(state, (&driven, caller, "end"), None, finish).await {
            Ok((answer, _)) => {
                ended = ended_in(&answer).is_some();
                notes.push(format!("the runner ended the process ({})", kind(&answer)));
            }
            Err(ServerError::Runner { refusal, .. }) if refusal == "session_ended" => {
                ended = true;
                notes.push("the session had ended before the runner was asked".to_owned());
            }
            Err(other) => return Err(other),
        }
    }
    with_seats(state, |store| {
        store.keep(Line::Stopped(Stopped {
            operation: operation.to_owned(),
            name,
            session: session.clone(),
            forced: force,
            by: caller.to_owned(),
            at: now(),
        }))
    })?;
    Ok(Ended {
        session,
        ended,
        notes,
    })
}

#[cfg(test)]
mod tests {
    use super::part;

    #[test]
    fn a_restart_names_its_parts_the_same_each_time() {
        assert_eq!(part("op-a", "stop"), part("op-a", "stop"));
        assert_ne!(part("op-a", "stop"), part("op-a", "start"));
        assert_eq!(part("op-a", "start").len(), 35);
    }
}
