//! One alarm evaluation in the runner's existing usage follow pass.

use super::super::alarms_state::{AlarmError, unavailable};
use super::{Observation, ProxyEvidence};
use crate::error::ServerError;
use crate::routes::AppState;
use crate::runner_client::RunnerRecord;
use lys_runner::{Act, Answer};
use std::io::BufRead;
use std::path::Path;
use std::sync::Arc;

/// Whether the named proxy run file exists and contains a whole usage line.
///
/// # Errors
/// Refuses an invalid run key, unreadable file or malformed complete usage line.
pub fn proxy_evidence(dir: &Path, run: &str) -> Result<(bool, bool), AlarmError> {
    if !lys_home::proxy::usage::is_run_key(run) {
        return Err(AlarmError::Malformed("proxy run key is invalid".to_owned()));
    }
    let path = dir.join("usage").join(format!("{run}.jsonl"));
    let file = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok((false, false)),
        Err(error) => return Err(unavailable(error)),
    };
    if !file.metadata().map_err(unavailable)?.is_file() {
        return Err(unavailable("proxy usage path is not a file"));
    }
    let mut reader = std::io::BufReader::new(file);
    let mut line = Vec::new();
    if reader.read_until(b'\n', &mut line).map_err(unavailable)? > 0 && line.last() == Some(&b'\n')
    {
        let usage: lys_home::proxy::usage::UsageLine =
            serde_json::from_slice(&line).map_err(unavailable)?;
        if usage.run != run {
            return Err(unavailable("proxy usage line belongs to another run"));
        }
        return Ok((true, true));
    }
    Ok((true, false))
}

fn server(error: &AlarmError) -> ServerError {
    ServerError::RuntimeUnavailable {
        reason: error.to_string(),
    }
}

/// Evaluate the machine followed by the existing usage loop.
///
/// # Errors
/// Refuses a missing runner and propagates evaluation failures.
pub async fn tick_for_machine(state: &Arc<AppState>, machine: &str) -> Result<(), ServerError> {
    let runner = crate::runner_sessions::machine_runner(state, machine)?
        .ok_or(ServerError::MachineWithoutRunner)?;
    tick(state, machine, runner).await
}

pub(crate) fn now_ms() -> Result<u64, ServerError> {
    u64::try_from(jiff::Timestamp::now().as_millisecond())
        .map_err(|error| server(&unavailable(error)))
}

/// Evaluate a read usage page before the next follow request waits.
///
/// # Errors
/// Propagates runner, launch, configuration and alarm-store failures.
pub async fn tick(
    state: &Arc<AppState>,
    machine: &str,
    runner: RunnerRecord,
) -> Result<(), ServerError> {
    tick_with_clock(state, machine, runner, now_ms).await
}

pub(crate) async fn tick_with_clock(
    state: &Arc<AppState>,
    machine: &str,
    runner: RunnerRecord,
    now: impl FnOnce() -> Result<u64, ServerError> + Send,
) -> Result<(), ServerError> {
    let tracked = crate::runtime_api::with_runtime(state, |store| {
        Ok(store
            .sessions()
            .iter()
            .filter(|session| session.machine == machine && session.agent.is_some())
            .cloned()
            .collect::<Vec<_>>())
    })?;
    let live = match crate::runner_client::ask(
        state,
        machine,
        runner.clone(),
        Act::Liveness { session: None },
    )
    .await?
    {
        Answer::Liveness { sessions } => sessions,
        other => {
            return Err(ServerError::Runner {
                refusal: "runner_answer_unexpected".to_owned(),
                words: format!("alarm liveness read answered {other:?}"),
            });
        }
    };
    let mut observations = Vec::new();
    let mut thresholds = std::collections::BTreeMap::new();
    for mut session in tracked {
        if !session.stopped() && !live.iter().any(|seen| seen.session == session.session) {
            let answered = crate::runner_client::ask(
                state,
                machine,
                runner.clone(),
                Act::Status {
                    session: Some(session.session.clone()),
                },
            )
            .await;
            if matches!(&answered, Err(ServerError::Runner { refusal, .. }) if refusal == "session_unknown")
            {
                session = crate::runtime_api::with_runtime(state, |store| {
                    store
                        .session(&session.session)
                        .cloned()
                        .ok_or(ServerError::RuntimeSessionUnknown)
                })?;
            } else if let Err(error) = answered {
                (state.say)(&format!(
                    "refusals: session {} on machine {machine} could not be reconciled: {error}",
                    session.session
                ));
            }
        }
        match session_observation(state, machine, &session, &live) {
            Ok((observation, threshold)) => {
                thresholds.insert(session.session.clone(), threshold);
                observations.push(observation);
            }
            Err(error) => {
                (state.say)(&format!(
                    "refusals: session {} on machine {machine} skipped during alarm evaluation: {error}; the runner feed remains followed",
                    session.session
                ));
            }
        }
    }
    let at_ms = now()?;
    let changed = super::super::alarms_api::with_store(state, |store, _threshold| {
        store.observe_profiles(&observations, at_ms, &thresholds)
    })
    .map_err(|error| server(&error))?;
    if changed {
        state.changes.signal()?;
    }
    Ok(())
}

