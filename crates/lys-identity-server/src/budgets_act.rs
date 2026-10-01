//! What a reached budget does, once per crossing.
//!
//! A use is reported for an agent at `POST /agents/{id}/usage`. Each budget
//! it crosses is kept with it, under a stable operation id, before anything
//! is asked; then each crossing not yet settled is asked of its session's
//! runner under that id: a compaction or a notice typed at the next turn
//! boundary, or a stop. A notice to the person ('tell') is kept for them to
//! read and sends nothing outward. After a lost answer or a restart the same
//! operation is asked again, never a replacement; a stop is confirmed only
//! on the exit the runner saw. `GET /agents/{id}/usage` settles what it can
//! and answers each crossing with what came of it: its receipt.

use std::sync::Arc;

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::routing::get;
use axum::{Json, Router};
use lys_runner::operations::{Operation, OperationRequest};
use serde::{Deserialize, Serialize};

use crate::budgets_api::{with_budgets, with_budgets_mut};
use crate::budgets_crossing::{Acted, Crossing, Receipt, Stands};
use crate::budgets_state::{Act, Standing, Usage};
use crate::error::ServerError;
use crate::routes::AppState;
use crate::runner_operate::{Undelivered, operate};
use crate::runner_sessions::{operator, usage_session};

/// A use measured for an agent.
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct UsageBody {
    /// The feed event's stable id: an event reported again charges nothing.
    event: String,
    /// When, in milliseconds since the Unix epoch.
    at_ms: i64,
    #[serde(default)]
    tokens: u64,
    #[serde(default)]
    running_ms: u64,
    #[serde(default)]
    session: Option<String>,
    #[serde(default)]
    context_percent: Option<u64>,
    #[serde(default)]
    dollars_micros: Option<u64>,
    #[serde(default)]
    account: Option<String>,
    #[serde(default)]
    plan_windows: Option<Vec<lys_runner::tracking_budget::PlanWindow>>,
}

/// An agent's crossings, each with what came of it.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct UsageView {
    /// The agent.
    pub agent: String,
    /// Each crossing and what came of its act, in the order kept.
    pub receipts: Vec<Receipt>,
    /// When the latest usage kept for the agent was measured, in
    /// milliseconds since the Unix epoch; none while its runner has
    /// reported none, so its budgets cannot yet be reached.
    pub last_reported_ms: Option<i64>,
    /// One measured figure or named gap for each current effective limit.
    pub used: Vec<crate::budgets_usage::Used>,
}

/// The usage routes.
pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/agents/{id}/usage", get(read).post(report))
}

async fn report(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(agent): Path<String>,
    given: Result<Json<UsageBody>, JsonRejection>,
) -> Result<Json<UsageView>, ServerError> {
    let Json(given) = given.map_err(|refused| ServerError::RequestMalformed {
        reason: refused.body_text(),
    })?;
    operator(&state, &headers, &agent, "usage")?;
    if let Some(session) = given.session.as_deref() {
        usage_session(&state, &agent, session)?;
    }
    if given.context_percent.is_some_and(|figure| figure > 100) {
        return Err(ServerError::RequestMalformed {
            reason: "context_percent is a percentage: 0 to 100".to_owned(),
        });
    }
    let usage = Usage {
        event: given.event,
        agent: agent.clone(),
        at_ms: given.at_ms,
        tokens: given.tokens,
        running_ms: given.running_ms,
        session: given.session,
        context_percent: given.context_percent,
        dollars_micros: given.dollars_micros,
        account: given.account,
        plan_windows: given.plan_windows,
        ..Usage::default()
    };
    crate::budgets_enforce::keep(&state, usage).await?;
    view(&state, &agent)
}

async fn read(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(agent): Path<String>,
) -> Result<Json<UsageView>, ServerError> {
    operator(&state, &headers, &agent, "usage")?;
    settle_for(&state, &agent).await?;
    view(&state, &agent)
}

fn view(state: &AppState, agent: &str) -> Result<Json<UsageView>, ServerError> {
    let budget = crate::budgets_api::view(
        state,
        &crate::budgets_state::Holder {
            kind: crate::budgets_state::HolderKind::Agent,
            id: agent.to_owned(),
        },
    )?;
    with_budgets(state, |store| {
        Ok(Json(UsageView {
            agent: agent.to_owned(),
            receipts: store.held().crossings.of_agent(agent),
            last_reported_ms: store.held().index.last_reported(agent),
            used: budget.used,
        }))
    })
}

