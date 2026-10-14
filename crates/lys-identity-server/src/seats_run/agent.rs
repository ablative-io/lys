//! The team page starts a registered terminal seat through its durable seat path.

use std::str::FromStr;

use lys_identity::OperationId;
use lys_runner::RunnerError;
use lys_runner::harness_control::Transport;
use serde_json::Value;

use super::{AppState, Arc, HeaderMap, START, admitted, begin, with_seats};
use crate::error::ServerError;
use crate::error_seat::SeatError;
use crate::provisioning_api::with_provisioning;
use crate::provisioning_store::Version;
use crate::runtime_api::with_runtime;
use crate::runtime_state::Reported;
use crate::seats_state::{Line, Moved, Seat, Started, Stopped};

fn matching(seat: &Seat, agent: &str, machine: &str) -> bool {
    !seat.retired && seat.added.agent == agent && seat.added.machine == machine
}

fn select_native<'a>(
    candidates: impl IntoIterator<Item = (&'a Seat, &'a Version)>,
    agent: &str,
    machine: &str,
) -> Result<Option<&'a Seat>, ServerError> {
    let mut selected = None;
    for (seat, version) in candidates {
        if !matching(seat, agent, machine)
            || crate::launch_template::requires_controls(version)
            || version
                .settings
                .harness
                .as_ref()
                .and_then(crate::launch_template::runner_harness)
                .is_none()
        {
            continue;
        }
        if selected.is_some() {
            return Err(RunnerError::refused(
                "pty_seat_ambiguous",
                "more than one terminal seat runs this agent on this computer; start the intended seat by name",
            )
            .into());
        }
        selected = Some(seat);
    }
    Ok(selected)
}

