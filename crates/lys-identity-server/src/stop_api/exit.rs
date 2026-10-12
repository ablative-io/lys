//! A stop waits for the runner's reap answer within the named end budget.

use std::sync::Arc;
use std::time::Duration;

use lys_runner::{Act, Answer};

use crate::error::ServerError;
use crate::routes::AppState;
use crate::runner_sessions::Driven;

const END_BUDGET: Duration = Duration::from_secs(10);

pub(super) async fn end_within(
    state: &Arc<AppState>,
    driven: &Driven,
) -> Result<Answer, ServerError> {
    let deadline = tokio::time::Instant::now() + END_BUDGET;
    let liveness = tokio::time::timeout_at(
        deadline,
        crate::runner_client::ask(
            state,
            &driven.machine,
            driven.runner.clone(),
            Act::Liveness {
                session: Some(driven.session.clone()),
            },
        ),
    )
    .await
    .map_err(|_| lingering(&driven.session, None))??;
    let Answer::Liveness { sessions } = liveness else {
        return Err(ServerError::Runner {
            refusal: "runner_reply_malformed".to_owned(),
            words: "an end's liveness request was not answered with liveness".to_owned(),
        });
    };
    let [live] = sessions.as_slice() else {
        return Err(ServerError::Runner {
            refusal: "runner_reply_malformed".to_owned(),
            words: "an end's liveness request did not name exactly one session".to_owned(),
        });
    };
    if live.session != driven.session {
        return Err(ServerError::Runner {
            refusal: "runner_reply_malformed".to_owned(),
            words: "an end's liveness answer names another session".to_owned(),
        });
    }
    bounded(
        &driven.session,
        live.pid,
        deadline,
        crate::runner_client::ask(
            state,
            &driven.machine,
            driven.runner.clone(),
            Act::End {
                session: driven.session.clone(),
            },
        ),
    )
    .await
}

async fn bounded(
    session: &str,
    pid: Option<u32>,
    deadline: tokio::time::Instant,
    end: impl std::future::Future<Output = Result<Answer, ServerError>>,
) -> Result<Answer, ServerError> {
    tokio::time::timeout_at(deadline, end)
        .await
        .map_err(|_| lingering(session, pid))?
}

fn lingering(session: &str, pid: Option<u32>) -> ServerError {
    let process = pid.map_or_else(
        || "an unreported pid".to_owned(),
        |pid| format!("pid {pid}"),
    );
    ServerError::Runner {
        refusal: "process_lingering".to_owned(),
        words: format!("session {session}: harness process {process} was not reaped within 10 s"),
    }
}

pub(super) fn read_refusals(
    state: &AppState,
    view: &mut super::StopView,
) -> Result<(), ServerError> {
    view.sessions_refused = crate::runtime_api::with_runtime(state, |store| {
        Ok(view
            .sessions_asked
            .iter()
            .filter_map(|session| store.session(session))
            .filter(|tracked| !tracked.stopped())
            .filter_map(|tracked| {
                tracked
                    .latest()
                    .filter(|report| {
                        report.what.starts_with("process_lingering:")
                            || report.what.starts_with("process_exit_unconfirmed:")
                    })
                    .map(|report| format!("{}: {}", tracked.session, report.what))
            })
            .collect())
    })?;
    Ok(())
}

pub(super) fn keep_refusal(
    state: &AppState,
    driven: &Driven,
    caller: &str,
    outcome: &str,
) -> Result<(), ServerError> {
    if state.runtime.is_none() {
        return Ok(());
    }
    crate::runtime_api::with_runtime(state, |store| {
        if store
            .session(&driven.session)
            .is_none_or(crate::runtime_state::Tracked::stopped)
        {
            return Ok(());
        }
        store
            .report(crate::runtime_state::Report {
                operation: lys_identity::OperationId::generate()?.to_string(),
                session: driven.session.clone(),
                agent: Some(driven.agent.clone()),
                machine: driven.machine.clone(),
                state: crate::runtime_state::Reported::StopAsked,
                what: outcome.to_owned(),
                confirmation: String::new(),
                reported_by: caller.to_owned(),
                at: crate::session::now(),
                launch: None,
            })
            .map(drop)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn stop_end_budget_names_the_lingering_pid() -> Result<(), Box<dyn std::error::Error>> {
        let error = bounded(
            "held",
            Some(41),
            tokio::time::Instant::now(),
            std::future::pending(),
        )
        .await
        .err()
        .ok_or("a pending reap was accepted")?;
        assert_eq!(error.name(), "process_lingering");
        assert!(error.to_string().contains("pid 41"));
        Ok(())
    }

    #[tokio::test]
    async fn stop_end_budget_accepts_a_reaped_answer() -> Result<(), Box<dyn std::error::Error>> {
        let ended = lys_runner::Ended {
            how: lys_runner::EndedHow::Exited,
            at: 1,
            status: Some(0),
            signal: None,
            reason: None,
            stopped: None,
        };
        let answer = bounded(
            "reaped",
            Some(42),
            tokio::time::Instant::now() + END_BUDGET,
            std::future::ready(Ok(Answer::Ended {
                session: "reaped".to_owned(),
                ended,
            })),
        )
        .await?;
        let Answer::Ended { session, ended } = answer else {
            return Err("end answer changed".into());
        };
        ended.require_reaped(&session)?;
        Ok(())
    }
}
