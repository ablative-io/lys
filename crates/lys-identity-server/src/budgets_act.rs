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

use std::collections::BTreeSet;
use std::str::FromStr;
use std::sync::Arc;

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::routing::get;
use axum::{Json, Router};
use lys_identity::{AgentId, IdentityId};
use lys_runner::operations::{Operation, OperationRequest};
use serde::{Deserialize, Serialize};

use crate::budgets_api::with_budgets;
use crate::budgets_crossing::{Acted, Crossing, Receipt, Stands};
use crate::budgets_state::{Act, Held, Measure, Standing, Usage, covered};
use crate::error::ServerError;
use crate::routes::{AppState, with_directory};
use crate::runner_operate::{Undelivered, operate};
use crate::runner_sessions::operator;

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
}

/// An agent's crossings, each with what came of it.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct UsageView {
    /// The agent.
    pub agent: String,
    /// Each crossing and what came of its act, in the order kept.
    pub receipts: Vec<Receipt>,
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
        crossed: Vec::new(),
    };
    let mut agents: BTreeSet<String> = with_budgets(&state, |store| {
        Ok(store
            .held()
            .uses
            .iter()
            .map(|used| used.agent.clone())
            .collect())
    })?;
    agents.insert(agent.clone());
    let standings = agents
        .iter()
        .map(|each| standing(&state, each))
        .collect::<Result<Vec<_>, _>>()?;
    let targets = Targets {
        live: crate::runner_api::open_sessions(&state, &agent)?
            .into_iter()
            .map(|driven| driven.session)
            .collect(),
        compact: crate::runner_api::session_settings(&state, &agent)?
            .and_then(|settings| settings.compact),
    };
    with_budgets(&state, |store| {
        let crossed = crossings(store.held(), &usage, &standings, &targets)?;
        store.charge(Usage { crossed, ..usage })
    })?;
    settle(&state).await?;
    view(&state, &agent)
}

async fn read(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(agent): Path<String>,
) -> Result<Json<UsageView>, ServerError> {
    operator(&state, &headers, &agent, "usage")?;
    settle(&state).await?;
    view(&state, &agent)
}

fn view(state: &AppState, agent: &str) -> Result<Json<UsageView>, ServerError> {
    with_budgets(state, |store| {
        Ok(Json(UsageView {
            agent: agent.to_owned(),
            receipts: store.held().crossings.of_agent(agent),
        }))
    })
}

/// Where `agent` stands: the teams it is in and its responsible person.
fn standing(state: &AppState, agent: &str) -> Result<Standing, ServerError> {
    let teams = if state.teams.is_some() {
        crate::teams_api::with_teams(state, |store| {
            Ok(store
                .teams()
                .iter()
                .filter(|team| team.retired.is_none() && team.members.iter().any(|m| m == agent))
                .map(|team| team.created.id.clone())
                .collect())
        })?
    } else {
        BTreeSet::new()
    };
    let person = match AgentId::from_str(agent) {
        Ok(id) => with_directory(state, |directory| {
            Ok(directory
                .projection()?
                .record(IdentityId::Agent(id))
                .and_then(lys_identity::projection::Record::responsible)
                .map(|owner| owner.to_string()))
        })?,
        Err(_not_an_agent) => None,
    };
    Ok(Standing {
        agent: agent.to_owned(),
        teams,
        person,
    })
}

/// Where a crossing's act can go: the agent's live sessions, and the
/// compaction command its profile names.
struct Targets {
    live: Vec<String>,
    compact: Option<String>,
}

/// The budgets `usage` crosses, as the budgets stood before it.
fn crossings(
    held: &Held,
    usage: &Usage,
    standings: &[Standing],
    targets: &Targets,
) -> Result<Vec<Crossing>, ServerError> {
    let unavailable = |reason: String| ServerError::BudgetsUnavailable { reason };
    let own = standings
        .iter()
        .find(|standing| standing.agent == usage.agent)
        .cloned()
        .unwrap_or_default();
    let mut crossed = Vec::new();
    for measure in [Measure::ContextPercent, Measure::Tokens, Measure::RunningMs] {
        let Some(budget) = held.applying(&own, measure) else {
            continue;
        };
        let (before, figure, mark) = match measure {
            Measure::ContextPercent => {
                let (Some(session), Some(figure)) = (&usage.session, usage.context_percent) else {
                    continue;
                };
                let before = held.crossings.context.get(session).copied().unwrap_or(0);
                (before, figure, format!("rise {}", usage.event))
            }
            Measure::Tokens | Measure::RunningMs => {
                let agents = covered(budget, standings);
                let before = held
                    .spent(budget, &agents, usage.at_ms)
                    .map_err(unavailable)?;
                let used = if measure == Measure::Tokens {
                    usage.tokens
                } else {
                    usage.running_ms
                };
                let start = match &budget.period {
                    Some(period) => period.start_of(usage.at_ms).map_err(unavailable)?,
                    None => 0,
                };
                (
                    before,
                    before.saturating_add(used),
                    format!("period {start}"),
                )
            }
        };
        if before >= budget.limit || figure < budget.limit {
            continue;
        }
        let sessions: Vec<Option<String>> = match (budget.act, &usage.session) {
            (Act::Tell, _) => vec![None],
            (_, Some(session)) => vec![Some(session.clone())],
            (_, None) if targets.live.is_empty() => vec![None],
            (_, None) => targets.live.iter().cloned().map(Some).collect(),
        };
        for session in sessions {
            let text = match budget.act {
                Act::Compact => targets.compact.clone(),
                Act::Notice => Some(notice(measure, figure, budget.limit)),
                Act::Stop | Act::Tell => None,
            };
            crossed.push(Crossing {
                operation: Crossing::id(
                    &budget.holder,
                    measure,
                    budget.version,
                    &mark,
                    session.as_deref().unwrap_or_default(),
                ),
                holder: budget.holder.clone(),
                measure,
                version: budget.version,
                limit: budget.limit,
                figure,
                act: budget.act,
                agent: usage.agent.clone(),
                session,
                text,
                at_ms: usage.at_ms,
            });
        }
    }
    Ok(crossed)
}

/// The words of a notice typed into a session that reached `limit`.
fn notice(measure: Measure, figure: u64, limit: u64) -> String {
    match measure {
        Measure::ContextPercent => {
            format!("Lys: context is at {figure}% of the window; the budget is {limit}%.")
        }
        Measure::Tokens => format!("Lys: {figure} tokens used this period; the budget is {limit}."),
        Measure::RunningMs => format!(
            "Lys: {} minutes running this period; the budget is {} minutes.",
            figure / 60_000,
            limit / 60_000
        ),
    }
}

/// Ask every crossing not yet settled of its runner, under its own id, and
/// keep what came of each; one whose answer was lost waits for the next.
pub async fn settle(state: &Arc<AppState>) -> Result<(), ServerError> {
    if state.budgets.is_none() {
        return Ok(());
    }
    let unsettled = with_budgets(state, |store| Ok(store.held().crossings.unsettled()))?;
    for crossing in unsettled {
        if let Some(acted) = act(state, &crossing).await {
            with_budgets(state, |store| store.acted(acted))?;
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
        Err(Undelivered::Unknown(_)) => None,
    }
}