fn terminal_seat(
    state: &AppState,
    agent: &str,
    machine: &str,
) -> Result<Option<Seat>, ServerError> {
    let seats = with_seats(state, |store| {
        Ok(store
            .seats()
            .iter()
            .filter(|seat| matching(seat, agent, machine))
            .cloned()
            .collect::<Vec<_>>())
    })?;
    with_provisioning(state, |store| {
        let candidates = seats
            .iter()
            .map(|seat| {
                store
                    .version(agent, seat.added.profile_version)
                    .map(|version| (seat, version))
                    .ok_or(ServerError::ProfileVersionUnknown {
                        version: seat.added.profile_version,
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(select_native(candidates, agent, machine)?.cloned())
    })
}

pub(crate) async fn check_terminal_seat(
    state: &Arc<AppState>,
    agent: &str,
    machine: &str,
    session: &str,
) -> Result<(), ServerError> {
    if let Some(held) = terminal_seat(state, agent, machine)? {
        previous_ended(state, &held, session).await?;
    }
    Ok(())
}

async fn previous_ended(
    state: &Arc<AppState>,
    held: &Seat,
    new_session: &str,
) -> Result<(), ServerError> {
    if !held.running || held.session.as_deref() == Some(new_session) {
        return Ok(());
    }
    let session = held
        .session
        .as_deref()
        .ok_or_else(|| ServerError::LaunchUnrenderable {
            reason: "the running terminal seat names no session".to_owned(),
        })?;
    let stopped = with_runtime(state, |store| {
        Ok(store.session(session).is_some_and(|tracked| {
            tracked.agent.as_deref() == Some(held.added.agent.as_str())
                && tracked.machine == held.added.machine
                && tracked
                    .latest()
                    .is_some_and(|report| report.state == Reported::Stopped)
        }))
    })?;
    if stopped {
        return require_ended(&held.added.name, session, true, None);
    }
    let live = super::live_session(state, &held.added.machine, session).await?;
    require_ended(
        &held.added.name,
        session,
        false,
        live.map(|live| (live.alive, live.ended)),
    )
}

fn require_ended(
    name: &str,
    session: &str,
    stopped: bool,
    observed: Option<(bool, bool)>,
) -> Result<(), ServerError> {
    match (stopped, observed) {
        (true, _) | (false, Some((false, true))) => Ok(()),
        (false, Some((true, false))) => Err(SeatError::Running {
            name: name.to_owned(),
            session: session.to_owned(),
        }
        .into()),
        (false, Some((true, true))) => Err(RunnerError::refused(
            "runner_reply_malformed",
            "the previous terminal session was reported both alive and ended",
        )
        .into()),
        (false, None | Some((false, false))) => Err(RunnerError::refused(
            "pty_previous_session_unconfirmed",
            format!("the previous session {session} has no confirmed end; read or stop it before starting again"),
        )
        .into()),
    }
}

pub(crate) async fn start_from_agent(
    state: &Arc<AppState>,
    headers: &HeaderMap,
    agent: &str,
    machine: &str,
    operation: &str,
    directory: Option<&str>,
) -> Result<Option<Value>, ServerError> {
    let Some(held) = terminal_seat(state, agent, machine)? else {
        return Ok(None);
    };
    let (admission, standing) = admitted(
        state,
        headers,
        &held,
        START,
        &format!("/agents/{agent}/start-command"),
    )?;
    let begun = Box::pin(begin(
        state,
        headers,
        (&held, &standing),
        &admission.caller,
        operation,
        (None, false, directory.map(str::to_owned)),
    ))
    .await?;
    Ok(Some(begun.start))
}

pub(crate) async fn remember_started(
    state: &Arc<AppState>,
    view: &Value,
    caller: &str,
) -> Result<(), ServerError> {
    let (agent, machine, session) = (
        text(view, "agent")?,
        text(view, "machine")?,
        text(view, "session")?,
    );
    let Some(held) = terminal_seat(state, agent, machine)? else {
        return Ok(());
    };
    let number = profile_number(view)?;
    let version = with_provisioning(state, |store| {
        store
            .version(agent, number)
            .cloned()
            .ok_or(ServerError::ProfileVersionUnknown { version: number })
    })?;
    crate::start_checks::reviewed(&version)?;
    let transport = transport(&version, view)?;
    previous_ended(state, &held, session).await?;
    let observed = super::live_session(state, machine, session)
        .await?
        .and_then(|live| live.harness_session);
    let native = native_conversation(view, transport, observed.as_deref())?;
    with_seats(state, |store| {
        let current = store.named(&held.added.name)?;
        if let Some(recorded) = store.recorded(session) {
            return match recorded {
                Line::Started(started) if started.name == current.added.name => Ok(()),
                _ => Err(SeatError::OperationReused {
                    operation: session.to_owned(),
                }
                .into()),
            };
        }
        if current.added.operation != held.added.operation || current.revision != held.revision {
            return Err(RunnerError::refused(
                "pty_seat_changed",
                "the seat changed while its runner was starting; read the current seat before starting again",
            )
            .into());
        }
        store.keep_all(transition(
            &current,
            &version,
            view,
            native,
            caller,
            crate::session::now(),
        )?)?;
        Ok(())
    })
}

fn text<'a>(view: &'a Value, member: &str) -> Result<&'a str, ServerError> {
    view.get(member)
        .and_then(Value::as_str)
        .ok_or_else(|| ServerError::LaunchUnrenderable {
            reason: format!("the terminal start names no {member}"),
        })
}

fn profile_number(view: &Value) -> Result<u32, ServerError> {
    view.get("provisioning_version")
        .and_then(Value::as_u64)
        .and_then(|number| u32::try_from(number).ok())
        .ok_or_else(|| ServerError::LaunchUnrenderable {
            reason: "the terminal start names no valid profile version".to_owned(),
        })
}

fn transport(version: &Version, view: &Value) -> Result<Transport, ServerError> {
    let harness = version
        .settings
        .harness
        .as_ref()
        .and_then(crate::launch_template::runner_harness);
    let (name, transport) = match harness {
        Some(lys_runner::tracking::Harness::ClaudeCode) => ("claude-code", Transport::Claude),
        Some(lys_runner::tracking::Harness::Codex) => ("codex", Transport::Codex),
        None => {
            return Err(ServerError::LaunchUnrenderable {
                reason: "the terminal start's profile declares no native harness".to_owned(),
            });
        }
    };
    if crate::launch_template::requires_controls(version)
        || text(view, "harness")? != name
        || profile_number(view)? != version.number
    {
        return Err(ServerError::LaunchUnrenderable {
            reason: "the terminal start does not match its exact reviewed profile".to_owned(),
        });
    }
    Ok(transport)
}

fn selected_conversation<'a>(
    view: &'a Value,
    member: &str,
) -> Result<Option<&'a str>, ServerError> {
    match view.get(member) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(native)) => Ok(Some(native)),
        Some(_) => Err(RunnerError::refused(
            "pty_resume_unavailable",
            format!("the kept {member} is invalid; choose Start again"),
        )
        .into()),
    }
}

