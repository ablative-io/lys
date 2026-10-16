//! Admitted launches retain their runner checks and their selected conversation.

use super::{AppState, Arc, Json, ServerError, Value, kept_launch, with_provisioning};
use crate::launch_template::HandleName;
use crate::network_store::NetworkStore;
use crate::provisioning_store::Version;
use crate::runtime_api::with_runtime;
use crate::runtime_state::{Report, Reported, Tracked};
use std::sync::Mutex;

#[cfg(test)]
#[path = "terminal_tests.rs"]
mod terminal_tests;

pub(crate) fn keep_start(state: &AppState, report: Report) -> Result<Tracked, ServerError> {
    let network = state
        .network
        .as_ref()
        .ok_or(ServerError::MachineWithoutRunner)?;
    let machine = report.machine.clone();
    // Session reads take runtime before network; admission must use the same order.
    with_runtime(state, |store| {
        keep_with_runner(network, &machine, || store.report(report))
    })
}

pub(super) fn keep_with_runner(
    network: &Mutex<NetworkStore>,
    machine: &str,
    keep: impl FnOnce() -> Result<Tracked, ServerError>,
) -> Result<Tracked, ServerError> {
    let mut network = network
        .lock()
        .map_err(|error| ServerError::NetworkUnavailable {
            reason: format!("the network lock is poisoned: {error}"),
        })?;
    network.settle()?;
    network
        .runner(machine)
        .ok_or(ServerError::MachineWithoutRunner)?;
    // Removal is ordered after the append, including an uncertain append result.
    let kept = keep();
    drop(network);
    kept
}

/// A replay requires the recorded runner even though its launch is already kept.
pub(crate) async fn run(
    state: &Arc<AppState>,
    view: Value,
    admitted_by: &str,
    record_seat: bool,
) -> Result<Json<Value>, ServerError> {
    let member = |name: &str| {
        view.get(name)
            .and_then(Value::as_str)
            .map(str::to_owned)
            .ok_or_else(|| ServerError::LaunchUnrenderable {
                reason: format!("the kept start names no {name}"),
            })
    };
    let (agent, machine) = (member("agent")?, member("machine")?);
    crate::runner_sessions::machine_runner(state, &machine)?
        .ok_or(ServerError::MachineWithoutRunner)?;
    let mut launch = with_provisioning(state, |store| kept_launch(store, &view))?;
    launch.policy = crate::agent_policy_api::launch_policy(state, &agent)?;
    let terminal = launch
        .config
        .as_ref()
        .is_some_and(|config| !config.requires_controls && config.harness.is_some());
    let conversation = view.get("conversation").and_then(Value::as_str);
    fresh_terminal_conversation(&mut launch, conversation)?;
    if terminal && record_seat {
        crate::seats_run::check_terminal_seat(state, &agent, &machine, &launch.session).await?;
    }
    let ran = crate::runner_sessions::run_resuming(
        state,
        (&agent, &machine, admitted_by),
        launch,
        conversation,
    )
    .await?;
    let session = member("session")?;
    let mut view = view;
    view["runner"] = ran;
    if terminal
        && let Some(live) = crate::seats_acts::live_session(state, &machine, &session).await?
    {
        if let Some(native) = &live.harness_session {
            match view.get("harness").and_then(Value::as_str) {
                Some("claude-code") => {
                    lys_runner::harness_control::Transport::Claude.validate_resume(native)?;
                }
                Some("codex") => {
                    lys_runner::harness_control::Transport::Codex.validate_resume(native)?;
                }
                _ => {}
            }
            remember_native(state, &session, native)?;
            view["native_conversation"] = serde_json::json!(native);
        }
        if live.harness.started_fresh {
            view["conversation"] = Value::Null;
            view["native_conversation"] = serde_json::json!(live.harness_session);
            view["runner"]["started_fresh"] = Value::Bool(true);
        }
    }
    if terminal && record_seat {
        crate::seats_run::remember_started(state, &view, admitted_by).await?;
    }
    Ok(Json(view))
}

pub(super) fn remember_native(
    state: &AppState,
    session: &str,
    native: &str,
) -> Result<(), ServerError> {
    with_runtime(state, |store| remember_native_in(store, session, native))
}

