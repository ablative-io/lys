//! Current control evidence uses the session owner and explicit person decisions.

use std::str::FromStr;
use std::sync::Arc;

use axum::extract::rejection::{JsonRejection, QueryRejection};
use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::routing::{get, post};
use axum::{Json, Router};
use lys_identity::{AgentId, IdentityId, OperationId};
use lys_runner::harness_control::ControlStatus;
use lys_runner::operations::{ControlPage, ControlReceipt, Reconciled, Reconciliation};
use lys_runner::{Act, Answer};
use serde::{Deserialize, Serialize};

use crate::error::ServerError;
use crate::routes::{AppState, signed_in, with_directory};
use crate::runner_sessions::{Driven, machine_runner, operator};
use crate::runtime_api::with_runtime;

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = ControlPageQuery)]
pub(crate) struct PageQuery {
    after: Option<String>,
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = ControlDecisionChoice)]
pub(crate) enum Choice {
    Seen,
    NotSeen,
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = ControlDecisionBody)]
pub(crate) struct DecisionBody {
    operation: String,
    decision: Choice,
}

#[derive(Serialize, utoipa::ToSchema)]
#[schema(as = ControlStatusView)]
pub(crate) struct Status {
    session: String,
    control: Option<ControlStatus>,
}

pub(super) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/agents/{id}/control-sessions", get(agent_sessions))
        .route("/runtime/sessions/{session}/controls", get(status))
        .route("/runtime/sessions/{session}/control-receipts", get(page))
        .route(
            "/runtime/sessions/{session}/control-receipts/{operation}/reconcile",
            post(decide),
        )
}

async fn agent_sessions(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    query: Result<Query<PageQuery>, QueryRejection>,
) -> Result<Json<crate::runtime_state::control::ControlSessions>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    let Ok(agent) = AgentId::from_str(&id) else {
        return Err(ServerError::AgentNotVisible);
    };
    let agent = with_directory(&state, |directory| {
        let projection = directory.projection()?;
        let asker = crate::grants::caller(&state, &headers, projection)?;
        let administrator = state.admission.is_administrator(projection, &actor)?;
        if projection.record(IdentityId::Agent(agent)).is_none()
            || !crate::runtime_api::sees(projection, administrator, asker, &agent.to_string())
        {
            return Err(ServerError::AgentNotVisible);
        }
        Ok(agent.to_string())
    })?;
    let Query(query) = query.map_err(|error| ServerError::RequestMalformed {
        reason: error.body_text(),
    })?;
    with_runtime(&state, |store| {
        store.control_sessions(&agent, query.after.as_deref())
    })
    .map(Json)
}

fn refused(name: &str, reason: impl Into<String>) -> ServerError {
    ServerError::Runner {
        refusal: name.to_owned(),
        words: reason.into(),
    }
}

pub(crate) fn target(
    state: &AppState,
    headers: &HeaderMap,
    session: &str,
    act: &str,
) -> Result<Driven, ServerError> {
    signed_in(state, headers)?;
    with_directory(state, |directory| {
        crate::grants::caller(state, headers, directory.projection()?)
    })?;
    let (agent, machine) = with_runtime(state, |store| {
        let tracked = store
            .session(session)
            .ok_or(ServerError::RuntimeSessionUnknown)?;
        let agent = tracked
            .agent
            .clone()
            .ok_or(ServerError::RuntimeSessionUnknown)?;
        Ok((agent, tracked.machine.clone()))
    })?;
    match operator(state, headers, &agent, act) {
        Err(ServerError::NotPermitted { .. } | ServerError::AgentNotVisible) => {
            return Err(ServerError::RuntimeSessionUnknown);
        }
        answer => {
            answer?;
        }
    }
    let runner = machine_runner(state, &machine)?.ok_or_else(|| ServerError::RunnerAbsent {
        machine: machine.clone(),
    })?;
    Ok(Driven {
        session: session.to_owned(),
        agent,
        machine,
        runner,
    })
}

