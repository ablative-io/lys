//! A gentle stop waits for a reap. Only a person's End now asks `Act::End`.

use crate::error::ServerError;
use crate::routes::AppState;
use crate::runner_sessions::Driven;
use axum::Json;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use lys_runner::Act;
use serde_json::{Value, json};
use std::sync::Arc;

#[derive(serde::Serialize, utoipa::ToSchema)]
#[serde(transparent)]
#[schema(value_type = Object)]
pub(crate) struct SessionAnswer(Value);

pub(crate) fn asked(
    state: &AppState,
    driven: &Driven,
    caller: &str,
    what: &str,
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
                what: what.to_owned(),
                confirmation: String::new(),
                reported_by: caller.to_owned(),
                at: crate::session::now(),
                launch: None,
            })
            .map(drop)
    })
}

pub(super) async fn stop(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    body: Result<Json<crate::runner_api::Empty>, axum::extract::rejection::JsonRejection>,
) -> Result<Json<SessionAnswer>, ServerError> {
    let (driven, caller) = crate::runner_api::admitted(&state, &headers, &id, "end")?;
    let Json(crate::runner_api::Empty {}) =
        body.map_err(|error| ServerError::RequestMalformed {
            reason: error.body_text(),
        })?;
    let (ended, notes) = crate::seats_monitor::stop_session(
        &state,
        &driven,
        &caller,
        &lys_identity::OperationId::generate()?.to_string(),
    )
    .await?;
    Ok(Json(SessionAnswer(
        json!({"session":id, "answer":{"kind":"ended", "session":id, "ended":ended}, "ended":true, "notes":notes}),
    )))
}

pub(super) async fn end_now(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    body: Result<Json<crate::runner_api::Empty>, axum::extract::rejection::JsonRejection>,
) -> Result<Json<SessionAnswer>, ServerError> {
    let actor = crate::routes::signed_in(&state, &headers)?;
    if matches!(
        actor.provenance().method(),
        lys_identity::AuthMethod::AgentSignature(_) | lys_identity::AuthMethod::AgentPass(_)
    ) {
        return Err(ServerError::NotPermitted {
            reason: "End now is a person's act".to_owned(),
        });
    }
    let (driven, caller) = crate::runner_api::admitted(&state, &headers, &id, "end")?;
    let Json(crate::runner_api::Empty {}) =
        body.map_err(|error| ServerError::RequestMalformed {
            reason: error.body_text(),
        })?;
    asked(&state, &driven, &caller, "End now asked by the person")?;
    crate::runner_api::perform(
        &state,
        (&driven, caller, "end_now"),
        crate::runner_api::Carried::default(),
        Act::End { session: id },
    )
    .await
    .map(|Json(answer)| Json(SessionAnswer(answer)))
}