fn session_observation(
    state: &AppState,
    machine: &str,
    session: &crate::runtime_state::Tracked,
    live: &[lys_runner::liveness::LivenessView],
) -> Result<(Observation, u64), ServerError> {
    let agent = session
        .agent
        .as_ref()
        .ok_or_else(|| server(&unavailable("tracked alarm session has no agent")))?;
    let seen = live.iter().find(|seen| seen.session == session.session);
    if !session.stopped() && seen.is_none() {
        return Err(server(&unavailable(format!(
            "runner liveness omits tracked session {}",
            session.session
        ))));
    }
    let first = session
        .first()
        .ok_or_else(|| server(&unavailable("tracked alarm session has no start report")))?;
    let threshold = crate::provisioning_api::with_provisioning(state, |store| {
        let number = if let Some(launch) = &first.launch {
            let number = launch
                .get("provisioning_version")
                .and_then(serde_json::Value::as_u64)
                .ok_or_else(|| server(&unavailable("alarm launch has no provisioning version")))?;
            u32::try_from(number).map_err(|error| server(&unavailable(error)))?
        } else {
            store
                .profile(agent)
                .and_then(|profile| profile.versions.last())
                .ok_or(ServerError::LaunchRecordMissing)?
                .number
        };
        store.session_stall_after(agent, number, &session.session, &*state.say)
    })?;
    let run = if let Some(launch) = &first.launch {
        crate::provisioning_api::with_provisioning(state, |store| {
            Ok(crate::launch_api::kept_launch(store, launch)?
                .environment
                .get(lys_home::harness::rendering::RUN_VARIABLE)
                .cloned())
        })?
    } else {
        None
    };
    let proxy = if let Some(run) = &run {
        let dir = state
            .proxy_dir
            .as_ref()
            .ok_or_else(|| server(&unavailable("proxied session has no configured proxy_dir")))?;
        match proxy_evidence(dir, run).map_err(|error| server(&error))? {
            (false, _) => ProxyEvidence::MissingFile,
            (true, false) => ProxyEvidence::AwaitingUsage,
            (true, true) => ProxyEvidence::UsageSeen,
        }
    } else {
        ProxyEvidence::Direct
    };
    Ok((
        Observation {
            session: session.session.clone(),
            agent: agent.clone(),
            machine: machine.to_owned(),
            live: !session.stopped() && seen.is_some_and(|seen| seen.alive && !seen.ended),
            started_at_ms: seen.map_or(first.at.saturating_mul(1000), |seen| seen.started_at),
            last_signal_at_ms: seen.and_then(|seen| seen.last_signal_at),
            proxy,
        },
        threshold,
    ))
}

#[cfg(test)]
#[macro_use]
#[path = "../../tests/alarms_follow.rs"]
mod test_definitions;

#[cfg(test)]
#[macro_use]
#[path = "../../tests/kept_feed.rs"]
mod kept_test_definitions;

#[cfg(test)]
mod tests {
    alarms_follow!();
    kept_feed!();
}

#[cfg(test)]
#[path = "follow/seatfix_tests.rs"]
mod seatfix_tests;