fn remember_native_in(
    store: &mut crate::runtime_store::RuntimeStore,
    session: &str,
    native: &str,
) -> Result<(), ServerError> {
    let tracked = store
        .session(session)
        .ok_or(ServerError::RuntimeSessionUnknown)?;
    // A stopped session takes no further reports; the next admission keeps the selected id.
    if tracked.stopped() {
        return Ok(());
    }
    if tracked.reports.iter().rev().find_map(|report| {
        report
            .launch
            .as_ref()
            .and_then(|view| view.get("native_conversation"))
            .and_then(Value::as_str)
    }) == Some(native)
    {
        return Ok(());
    }
    let mut view = tracked
        .first()
        .and_then(|first| first.launch.clone())
        .ok_or(ServerError::LaunchRecordMissing)?;
    view["native_conversation"] = serde_json::json!(native);
    let report = Report {
        operation: lys_identity::OperationId::generate()?.to_string(),
        session: session.to_owned(),
        agent: tracked.agent.clone(),
        machine: tracked.machine.clone(),
        state: tracked
            .latest()
            .ok_or(ServerError::RuntimeSessionUnknown)?
            .state,
        what: tracked
            .reports
            .iter()
            .rev()
            .find(|report| {
                report.state == Reported::Running
                    && report.what.starts_with("the runner started process ")
            })
            .ok_or(ServerError::RuntimeSessionUnknown)?
            .what
            .clone(),
        confirmation: "the runner identified the native conversation".to_owned(),
        reported_by: format!("the runner of machine {}", tracked.machine),
        at: crate::session::now(),
        launch: Some(view),
    };
    store.report(report).map(drop)
}

/// Whether the start's own arguments continue a conversation the person named
/// in the agent's settings ("Continue an existing conversation"), as
/// `--resume <id>`, `--resume=<id>` or `-r <id>`. Such a start is not a fresh
/// one: the runner proves that conversation's transcript, or starts a new one
/// and says so when the computer has none.
fn names_a_conversation(arguments: &[String]) -> bool {
    arguments.iter().any(|argument| {
        matches!(
            argument
                .split_once('=')
                .map_or(argument.as_str(), |(flag, _)| flag),
            "--resume" | "-r"
        )
    })
}

fn fresh_terminal_conversation(
    launch: &mut lys_runner::Launch,
    resume: Option<&str>,
) -> Result<(), ServerError> {
    let terminal = resume.is_none()
        && !names_a_conversation(&launch.arguments)
        && launch
            .config
            .as_ref()
            .is_some_and(|config| !config.requires_controls && config.harness.is_some());
    let arguments = if terminal
        && launch
            .config
            .as_ref()
            .is_some_and(|config| config.harness == Some(lys_runner::tracking::Harness::ClaudeCode))
    {
        let conversation = lys_runner::pty_conversation::conversation_uuid(&launch.session)?;
        Some(lys_runner::pty_conversation::claude_arguments(
            &launch.arguments,
            &conversation,
            false,
        )?)
    } else {
        None
    };
    if let Some(arguments) = arguments {
        launch.arguments = arguments;
    }
    if terminal {
        launch.environment.insert(
            "LYS_PTY_STATUS_NOTE".to_owned(),
            "no earlier conversation to resume; started a new one".to_owned(),
        );
    }
    Ok(())
}

/// A latest reviewed profile is applied only after its whole native launch ran.
pub(crate) fn enforced(
    state: &AppState,
    agent: &str,
    version: Option<&Version>,
) -> Result<bool, ServerError> {
    let Some(version) = version.filter(|version| version.reviewed.is_some()) else {
        return Ok(false);
    };
    if state.runtime.is_none() {
        return Ok(false);
    }
    with_runtime(state, |store| {
        for tracked in store
            .sessions()
            .iter()
            .filter(|tracked| tracked.agent.as_deref() == Some(agent))
        {
            let Some(latest) = tracked.latest() else {
                continue;
            };
            if latest.state != Reported::Running
                || latest.reported_by != format!("the runner of machine {}", tracked.machine)
                || !latest.what.starts_with("the runner started process ")
            {
                continue;
            }
            let Some(view) = tracked.first().and_then(|first| first.launch.as_ref()) else {
                continue;
            };
            if view.get("provisioning_version").and_then(Value::as_u64)
                != Some(u64::from(version.number))
                || !view
                    .get("left_out")
                    .and_then(Value::as_array)
                    .is_some_and(Vec::is_empty)
            {
                continue;
            }
            let template = view
                .get("template")
                .and_then(Value::as_str)
                .ok_or_else(|| ServerError::LaunchUnrenderable {
                    reason: "the running start names no template".to_owned(),
                })?;
            let digest = view
                .get("template_sha256")
                .and_then(Value::as_str)
                .ok_or_else(|| ServerError::LaunchUnrenderable {
                    reason: "the running start names no template digest".to_owned(),
                })?;
            let native = crate::launch_template::from_template(version, template, digest)?;
            let handles: Vec<HandleName> =
                serde_json::from_value(view.get("handles").cloned().ok_or_else(|| {
                    ServerError::LaunchUnrenderable {
                        reason: "the running start names no handles".to_owned(),
                    }
                })?)
                .map_err(|error| ServerError::LaunchUnrenderable {
                    reason: error.to_string(),
                })?;
            if let (Some(kept), Some(harness)) = (
                view.get("command").and_then(Value::as_str),
                version.settings.harness.as_ref(),
            ) && crate::launch_template::matches_command(
                &native,
                digest,
                &handles,
                &harness.program,
                kept,
            ) {
                return Ok(true);
            }
        }
        Ok(false)
    })
}
