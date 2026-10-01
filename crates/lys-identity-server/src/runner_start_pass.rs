//! Select one run credential while its table is exclusively borrowed.
use crate::agent_pass_store::Passes;
use crate::error::ServerError;
use lys_identity::AgentId;
use lys_runner::{Act, Launch};

#[cfg(test)]
#[path = "runner_start_pass_tests.rs"]
mod tests;

pub(crate) fn act(
    passes: &mut Passes,
    agent: AgentId,
    origin: &str,
    launch: Launch,
) -> Result<Act, ServerError> {
    if passes.has_session(&launch.session)? {
        return Ok(Act::Status {
            session: Some(launch.session),
        });
    }
    if launch.config.is_none() {
        return Ok(Act::Start {
            launch: Box::new(launch),
            lys_mcp: None,
        });
    }
    let record = launch
        .environment
        .get("LYS_LAUNCH_RECORD")
        .map_or(launch.session.as_str(), String::as_str);
    // The check that no pass is held and the issue take one lock, so two
    // starts of the same session cannot both issue.
    let Some(pass) = passes.issue_unless_present(agent, record, &launch.session)? else {
        return Ok(Act::Status {
            session: Some(launch.session),
        });
    };
    Ok(Act::Start {
        launch: Box::new(launch),
        lys_mcp: Some(lys_runner::protocol::LysMcp {
            url: format!("{origin}/api/mcp"),
            pass: pass.to_string(),
        }),
    })
}

pub(crate) fn record_after_end<T>(
    ending: Result<(), ServerError>,
    keep: impl FnOnce() -> Result<T, ServerError>,
) -> Result<T, ServerError> {
    let kept = keep();
    match (ending, kept) {
        (Ok(()), result) => result,
        (Err(ending), Ok(_)) => Err(ending),
        (Err(ending), Err(recording)) => Err(ServerError::RuntimeUnavailable {
            reason: format!(
                "ending the run pass failed: {ending}; keeping the runner act failed: {recording}"
            ),
        }),
    }
}
