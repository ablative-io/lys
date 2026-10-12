//! Compaction uses the immutable profile version kept on the session's start.
use crate::error::ServerError;
use crate::provisioning_store::SessionSettings;
use crate::routes::AppState;

pub(crate) fn start_settings(
    state: &AppState,
    agent: &str,
    session: &str,
) -> Result<(u32, Option<SessionSettings>), ServerError> {
    let number = crate::runtime_api::with_runtime(state, |store| {
        let tracked = store
            .session(session)
            .ok_or(ServerError::RuntimeSessionUnknown)?;
        if tracked.agent.as_deref() != Some(agent) {
            return Err(ServerError::RuntimeSessionUnknown);
        }
        let number = tracked
            .reports
            .iter()
            .find_map(|report| report.launch.as_ref())
            .and_then(|launch| launch.get("provisioning_version"))
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| ServerError::Runner {
                refusal: "compact_start_version_missing".to_owned(),
                words: format!("session {session} has no profile version on its kept start"),
            })?;
        u32::try_from(number).map_err(|error| ServerError::Runner {
            refusal: "compact_start_version_invalid".to_owned(),
            words: error.to_string(),
        })
    })?;
    crate::provisioning_api::with_provisioning(state, |store| {
        let version = store
            .version(agent, number)
            .ok_or(ServerError::ProfileVersionUnknown { version: number })?;
        Ok((number, version.settings.session.clone()))
    })
}

pub(crate) fn compact_command(
    state: &AppState,
    agent: &str,
    session: &str,
) -> Result<(String, u32, bool), ServerError> {
    let (version, settings) = start_settings(state, agent, session)?;
    let requires_controls = settings
        .as_ref()
        .is_some_and(|settings| settings.requires_controls);
    let command = settings
        .and_then(|settings| settings.compact)
        .filter(|text| !text.trim().is_empty())
        .unwrap_or_else(|| "/compact".to_owned());
    Ok((command, version, requires_controls))
}

pub(crate) async fn prepare_compact(
    state: &std::sync::Arc<AppState>,
    agent: &str,
    session: &str,
) -> Result<(lys_runner::Act, String, u32), ServerError> {
    let (command, version, requires_controls) = compact_command(state, agent, session)?;
    let operation = lys_identity::OperationId::generate()
        .map_err(|error| ServerError::Runner {
            refusal: "compact_operation_unavailable".to_owned(),
            words: error.to_string(),
        })?
        .to_string();
    if !requires_controls {
        super::queue_wake(state, agent, session, &operation).await?;
    }
    let act = lys_runner::Act::Operate {
        operation: lys_runner::operations::Operation {
            operation,
            session: session.to_owned(),
            request: lys_runner::operations::OperationRequest::Compact {
                text: command.clone(),
            },
        },
    };
    (state.say)(&format!(
        "compact session {session}: profile_version={version}"
    ));
    Ok((act, command, version))
}
