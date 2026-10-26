//! A stop waits for the runner's reap. No elapsed interval decides its outcome.

use std::sync::Arc;

use lys_runner::{Act, Answer};

use crate::error::ServerError;
use crate::routes::AppState;
use crate::runner_sessions::Driven;

pub(super) async fn end_within(
    state: &Arc<AppState>,
    driven: &Driven,
) -> Result<Answer, ServerError> {
    let liveness = crate::runner_client::ask(
        state,
        &driven.machine,
        driven.runner.clone(),
        Act::Liveness {
            session: Some(driven.session.clone()),
        },
    )
    .await?;
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
    wait_for_reap(crate::runner_client::ask(
        state,
        &driven.machine,
        driven.runner.clone(),
        Act::End {
            session: driven.session.clone(),
        },
    ))
    .await
}

async fn wait_for_reap(
    end: impl std::future::Future<Output = Result<Answer, ServerError>>,
) -> Result<Answer, ServerError> {
    let answer = end.await?;
    if let Answer::Ended { session, ended } = &answer {
        ended.require_reaped(session)?;
        Ok(answer)
    } else {
        Err(ServerError::Runner {
            refusal: "runner_reply_malformed".to_owned(),
            words: "an end was not answered with the runner's reap".to_owned(),
        })
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
#[path = "exit_tests.rs"]
mod exit_tests;