pub(crate) fn service_target(state: &AppState, session: &str) -> Result<Driven, ServerError> {
    let (agent, machine) = with_runtime(state, |store| {
        let tracked = store
            .session(session)
            .ok_or(ServerError::RuntimeSessionUnknown)?;
        Ok((
            tracked
                .agent
                .clone()
                .ok_or(ServerError::RuntimeSessionUnknown)?,
            tracked.machine.clone(),
        ))
    })?;
    let runner = machine_runner(state, &machine)?.ok_or_else(|| ServerError::RunnerAbsent {
        machine: machine.clone(),
    })?;
    Ok(Driven {
        session: session.to_owned(),
        agent,
        machine,
        runner,
    })
}

async fn current_status(
    state: &Arc<AppState>,
    driven: &Driven,
) -> Result<Option<ControlStatus>, ServerError> {
    let answer = crate::runner_client::ask(
        state,
        &driven.machine,
        driven.runner.clone(),
        Act::ControlStatus {
            session: driven.session.clone(),
        },
    )
    .await?;
    match answer {
        Answer::ControlStatus { session, control } if session == driven.session => Ok(control),
        other => Err(refused(
            "control_status_mismatch",
            format!(
                "the runner answered {} without the requested current control",
                crate::runner_sessions::kind(&other)
            ),
        )),
    }
}

pub(crate) async fn managed_status(
    state: &Arc<AppState>,
    driven: &Driven,
) -> Result<ControlStatus, ServerError> {
    let control = current_status(state, driven).await?.ok_or_else(|| {
        refused(
            "control_transport_unsupported",
            "the intended session has no live managed control channel",
        )
    })?;
    if matches!(
        control.phase,
        lys_runner::harness_control::ControlPhase::Unknown
            | lys_runner::harness_control::ControlPhase::Closed
    ) {
        return Err(refused(
            "control_boundary_unknown",
            "the intended session has no proved active or idle state",
        ));
    }
    Ok(control)
}

pub(crate) async fn read_receipt(
    state: &Arc<AppState>,
    driven: &Driven,
    operation: &str,
) -> Result<ControlReceipt, ServerError> {
    let answer = crate::runner_client::ask(
        state,
        &driven.machine,
        driven.runner.clone(),
        Act::ControlReceipt {
            operation: operation.to_owned(),
        },
    )
    .await?;
    match answer {
        Answer::ControlReceipt { receipt }
            if receipt.session == driven.session && receipt.operation == operation =>
        {
            Ok(receipt)
        }
        other => Err(refused(
            "control_receipt_mismatch",
            format!(
                "the runner answered {} without the requested operation and session",
                crate::runner_sessions::kind(&other)
            ),
        )),
    }
}

async fn status(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(session): Path<String>,
) -> Result<Json<Status>, ServerError> {
    let driven = target(&state, &headers, &session, "read control status")?;
    let control = current_status(&state, &driven).await?;
    Ok(Json(Status { session, control }))
}

async fn page(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(session): Path<String>,
    query: Result<Query<PageQuery>, QueryRejection>,
) -> Result<Json<ControlPage>, ServerError> {
    let driven = target(&state, &headers, &session, "read control receipts")?;
    let Query(query) = query.map_err(|error| ServerError::RequestMalformed {
        reason: error.body_text(),
    })?;
    let answer = crate::runner_client::ask(
        &state,
        &driven.machine,
        driven.runner,
        Act::ControlReceipts {
            session: session.clone(),
            after: query.after,
        },
    )
    .await?;
    match answer {
        Answer::ControlReceipts { page } if page.session == session => Ok(Json(page)),
        other => Err(refused(
            "control_page_mismatch",
            format!(
                "the runner answered {} without the requested session",
                crate::runner_sessions::kind(&other)
            ),
        )),
    }
}

