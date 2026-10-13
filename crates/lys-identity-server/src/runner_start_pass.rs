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
    // Read before any pass is issued: a start refused here holds none.
    let proxy = proxied(&launch)?;
    if launch.config.is_none() {
        return Ok(Act::Start {
            tracking: None,
            proxy,
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
        tracking: None,
        proxy,
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
/// environment, with the window the profile explicitly declared. The account
/// is the one each call's own headers name.
///
/// A launch that names no run key does not go through the proxy and is
/// tracked through none. One that names a key and no profile version that
/// reads is refused by name: its calls would be forwarded and paid for with
/// the runner never told to count them, so it is never started untracked
/// in silence.
fn proxied(
    launch: &Launch,
) -> Result<Option<lys_runner::tracking_proxy::ProxyTracking>, ServerError> {
    let Some(run) = launch
        .environment
        .get(lys_home::harness::rendering::RUN_VARIABLE)
    else {
        return Ok(None);
    };
    let refused = |what: &str| ServerError::LaunchUnrenderable {
        reason: format!(
            "the launch of session {} names a run key ({}) and {what}, so its model calls \
             could not be counted; the start is refused",
            launch.session,
            lys_home::harness::rendering::RUN_VARIABLE
        ),
    };
    let profile_version = launch
        .environment
        .get(PROFILE_VERSION_VARIABLE)
        .ok_or_else(|| refused("no profile version (LYS_PROVISIONING_VERSION is absent)"))?
        .parse::<u32>()
        .map_err(|error| {
            refused(&format!(
                "a profile version that does not read as a number \
                 (LYS_PROVISIONING_VERSION: {error})"
            ))
        })?;
    let context_window = launch
        .environment
        .get("LYS_CONTEXT_WINDOW")
        .map(|given| {
            given
                .parse::<u32>()
                .map_err(|error| refused(&format!("an invalid session.context_window: {error}")))
                .and_then(|window| {
                    if window == 0 {
                        Err(refused("session.context_window must be positive"))
                    } else {
                        Ok(u64::from(window))
                    }
                })
        })
        .transpose()?
        .unwrap_or(0);
    Ok(Some(lys_runner::tracking_proxy::ProxyTracking {
        run: run.clone(),
        context_window,
        profile_version,
        account: launch
            .rotation
            .as_ref()
            .and_then(|rotation| rotation.accounts.first().cloned()),
    }))
}

/// The variable a launch names its profile version in.
const PROFILE_VERSION_VARIABLE: &str = "LYS_PROVISIONING_VERSION";

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

#[cfg(test)]
mod profile_window_tests {
    use super::proxied;
    use lys_runner::Launch;
    use std::collections::BTreeMap;
    use std::error::Error;

    fn launch(window: &str) -> Launch {
        Launch {
            session: "session".to_owned(),
            program: "/bin/sh".to_owned(),
            arguments: Vec::new(),
            directory: "/tmp".to_owned(),
            environment: BTreeMap::from([
                (
                    "LYS_RUN".to_owned(),
                    "0123456789abcdef0123456789abcdef".to_owned(),
                ),
                ("LYS_PROVISIONING_VERSION".to_owned(), "1".to_owned()),
                ("LYS_CONTEXT_WINDOW".to_owned(), window.to_owned()),
            ]),
            config: None,
            columns: 120,
            rows: 30,
            rotation: None,
            policy: None,
        }
    }

    #[test]
    fn runner_start_uses_the_profiles_context_window() -> Result<(), Box<dyn Error>> {
        let tracking = proxied(&launch("200000"))?.ok_or("proxy tracking absent")?;
        assert_eq!(tracking.context_window, 200_000);
        Ok(())
    }

    #[test]
    fn runner_start_refuses_invalid_declared_windows_by_name() {
        for window in ["0", "word", "4294967296"] {
            let error = proxied(&launch(window)).expect_err("invalid window accepted");
            assert!(error.to_string().contains("context_window"), "{error}");
        }
    }
    #[test]
    fn run_tracking_records_the_account_handle_without_a_value() -> Result<(), Box<dyn Error>> {
        let mut launch = launch("200000");
        launch.rotation = Some(lys_runner::Rotation {
            accounts: vec!["account-handle".to_owned()],
            variable: "ACCOUNT".to_owned(),
            limit: lys_runner::rotation::Limit::ExitStatus { status: 9 },
            resume_arguments: Vec::new(),
        });
        let tracking = proxied(&launch)?.ok_or("proxy tracking absent")?;
        assert_eq!(tracking.account.as_deref(), Some("account-handle"));
        assert_eq!(serde_json::to_value(tracking)?["account"], "account-handle");
        Ok(())
    }
}