fn native_conversation(
    view: &Value,
    transport: Transport,
    observed: Option<&str>,
) -> Result<Option<String>, ServerError> {
    let retained = selected_conversation(view, "native_conversation")?;
    let resume = selected_conversation(view, "conversation")?;
    for native in [retained, resume, observed].into_iter().flatten() {
        transport.validate_resume(native).map_err(|error| {
            RunnerError::refused(&error.name(), format!("{error}; choose Start again"))
        })?;
    }
    if retained
        .zip(resume)
        .is_some_and(|(retained, resume)| retained != resume)
    {
        return Err(RunnerError::refused(
            "pty_conversation_mismatch",
            "the kept native and resumed conversations differ; choose Start again",
        )
        .into());
    }
    let expected = retained.or(resume);
    if expected
        .zip(observed)
        .is_some_and(|(expected, observed)| expected != observed)
    {
        return Err(RunnerError::refused(
            "pty_conversation_mismatch",
            "the runner reported another native conversation; choose Start again",
        )
        .into());
    }
    Ok(expected.or(observed).map(str::to_owned))
}

fn transition(
    held: &Seat,
    version: &Version,
    view: &Value,
    native: Option<String>,
    caller: &str,
    at: u64,
) -> Result<Vec<Line>, ServerError> {
    let session = text(view, "session")?;
    OperationId::from_str(session)?;
    transport(version, view)?;
    if text(view, "agent")? != held.added.agent || text(view, "machine")? != held.added.machine {
        return Err(ServerError::LaunchUnrenderable {
            reason: "the terminal start belongs to another seat's agent or computer".to_owned(),
        });
    }
    crate::provisioning_api::folder("directory", text(view, "directory")?.to_owned())?;
    let declared = version
        .settings
        .harness
        .as_ref()
        .ok_or(ServerError::HarnessUndeclared {
            version: version.number,
        })?;
    let mut lines = Vec::new();
    if held.running && held.session.as_deref() != Some(session) {
        let previous = held
            .session
            .clone()
            .ok_or_else(|| ServerError::LaunchUnrenderable {
                reason: "the running terminal seat names no session".to_owned(),
            })?;
        lines.push(Line::Stopped(Stopped {
            operation: super::part(session, "terminal-stop"),
            name: held.added.name.clone(),
            session: previous,
            forced: true,
            by: caller.to_owned(),
            at,
        }));
    }
    if held.added.profile_version != version.number || held.added.harness != declared.name {
        lines.push(Line::Moved(Moved {
            operation: super::part(session, "terminal-move"),
            name: held.added.name.clone(),
            profile_version: version.number,
            harness: declared.name.clone(),
            by: caller.to_owned(),
            at,
        }));
    }
    lines.push(Line::Started(Started {
        qualification: selected_conversation(&view["runner"], "qualification")?.map(str::to_owned),
        transport: "pty".to_owned(),
        operation: session.to_owned(),
        name: held.added.name.clone(),
        session: session.to_owned(),
        harness_session: native,
        by: caller.to_owned(),
        at,
    }));
    Ok(lines)
}

#[cfg(test)]
mod tests;