async fn decide(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((session, operation)): Path<(String, String)>,
    body: Result<Json<DecisionBody>, JsonRejection>,
) -> Result<Json<ControlReceipt>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    let person = with_directory(&state, |directory| {
        crate::read_api::own_person(directory.projection()?, &actor)
    })?
    .to_string();
    let Json(body) = body.map_err(|error| ServerError::RequestMalformed {
        reason: error.body_text(),
    })?;
    OperationId::from_str(&body.operation).map_err(|error| ServerError::RequestMalformed {
        reason: error.to_string(),
    })?;
    let driven = target(&state, &headers, &session, "reconcile a control receipt")?;
    let receipt = read_receipt(&state, &driven, &operation).await?;
    responsible(&state, &driven, &receipt, &person)?;
    let decision = match body.decision {
        Choice::Seen => Reconciliation::Seen,
        Choice::NotSeen => Reconciliation::NotSeen,
    };
    record_decision(
        &state,
        &driven,
        &receipt,
        Reconciled {
            operation: body.operation,
            by: person,
            at: u64::try_from(jiff::Timestamp::now().as_millisecond())
                .map_err(|error| refused("control_instant_invalid", error.to_string()))?,
            decision,
        },
    )
    .await
    .map(Json)
}

pub(crate) fn responsible(
    state: &AppState,
    driven: &Driven,
    receipt: &ControlReceipt,
    person: &str,
) -> Result<(), ServerError> {
    let permitted = if let Some(reference) = &receipt.reference {
        let (holder, responsible) = crate::goals_api::goals(state)?.with(|store| {
            let item = store
                .item(&reference.goal)
                .ok_or(crate::goals_state::GoalError::Unknown)?;
            Ok((item.goal.holder.clone(), item.goal.responsible.clone()))
        })?;
        responsible == person
            && crate::goals_api::control_recipient(state, &holder, person, &driven.agent, false)?
    } else {
        with_directory(state, |directory| {
            let agent = AgentId::from_str(&driven.agent)
                .map_err(|error| refused("control_agent_invalid", error.to_string()))?;
            let record = directory
                .projection()?
                .record(IdentityId::Agent(agent))
                .ok_or(ServerError::AgentNotVisible)?;
            Ok(record
                .responsible()
                .is_some_and(|owner| owner.to_string() == person))
        })?
    };
    if permitted {
        Ok(())
    } else {
        Err(ServerError::NotPermitted {
            reason: "only the current responsible person records this decision".to_owned(),
        })
    }
}

pub(crate) async fn record_decision(
    state: &Arc<AppState>,
    driven: &Driven,
    receipt: &ControlReceipt,
    decision: Reconciled,
) -> Result<ControlReceipt, ServerError> {
    if let Some(kept) = &receipt.reconciled {
        if kept.operation == decision.operation
            && kept.by == decision.by
            && kept.decision == decision.decision
        {
            return Ok(receipt.clone());
        }
        return Err(refused(
            "control_already_reconciled",
            "a different person decision is already recorded",
        ));
    }
    if receipt.state != lys_runner::operations::OperationState::Uncertain {
        return Err(refused(
            "control_not_uncertain",
            "only uncertain delivery takes a person decision",
        ));
    }
    let expected = decision.clone();
    let Json(answer) = crate::runner_api::perform(
        state,
        (driven, decision.by.clone(), "reconcile_control"),
        crate::runner_api::Carried::default(),
        Act::ReconcileControl {
            operation: receipt.operation.clone(),
            decision,
        },
    )
    .await?;
    let current: ControlReceipt = serde_json::from_value(answer["answer"]["receipt"].clone())
        .map_err(|error| refused("control_receipt_unreadable", error.to_string()))?;
    if current.operation != receipt.operation
        || current.session != receipt.session
        || current.reconciled.as_ref() != Some(&expected)
        || current.state != receipt.state
        || current.admitted != receipt.admitted
    {
        return Err(refused(
            "control_receipt_mismatch",
            "the decision answer does not preserve the requested identity, personal decision and admission evidence",
        ));
    }
    state.changes.signal()?;
    Ok(current)
}
