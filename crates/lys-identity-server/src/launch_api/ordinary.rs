//! Ordinary terminal starts resume the latest recorded launch on the selected machine.

use std::str::FromStr;
use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use lys_identity::{AgentId, OperationId};
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

#[derive(serde::Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct PreviewQuery {
    machine: String,
}

#[derive(serde::Serialize, utoipa::ToSchema)]
pub(crate) struct PreviewView {
    machine: String,
    directory: Option<String>,
}

/// The last successful conversation's folder, before a person presses Start.
pub(super) async fn preview(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(agent): Path<String>,
    given: Result<Query<PreviewQuery>, axum::extract::rejection::QueryRejection>,
) -> Result<Json<PreviewView>, ServerError> {
    let actor = crate::routes::signed_in(&state, &headers)?;
    let Query(query) = given.map_err(|error| ServerError::RequestMalformed {
        reason: error.body_text(),
    })?;
    let agent = AgentId::from_str(&agent)?.to_string();
    super::start_caller(&state, &headers, &actor, &agent)?;
    OperationId::from_str(&query.machine)?;
    let directory = with_runtime(&state, |store| {
        latest(store.sessions(), &agent, &query.machine, "")
            .and_then(|tracked| tracked.first())
            .and_then(|first| first.launch.as_ref())
            .filter(|view| view.get("directory").is_some())
            .map(|view| recorded_directory(view, None))
            .transpose()
    })?;
    Ok(Json(PreviewView {
        machine: query.machine,
        directory,
    }))
}

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
            && started(tracked)
    })
}

fn started(tracked: &Tracked) -> bool {
    tracked.reports.iter().any(|report| {
        report.state == crate::runtime_state::Reported::Running
            && report.reported_by == format!("the runner of machine {}", tracked.machine)
            && report.what.starts_with("the runner started process ")
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
    if let (Some(requested), Some(view)) = (requested, view)
        && view
            .get("directory")
            .and_then(Value::as_str)
            .is_some_and(|old| old != requested)
    {
        return Ok(Prior {
            session: tracked.session.clone(),
            directory: Some(recorded_directory(view, None)?),
            native: None,
        });
    }
    let native = if started(tracked) && view.is_some() {
        let observed = tracked.reports.iter().rev().find(|report| {
            report.state != crate::runtime_state::Reported::Starting
                && report.reported_by == format!("the runner of machine {}", tracked.machine)
                && report.confirmation == "the runner identified the native conversation"
        });
        observed
            .and_then(|report| report.launch.as_ref())
            .or(view)
            .map(|view| saved_native(view, harness))
            .transpose()?
            .flatten()
    } else {
        None
    };
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
        Some(recorded_directory(view, None)?)
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

pub(crate) async fn chosen(
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
    let recorded = if prior.directory.is_some() {
        prior.directory.clone()
    } else {
        with_runtime(state, |store| {
            Ok(store
                .session(&prior.session)
                .and_then(|tracked| tracked.first())
                .and_then(|first| first.launch.as_ref())
                .and_then(|view| view.get("directory"))
                .and_then(Value::as_str)
                .map(str::to_owned))
        })?
    };
    if requested
        .as_deref()
        .zip(recorded.as_deref())
        .is_some_and(|(new, old)| new != old)
    {
        return Ok(Chosen {
            profile: None,
            directory: requested,
            conversation: None,
        });
    }
    let observed = crate::seats_acts::live_session(state, machine, &prior.session)
        .await?
        .and_then(|live| live.harness_session);
    if let Some(native) = &observed {
        validate_native(harness, native)?;
        super::runner::remember_native(state, &prior.session, native)?;
    }
    let native = observed.or(prior.native);
    let native = known_native(harness, native)?;
    let directory = if native.is_some() {
        let view = super::admitted(state, &prior.session, agent)?
            .ok_or_else(|| refused("the known conversation has no recorded launch"))?;
        Some(recorded_directory(&view, requested.as_deref())?)
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
