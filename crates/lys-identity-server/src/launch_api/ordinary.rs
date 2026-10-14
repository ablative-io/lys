//! Ordinary terminal starts resume the latest recorded launch on the selected machine.

use std::str::FromStr;
use std::sync::Arc;

use lys_identity::OperationId;
use lys_runner::RunnerError;
use lys_runner::harness_control::Transport;
use lys_runner::tracking::Harness;
use serde_json::Value;

use super::Chosen;
use crate::error::ServerError;
use crate::provisioning_api::with_provisioning;
use crate::routes::AppState;
use crate::runtime_api::with_runtime;
use crate::runtime_state::Tracked;

fn refused(reason: &str) -> ServerError {
    RunnerError::refused(
        "pty_resume_unavailable",
        format!("{reason}; choose Start again to begin a new conversation"),
    )
    .into()
}

fn transport(harness: Harness) -> Transport {
    match harness {
        Harness::ClaudeCode => Transport::Claude,
        Harness::Codex => Transport::Codex,
    }
}

fn validate_native(harness: Harness, native: &str) -> Result<(), ServerError> {
    transport(harness).validate_resume(native).map_err(|error| {
        RunnerError::refused(&error.name(), format!("{error}; choose Start again")).into()
    })
}

fn recorded_directory(view: &Value, requested: Option<&str>) -> Result<String, ServerError> {
    let directory = view
        .get("directory")
        .and_then(Value::as_str)
        .ok_or_else(|| refused("the previous start has no recorded working folder"))?;
    let directory = crate::provisioning_api::folder("directory", directory.to_owned())
        .map_err(|error| refused(&format!("the previous working folder is invalid: {error}")))?;
    if requested.is_some_and(|requested| requested != directory.as_str()) {
        return Err(refused(
            "the selected folder differs from the previous conversation's recorded working folder",
        ));
    }
    Ok(directory)
}

fn selected<'a>(view: &'a Value, field: &str) -> Result<Option<&'a str>, ServerError> {
    match view.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(native)) => Ok(Some(native)),
        Some(_) => Err(refused(
            "the previous start has an invalid native conversation field",
        )),
    }
}

fn saved_native(view: &Value, harness: Harness) -> Result<Option<String>, ServerError> {
    for field in ["native_conversation", "conversation"] {
        if let Some(native) = selected(view, field)? {
            validate_native(harness, native)?;
            return Ok(Some(native.to_owned()));
        }
    }
    Ok(None)
}

fn known_native(harness: Harness, native: Option<String>) -> Result<Option<String>, ServerError> {
    if let Some(native) = &native {
        validate_native(harness, native)?;
    }
    Ok(native)
}

fn latest<'a>(
    sessions: &'a [Tracked],
    agent: &str,
    machine: &str,
    operation: &str,
) -> Option<&'a Tracked> {
    sessions.iter().rev().find(|tracked| {
        tracked.agent.as_deref() == Some(agent)
            && tracked.machine == machine
            && tracked.session != operation
    })
}

struct Prior {
    session: String,
    directory: Option<String>,
    native: Option<String>,
}

fn prior(
    tracked: &Tracked,
    harness: Harness,
    requested: Option<&str>,
) -> Result<Prior, ServerError> {
    let view = tracked.first().and_then(|first| first.launch.as_ref());
    let native = view
        .map(|view| saved_native(view, harness))
        .transpose()?
        .flatten();
    let directory = if native.is_some() {
        let view =
            view.ok_or_else(|| refused("the previous conversation has no recorded launch"))?;
        let expected = match harness {
            Harness::ClaudeCode => "claude-code",
            Harness::Codex => "codex",
        };
        if view.get("harness").and_then(Value::as_str) != Some(expected) {
            return Err(refused(
                "the selected profile uses a different native harness from the previous session",
            ));
        }
        Some(recorded_directory(view, requested)?)
    } else {
        None
    };
    Ok(Prior {
        session: tracked.session.clone(),
        directory,
        native,
    })
}

fn replay(view: &Value, machine: &str, requested: Option<&str>) -> Result<Chosen, ServerError> {
    if view.get("machine").and_then(Value::as_str) != Some(machine) {
        return Err(refused("the retained start belongs to a different machine"));
    }
    let conversation = selected(view, "conversation")?.map(str::to_owned);
    if let Some(native) = &conversation {
        let harness = match view.get("harness").and_then(Value::as_str) {
            Some("claude-code") => Harness::ClaudeCode,
            Some("codex") => Harness::Codex,
            _ => return Err(refused("the retained start has no declared native harness")),
        };
        validate_native(harness, native)?;
    }
    Ok(Chosen {
        profile: None,
        directory: Some(recorded_directory(view, requested)?),
        conversation,
    })
}

pub(super) async fn chosen(
    state: &Arc<AppState>,
    agent: &str,
    machine: &str,
    operation: &str,
    requested_directory: Option<String>,
    fresh: bool,
) -> Result<Chosen, ServerError> {
    let operation = OperationId::from_str(operation)?.to_string();
    let requested = requested_directory
        .map(|directory| crate::provisioning_api::folder("directory", directory))
        .transpose()?;
    if let Some(view) = super::admitted(state, &operation, agent)? {
        return replay(&view, machine, requested.as_deref());
    }
    let harness = with_provisioning(state, |store| {
        let Some(version) = store.latest_reviewed(agent) else {
            let latest = store
                .profile(agent)
                .and_then(|profile| profile.versions.last())
                .ok_or(ServerError::LaunchRecordMissing)?;
            return Err(ServerError::ProfileNotReviewed {
                version: latest.number,
            });
        };
        if crate::launch_template::requires_controls(version) {
            return Ok(None);
        }
        Ok(version
            .settings
            .harness
            .as_ref()
            .and_then(crate::launch_template::runner_harness))
    })?;
    let Some(harness) = harness else {
        return Ok(Chosen {
            profile: None,
            directory: requested,
            conversation: None,
        });
    };
    if fresh {
        return Ok(Chosen {
            profile: None,
            directory: requested,
            conversation: None,
        });
    }
    let prior = with_runtime(state, |store| {
        latest(store.sessions(), agent, machine, &operation)
            .map(|tracked| prior(tracked, harness, requested.as_deref()))
            .transpose()
    })?;
    let Some(prior) = prior else {
        return Ok(Chosen {
            profile: None,
            directory: requested,
            conversation: None,
        });
    };
    let native = match prior.native {
        Some(native) => Some(native),
        None => crate::seats_acts::live_session(state, machine, &prior.session)
            .await?
            .and_then(|live| live.harness_session),
    };
    let native = known_native(harness, native)?;
    let directory = if native.is_some() {
        match prior.directory {
            Some(directory) => Some(directory),
            None => {
                let view = super::admitted(state, &prior.session, agent)?
                    .ok_or_else(|| refused("the known conversation has no recorded launch"))?;
                Some(recorded_directory(&view, requested.as_deref())?)
            }
        }
    } else {
        requested
    };
    Ok(Chosen {
        profile: None,
        directory,
        conversation: native,
    })
}

#[cfg(test)]
mod tests;
