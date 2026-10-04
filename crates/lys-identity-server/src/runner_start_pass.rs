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
    seat: Option<lys_home::harness::lys_mcp::Seat>,
) -> Result<Act, ServerError> {
    if passes.has_session(&launch.session)? {
        return Ok(Act::Status {
            session: Some(launch.session),
        });
    }
    if launch.config.is_none() {
        return Ok(Act::Start {
            proxy: proxied(&launch),
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
        proxy: proxied(&launch),
        launch: Box::new(launch),
        lys_mcp: Some(lys_runner::protocol::LysMcp {
            url: format!("{origin}/api/mcp"),
            pass: pass.to_string(),
            seat,
        }),
    })
}

/// How the run is tracked through the proxy, when its launch minted it a run
/// key: the key and the profile version, as the launch wrote them in its
/// environment. No profile declares a context window yet, so none is said,
/// and the account is the one each call's own headers name.
fn proxied(launch: &Launch) -> Option<lys_runner::tracking_proxy::ProxyTracking> {
    let run = launch
        .environment
        .get(lys_home::harness::rendering::RUN_VARIABLE)?;
    let profile_version = launch
        .environment
        .get("LYS_PROVISIONING_VERSION")?
        .parse()
        .ok()?;
    Some(lys_runner::tracking_proxy::ProxyTracking {
        run: run.clone(),
        context_window: 0,
        profile_version,
        account: None,
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
