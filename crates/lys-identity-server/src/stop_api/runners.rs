//! Read runtime confirmations and ask each recorded runner to end its session.

use super::{AppState, ConfirmedEnd, ServerError, StopView, exit, now, with_runtime};
use std::sync::Arc;

/// `view` with each asked session whose end its runtime confirmed.
pub(super) fn confirmed(state: &AppState, mut view: StopView) -> Result<StopView, ServerError> {
    if state.runtime.is_none() {
        return Ok(view);
    }
    view.sessions_confirmed = with_runtime(state, |store| {
        Ok(view
            .sessions_asked
            .iter()
            .filter_map(|session| store.session(session))
            .filter(|tracked| tracked.stopped())
            .filter_map(|tracked| {
                tracked.latest().map(|report| ConfirmedEnd {
                    session: tracked.session.clone(),
                    at: report.at,
                    confirmation: report.confirmation.clone(),
                })
            })
            .collect())
    })?;
    exit::read_refusals(state, &mut view)?;
    Ok(view)
}

/// Ask the runner of each session to end it, every session at once so no
/// runner waits on another, answering, by name, each session whose runner
/// refused or could not be reached. A session on a machine that names no
/// runner stays asked, as its runtime has not said.
async fn end_sessions(state: &Arc<AppState>, sessions: Vec<String>, by: &str) -> Vec<String> {
    let mut asked = tokio::task::JoinSet::new();
    for session in sessions {
        let (state, by) = (Arc::clone(state), by.to_owned());
        asked.spawn(async move { end_session(&state, &session, &by).await });
    }
    let mut refused = Vec::new();
    while let Some(ended) = asked.join_next().await {
        match ended {
            Ok(named) => refused.extend(named),
            Err(failed) => refused.push(format!(
                "runner_unreachable: asking a runner to end a session ended abnormally: {failed}"
            )),
        }
    }
    refused.sort();
    refused
}

/// Ask the runner of `session` to end it, answering each refusal by name.
async fn end_session(state: &Arc<AppState>, session: &str, by: &str) -> Vec<String> {
    let mut refused = Vec::new();
    let driven = match crate::runner_sessions::driven(state, session) {
        Ok(driven) => driven,
        Err(ServerError::RunnerAbsent { .. }) => return refused,
        Err(other) => {
            refused.push(format!("{session}: {other}"));
            return refused;
        }
    };
    let answered = exit::end_within(state, &driven).await;
    let kept = match &answered {
        Ok(lys_runner::Answer::Ended {
            session: answered,
            ended,
        }) if answered == session => ended
            .require_reaped(session)
            .map_err(ServerError::from)
            .and_then(|()| crate::runner_sessions::record_end(state, &driven, ended)),
        Ok(other) => Err(ServerError::Runner {
            refusal: "runner_reply_malformed".to_owned(),
            words: format!(
                "an end was answered {}",
                crate::runner_sessions::kind(other)
            ),
        }),
        Err(error) => Err(ServerError::Runner {
            refusal: error.name(),
            words: error.to_string(),
        }),
    };
    let outcome = kept
        .as_ref()
        .map_or_else(ToString::to_string, |()| "ended".to_owned());
    let refusal_record = if kept.is_err() {
        exit::keep_refusal(state, &driven, by, &outcome)
    } else {
        Ok(())
    };
    let receipt = crate::runner_sessions::keep_act(
        state,
        crate::runner_acts::RunnerAct {
            act: "end".to_owned(),
            caller: by.to_owned(),
            session: session.to_owned(),
            agent: driven.agent.clone(),
            machine: driven.machine.clone(),
            at: now(),
            text: None,
            keys: Vec::new(),
            outcome,
        },
    );
    for failed in [kept.err(), receipt.err(), refusal_record.err()]
        .into_iter()
        .flatten()
    {
        refused.push(format!("{session}: {failed}"));
    }
    refused
}

/// `view` once each asked session not yet confirmed ended has been asked of
/// its runner, all at once, with each confirmed end and each refusal.
pub(super) async fn through_runners(
    state: &Arc<AppState>,
    view: StopView,
    by: &str,
) -> Result<StopView, ServerError> {
    let view = confirmed(state, view)?;
    let unconfirmed: Vec<String> = view
        .sessions_asked
        .iter()
        .filter(|session| {
            !view
                .sessions_confirmed
                .iter()
                .any(|end| &end.session == *session)
        })
        .cloned()
        .collect();
    let sessions_refused = end_sessions(state, unconfirmed, by).await;
    let mut view = confirmed(state, view)?;
    view.sessions_refused = sessions_refused;
    Ok(view)
}
