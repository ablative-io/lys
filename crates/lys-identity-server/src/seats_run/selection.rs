//! A terminal seat reuses its native identity and the folder recorded at start.

use lys_runner::RunnerError;
use lys_runner::tracking::Harness;
use serde_json::Value;

use super::{AppState, Arc, Seat, ServerError, conversation, live_session, with_seats};
use crate::provisioning_store::Version;

pub(super) struct Selection {
    pub(super) resume: Option<String>,
    pub(super) expected: Option<String>,
    pub(super) directory: Option<String>,
}

fn refused(reason: &str) -> ServerError {
    RunnerError::refused(
        "pty_resume_unavailable",
        format!("{reason}; choose Start again to begin a new conversation"),
    )
    .into()
}

fn directory(view: &Value) -> Result<String, ServerError> {
    view.get("directory")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| refused("the previous start has no recorded working folder"))
}

fn resumed(view: &Value) -> Result<Option<String>, ServerError> {
    match view.get("conversation") {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(conversation)) => Ok(Some(conversation.clone())),
        Some(_) => Err(refused("the kept start has an invalid native conversation")),
    }
}

fn native(version: &Version) -> Option<Harness> {
    version
        .settings
        .harness
        .as_ref()
        .and_then(crate::launch_template::runner_harness)
}

pub(super) async fn choose(
    state: &Arc<AppState>,
    held: &Seat,
    version: &Version,
    operation: &str,
    explicit: Option<&str>,
    fresh: bool,
    requested_directory: Option<String>,
) -> Result<Selection, ServerError> {
    let managed = crate::launch_template::requires_controls(version);
    let harness = native(version);
    if let Some(view) = crate::launch_api::admitted(state, operation, &held.added.agent)? {
        let resume = resumed(&view)?;
        let recorded_directory = directory(&view)?;
        if requested_directory
            .as_ref()
            .is_some_and(|given| given != &recorded_directory)
        {
            return Err(ServerError::RuntimeReportReused {
                operation: operation.to_owned(),
            });
        }
        if explicit.is_some() && explicit != resume.as_deref() {
            return Err(ServerError::RuntimeReportReused {
                operation: operation.to_owned(),
            });
        }
        let number = view
            .get("provisioning_version")
            .and_then(Value::as_u64)
            .and_then(|number| u32::try_from(number).ok())
            .ok_or_else(|| refused("the kept start has no valid profile version"))?;
        let recorded = crate::provisioning_api::with_provisioning(state, |store| {
            store
                .version(&held.added.agent, number)
                .cloned()
                .ok_or(ServerError::ProfileVersionUnknown { version: number })
        })?;
        let managed = crate::launch_template::requires_controls(&recorded);
        let harness = native(&recorded);
        let expected = if managed {
            None
        } else {
            match harness {
                Some(Harness::ClaudeCode) => Some(match &resume {
                    Some(conversation) => conversation.clone(),
                    None => lys_runner::pty_conversation::conversation_uuid(operation)?,
                }),
                Some(Harness::Codex) => resume.clone().or_else(|| {
                    (held.session.as_deref() == Some(operation))
                        .then(|| held.harness_session.clone())
                        .flatten()
                }),
                None => None,
            }
        };
        return Ok(Selection {
            resume,
            expected,
            directory: Some(recorded_directory),
        });
    }
    if managed {
        return Ok(Selection {
            resume: explicit.map(str::to_owned),
            expected: None,
            directory: requested_directory.or_else(|| held.added.working_folder.clone()),
        });
    }
    let mut remembered = held.clone();
    remembered.harness_session = with_seats(state, |store| {
        store.remembered_conversation(&held.added.name)
    })?;
    if !fresh
        && explicit.is_none()
        && remembered.harness_session.is_none()
        && let Some(session) = &held.session
        && let Some(live) = live_session(state, &held.added.machine, session).await?
    {
        remembered.harness_session = live.harness_session;
    }
    let plan = conversation::pty(&remembered, operation, explicit, fresh, harness)?;
    let folder = if plan.resume.is_some() {
        let recorded = resume_directory(state, held, harness)?;
        if requested_directory
            .as_ref()
            .is_some_and(|given| recorded.as_ref() != Some(given))
        {
            return Err(refused(
                "the requested working folder differs from the previous conversation",
            ));
        }
        recorded
    } else {
        requested_directory.or_else(|| held.added.working_folder.clone())
    };
    Ok(Selection {
        resume: plan.resume,
        expected: plan.expected,
        directory: folder,
    })
}

fn resume_directory(
    state: &AppState,
    held: &Seat,
    harness: Option<Harness>,
) -> Result<Option<String>, ServerError> {
    let Some(session) = &held.session else {
        return Ok(held.added.working_folder.clone());
    };
    if let Some(view) = crate::launch_api::admitted(state, session, &held.added.agent)? {
        return Ok(Some(recorded_folder(&view, harness)?));
    }
    held.added
        .working_folder
        .clone()
        .map(Some)
        .ok_or_else(|| refused("the previous seat has no recorded working folder"))
}

fn recorded_folder(view: &Value, harness: Option<Harness>) -> Result<String, ServerError> {
    let expected = match harness {
        Some(Harness::ClaudeCode) => "claude-code",
        Some(Harness::Codex) => "codex",
        None => return Err(refused("the selected profile has no native harness")),
    };
    if view.get("harness").and_then(Value::as_str) != Some(expected) {
        return Err(refused(
            "the selected profile uses a different native harness",
        ));
    }
    directory(view)
}

#[cfg(test)]
mod tests;