/// Where `agent` stands: the teams it is in and its responsible person.
fn standing(state: &AppState, agent: &str) -> Result<Standing, ServerError> {
    crate::budgets_members::standings(state)?
        .into_iter()
        .find(|standing| standing.agent == agent)
        .ok_or(ServerError::AgentNotVisible)
}

/// Ask every crossing not yet settled of its runner, under its own id, and
/// keep what came of each; one whose answer was lost waits for the next.
pub async fn settle(state: &Arc<AppState>) -> Result<(), ServerError> {
    if state.budgets.is_none() {
        return Ok(());
    }
    let unsettled = with_budgets(state, |store| Ok(store.held().crossings.unsettled()))?;
    settle_crossings(state, unsettled).await
}

pub(crate) async fn settle_for(state: &Arc<AppState>, agent: &str) -> Result<(), ServerError> {
    if state.budgets.is_none() {
        return Ok(());
    }
    let unsettled = with_budgets(state, |store| {
        Ok(store.held().crossings.unsettled_for(agent))
    })?;
    settle_crossings(state, unsettled).await
}

pub(crate) async fn settle_crossings(
    state: &Arc<AppState>,
    unsettled: Vec<Crossing>,
) -> Result<(), ServerError> {
    for crossing in unsettled {
        if let Some(acted) = act(state, &crossing).await {
            with_budgets_mut(state, |store| store.acted(acted))?;
        }
    }
    Ok(())
}

/// Settle what a stopped server left unsettled, once, when it starts.
pub fn settle_at_start(state: &Arc<AppState>) {
    if state.budgets.is_none() {
        return;
    }
    let state = Arc::clone(state);
    tokio::spawn(async move {
        if let Err(error) = settle(&state).await {
            (state.say)(&format!("budgets: settling at start failed: {error}"));
        }
    });
}

fn at_ms() -> i64 {
    jiff::Timestamp::now().as_millisecond()
}

/// What came of asking `crossing`'s act; none when the answer was lost.
async fn act(state: &Arc<AppState>, crossing: &Crossing) -> Option<Acted> {
    let kept = |stands: Stands, words: String| Acted {
        operation: crossing.operation.clone(),
        stands,
        words,
        at_ms: at_ms(),
        ended: None,
    };
    if crossing.holder.kind == crate::budgets_state::HolderKind::Team {
        match standing(state, &crossing.agent) {
            Ok(standing) if standing.teams.contains(&crossing.holder.id) => {}
            Ok(_) => {
                return Some(kept(
                    Stands::Refused,
                    format!(
                        "team_membership_held: agent `{}` is not an admitted member of team `{}`",
                        crossing.agent, crossing.holder.id
                    ),
                ));
            }
            Err(error) => {
                (state.say)(&format!(
                    "budget crossing `{}` awaits membership read: {error}",
                    crossing.operation
                ));
                return None;
            }
        }
    }
    if let Some(session) = crossing.session.as_deref()
        && let Err(error) = usage_session(state, &crossing.agent, session)
    {
        return Some(kept(Stands::Refused, error.to_string()));
    }
    let request = match (crossing.act, crossing.text.clone()) {
        (Act::Tell, _) => {
            return Some(kept(
                Stands::Told,
                format!(
                    "kept for agent {}'s responsible person to read; nothing is sent",
                    crossing.agent
                ),
            ));
        }
        (Act::Stop, _) => OperationRequest::Stop,
        (Act::Compact, Some(text)) => OperationRequest::Compact { text },
        (Act::Notice, Some(text)) => OperationRequest::Notice { text },
        (Act::Compact, None) => {
            return Some(kept(
                Stands::Refused,
                format!(
                    "compact_unnamed: the profile of {} names no compaction command",
                    crossing.agent
                ),
            ));
        }
        (Act::Notice, None) => {
            return Some(kept(Stands::Refused, "a notice with no words".to_owned()));
        }
    };
    let Some(session) = crossing.session.clone() else {
        return Some(kept(
            Stands::Refused,
            format!(
                "no_live_session: agent {} had no live session when the budget was reached",
                crossing.agent
            ),
        ));
    };
    let operation = Operation {
        operation: crossing.operation.clone(),
        session,
        request,
    };
    match operate(state, operation).await {
        Ok(outcome) => Some(Acted::from_runner(&outcome, at_ms())),
        Err(Undelivered::Refused(words)) => Some(kept(Stands::Refused, words)),
        Err(Undelivered::Unknown(error)) => {
            (state.say)(&format!(
                "budget crossing {} awaits its runner answer: {error}",
                crossing.operation
            ));
            None
        }
    }
}
