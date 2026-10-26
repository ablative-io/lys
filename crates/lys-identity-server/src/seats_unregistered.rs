//! Read every runner concurrently without letting an absent runner hold the survey open.

use std::collections::BTreeSet;
use std::future::Future;
use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::http::HeaderMap;

use super::with_seats;
use crate::error::ServerError;
use crate::network_api::with_network;
use crate::routes::{AppState, signed_in};
use crate::seats_acts::{Live, liveness};
use crate::seats_views::{SeatRunnerUnread, SeatUnregistered, UnregisteredSession};

type Answered = (String, String, Result<Vec<Live>, ServerError>);

async fn answers<F, A>(
    machines: Vec<(String, String)>,
    read: F,
) -> Result<Vec<Answered>, ServerError>
where
    F: Fn(String) -> A,
    A: Future<Output = Result<Vec<Live>, ServerError>> + Send + 'static,
{
    let mut asked = tokio::task::JoinSet::new();
    for (machine, name) in machines {
        let reading = read(machine.clone());
        asked.spawn(async move {
            let answer = reading.await;
            (machine, name, answer)
        });
    }
    let mut answered = Vec::new();
    while let Some(answer) = asked.join_next().await {
        answered.push(answer.map_err(|error| ServerError::Runner {
            refusal: "runner_unreachable".to_owned(),
            words: format!("the runner survey ended abnormally: {error}"),
        })?);
    }
    answered.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(answered)
}

pub(super) async fn list(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<SeatUnregistered>, ServerError> {
    signed_in(&state, &headers)?;
    let held: BTreeSet<String> = with_seats(&state, |store| {
        Ok(store
            .seats()
            .iter()
            .filter_map(|seat| seat.session.clone())
            .collect())
    })?;
    let machines = if state.network.is_none() {
        Vec::new()
    } else {
        with_network(&state, |store| {
            Ok(store
                .machines()
                .iter()
                .filter(|machine| machine.retired.is_none() && store.runner(&machine.id).is_some())
                .map(|machine| (machine.id.clone(), machine.name.clone()))
                .collect())
        })?
    };
    let readings = answers(machines, |machine| {
        let state = Arc::clone(&state);
        async move { liveness(&state, &machine, None).await }
    })
    .await?;
    let mut sessions = Vec::new();
    let mut unanswered = Vec::new();
    for (machine, name, reading) in readings {
        match reading {
            Ok(live) => {
                for live in live
                    .into_iter()
                    .filter(|live| live.alive && !live.ended && !held.contains(&live.session))
                {
                    let agent = super::session_agent(&state, &live.session)?;
                    sessions.push(UnregisteredSession {
                        session: live.session,
                        machine: machine.clone(),
                        agent,
                        pid: live.pid,
                        started_at: live.started_at,
                    });
                }
            }
            Err(refused) => unanswered.push(SeatRunnerUnread {
                machine,
                name,
                refusal: refused.name(),
                reason: refused.to_string(),
            }),
        }
    }
    Ok(Json(SeatUnregistered {
        sessions,
        unanswered,
    }))
}

#[cfg(test)]
#[path = "seats_unregistered_tests.rs"]
mod tests;
