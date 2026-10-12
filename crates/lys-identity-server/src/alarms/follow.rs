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

fn server(error: AlarmError) -> ServerError {
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

/// Evaluate after usage has been kept, before the next follow request waits.
///
/// # Errors
/// Propagates runner, launch, configuration and alarm-store failures.
pub async fn tick(
    state: &Arc<AppState>,
    machine: &str,
    runner: RunnerRecord,
) -> Result<(), ServerError> {
    let tracked = crate::runtime_api::with_runtime(state, |store| {
        Ok(store
            .sessions()
            .iter()
            .filter(|session| session.machine == machine && session.agent.is_some())
            .cloned()
            .collect::<Vec<_>>())
    })?;
    let live =
        match crate::runner_client::ask(state, machine, runner, Act::Liveness { session: None })
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
    for session in tracked {
        let agent = session
            .agent
            .as_ref()
            .ok_or_else(|| server(unavailable("tracked alarm session has no agent")))?;
        let seen = live.iter().find(|seen| seen.session == session.session);
        if !session.stopped() && seen.is_none() {
            return Err(server(unavailable(format!(
                "runner liveness omits tracked session {}",
                session.session
            ))));
        }
        let first = session
            .first()
            .ok_or_else(|| server(unavailable("tracked alarm session has no start report")))?;
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
            let dir = state.proxy_dir.as_ref().ok_or_else(|| {
                server(unavailable("proxied session has no configured proxy_dir"))
            })?;
            match proxy_evidence(dir, run).map_err(server)? {
                (false, _) => ProxyEvidence::MissingFile,
                (true, false) => ProxyEvidence::AwaitingUsage,
                (true, true) => ProxyEvidence::UsageSeen,
            }
        } else {
            ProxyEvidence::Direct
        };
        observations.push(Observation {
            session: session.session.clone(),
            agent: agent.clone(),
            machine: machine.to_owned(),
            live: !session.stopped() && seen.is_some_and(|seen| seen.alive && !seen.ended),
            started_at_ms: seen.map_or(first.at.saturating_mul(1000), |seen| seen.started_at),
            last_signal_at_ms: seen.and_then(|seen| seen.last_signal_at),
            proxy,
        });
    }
    let at_ms = u64::try_from(jiff::Timestamp::now().as_millisecond())
        .map_err(|error| server(unavailable(error)))?;
    let changed = super::super::alarms_api::with_store(state, |store, threshold| {
        store.observe(&observations, at_ms, threshold)
    })
    .map_err(server)?;
    if changed {
        state.changes.signal()?;
    }
    Ok(())
}
