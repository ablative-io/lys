//! Budgets on agents, teams and people, read and changed through the API.
//!
//! A budget is set by its holder's responsible person or an administrator:
//! an agent's by the person responsible for it, a team's by its owner. A
//! person's budget limits that person, so only an administrator sets it; the
//! person reads it. Another person's budget is refused `not_permitted` and
//! its values are never shown. A change names the version it was read at, and a change that
//! crossed another is refused `BudgetVersionConflict`.

use std::str::FromStr;
use std::sync::Arc;

use axum::body::{Body, Bytes};
use axum::extract::rejection::BytesRejection;
use axum::extract::{FromRequest, OriginalUri, Path, State};
use axum::http::{HeaderMap, Request};
use axum::routing::{get, post};
use axum::{Json, Router};
use lys_identity::{Actor, AgentId, IdentityId};
use serde::{Deserialize, Serialize};

use crate::budgets_limits::{Limit, Limits};
use crate::budgets_state::{Holder, HolderKind, Measure};
use crate::budgets_store::BudgetStore;
use crate::budgets_usage::Used;
use crate::error::ServerError;
use crate::error_budget::BudgetError;
use crate::read_api::own_person;
use crate::routes::{AppState, signed_in, with_directory};
use crate::session::now;

mod confirm;
pub(crate) use confirm::ConfirmBody;
use confirm::{confirm, source_gaps};

/// A budget to set on a holder.
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct BudgetBody {
    limits: Vec<Limit>,
    #[serde(default)]
    #[schema(value_type = Option<f64>)]
    warn_at: Option<serde_json::Number>,
    /// The version read before this change; 0 for a budget not yet set.
    version: u64,
}

/// The budgets held on one holder.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct BudgetsView {
    /// The holder.
    pub holder: Holder,
    /// Every requested limit, preserving its own action and explicit migrated zone.
    pub limits: Vec<Limit>,
    /// The warning percentage applied to each limit.
    #[schema(value_type = Option<f64>)]
    pub warn_at: Option<serde_json::Number>,
    /// The organisation zone used when a limit has no explicit one.
    pub zone: String,
    /// The whole holder version; zero when never set.
    pub version: u64,
    /// Who set it; absent when never set.
    pub by: Option<String>,
    /// When set; absent when never set.
    pub at: Option<u64>,
    /// Measured figures, one for each effective limit.
    pub used: Vec<Used>,
    /// Each unit whose source is unavailable, with its named reason.
    pub unavailable: Vec<UnitUnavailable>,
    /// Applicable teams and their actual aggregates, including ancestors.
    pub within: Vec<Within>,
    /// Earlier restrictions still enforced pending legacy confirmation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effective_limits: Option<Vec<Limit>>,
    /// Requested legacy values and the full budgets enforced pending confirmation.
    pub unconfirmed: Vec<crate::budgets_legacy::Unconfirmed>,
}

/// A reported unit is absent for a named reason, never inferred from another unit.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct UnitUnavailable {
    /// The unit with no measured source.
    pub unit: Measure,
    /// The reported gap.
    pub reason: String,
}

/// A team that covers the agent and its deduplicated member aggregate.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct Within {
    /// The team identity.
    pub team: String,
    /// The team display name.
    pub name: String,
    /// The current effective limits.
    pub limits: Vec<Limit>,
    /// The measured aggregate for each limit.
    pub used: Vec<Used>,
}

/// The budget routes.
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/budgets/{kind}/{id}", get(read).put(set))
        .route("/budgets/person/{id}/confirm", post(confirm))
        .route("/teams/{id}/budget", get(team_budget).put(set_team_budget))
}

fn kind_of(kind: &str) -> Result<HolderKind, ServerError> {
    match kind {
        "agent" => Ok(HolderKind::Agent),
        "team" => Ok(HolderKind::Team),
        "person" => Ok(HolderKind::Person),
        other => Err(ServerError::Budget(BudgetError::BudgetRefused {
            refusal: "holder_unknown",
            words: format!("{other} is not a holder: agent, team or person"),
        })),
    }
}

fn refused(holder: &Holder) -> ServerError {
    ServerError::NotPermitted {
        reason: format!(
            "only the responsible person or an administrator reads or sets budgets on {} {}",
            kind_name(holder.kind),
            holder.id
        ),
    }
}

fn kind_name(kind: HolderKind) -> &'static str {
    match kind {
        HolderKind::Agent => "agent",
        HolderKind::Team => "team",
        HolderKind::Person => "person",
    }
}

/// The caller's person, once the caller is proved to hold authority over
/// `holder`: an administrator, or the person responsible for it.
pub(crate) fn authorised(
    state: &AppState,
    actor: &Actor,
    holder: &Holder,
) -> Result<String, ServerError> {
    let administrator = crate::routes::is_administrator(state, actor)?;
    let (person, responsible) = with_directory(state, |directory| {
        let projection = directory.projection()?;
        let person = own_person(projection, actor)?.to_string();
        let responsible = match holder.kind {
            HolderKind::Agent => AgentId::from_str(&holder.id)
                .ok()
                .and_then(|agent| projection.record(IdentityId::Agent(agent)))
                .and_then(lys_identity::projection::Record::responsible)
                .map(|owner| owner.to_string()),
            HolderKind::Person => Some(holder.id.clone()),
            HolderKind::Team => None,
        };
        Ok((person, responsible))
    })?;
    let responsible = match holder.kind {
        HolderKind::Team => crate::teams_api::with_teams(state, |store| {
            Ok(store
                .team(&holder.id)
                .map(|team| team.created.owner.clone()))
        })?,
        HolderKind::Agent | HolderKind::Person => responsible,
    };
    if administrator || responsible.as_deref() == Some(person.as_str()) {
        Ok(person)
    } else {
        Err(refused(holder))
    }
}

pub(crate) fn with_budgets<T>(
    state: &AppState,
    act: impl FnOnce(&mut BudgetStore) -> Result<T, ServerError>,
) -> Result<T, ServerError> {
    let store = state.budgets.as_ref().ok_or_else(|| {
        ServerError::Budget(BudgetError::BudgetsUnavailable {
            reason: "the configuration names no budgets_dir".to_owned(),
        })
    })?;
    let mut store = store.lock().map_err(|error| {
        ServerError::Budget(BudgetError::BudgetsUnavailable {
            reason: format!("budget store lock poisoned: {error}"),
        })
    })?;
    store.settle()?;
    act(&mut store)
}

async fn read(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((kind, id)): Path<(String, String)>,
) -> Result<Json<BudgetsView>, ServerError> {
    holder_budget(&state, &headers, &kind, id).map(Json)
}

/// What `GET /budgets/{kind}/{id}` answers the caller.
pub(crate) fn holder_budget(
    state: &AppState,
    headers: &HeaderMap,
    kind: &str,
    id: String,
) -> Result<BudgetsView, ServerError> {
    let actor = signed_in(state, headers)?;
    let holder = Holder {
        kind: kind_of(kind)?,
        id,
    };
    authorised(state, &actor, &holder)?;
    view(state, &holder)
}

async fn set(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    OriginalUri(uri): OriginalUri,
    Path((kind, id)): Path<(String, String)>,
    principal: Option<axum::Extension<crate::agent_signature::TokenPrincipal>>,
    body: Result<Bytes, BytesRejection>,
) -> Result<Json<BudgetsView>, ServerError> {
    let bytes = body.map_err(|rejected| {
        ServerError::Budget(BudgetError::BudgetRefused {
            refusal: "budget_malformed",
            words: rejected.body_text(),
        })
    })?;
    let holder = Holder {
        kind: kind_of(&kind)?,
        id,
    };
    let giver = crate::budgets_giving::giver(
        &state,
        &headers,
        ("PUT", uri.path(), &bytes),
        &holder,
        principal.as_ref().map(|value| &value.0),
    )?;
    let mut request = Request::new(Body::from(bytes));
    *request.headers_mut() = headers;
    let Json(body) = Json::<BudgetBody>::from_request(request, &state)
        .await
        .map_err(|rejected| {
            ServerError::Budget(BudgetError::BudgetRefused {
                refusal: "budget_malformed",
                words: rejected.body_text(),
            })
        })?;
    let limits = Limits {
        holder: holder.clone(),
        limits: body.limits,
        warn_at: body.warn_at,
        version: body.version,
        by: giver.by(),
        at: now(),
    }
    .checked()
    .map_err(|refused| {
        ServerError::Budget(BudgetError::BudgetRefused {
            refusal: refused.refusal,
            words: refused.words,
        })
    })?;
    let zone = crate::configuration_api::organisation(&state)?.zone;
    let standings = crate::budgets_members::standings(&state)?;
    let agents = crate::budgets_members::covered(&holder, &standings);
    let at_ms = jiff::Timestamp::now().as_millisecond();
    crate::budgets_giving::append(&state, &giver, limits, body.version, &zone, &agents, at_ms)?;
    if let Err(error) = crate::budgets_rejudge::rejudge(&state).await {
        (state.say)(&format!(
            "budgets: the compactions waiting under the old budget were not judged again: {error}"
        ));
    }
    view(&state, &holder).map(Json)
}

/// A mutation checks the shared upgrade state before allowing a new snapshot.
pub(crate) fn with_budgets_mut<T>(
    state: &AppState,
    act: impl FnOnce(&mut BudgetStore) -> Result<T, ServerError>,
) -> Result<T, ServerError> {
    crate::budgets_migration::advance(state)?;
    with_budgets(state, act)
}

async fn team_budget(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<BudgetsView>, ServerError> {
    read(State(state), headers, Path(("team".to_owned(), id))).await
}

async fn set_team_budget(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    uri: OriginalUri,
    Path(id): Path<String>,
    body: Result<Bytes, BytesRejection>,
) -> Result<Json<BudgetsView>, ServerError> {
    set(
        State(state),
        headers,
        uri,
        Path(("team".to_owned(), id)),
        None,
        body,
    )
    .await
}

/// All budget reads share the same source validation and team aggregate computation.
pub(crate) fn view(state: &AppState, holder: &Holder) -> Result<BudgetsView, ServerError> {
    let zone = crate::configuration_api::organisation(state)?.zone;
    let standings = crate::budgets_members::standings(state)?;
    let teams = if holder.kind == HolderKind::Agent && state.teams.is_some() {
        crate::teams_api::with_teams(state, |store| Ok(store.teams().to_vec()))?
    } else {
        Vec::new()
    };
    let mut agents = crate::budgets_members::covered(holder, &standings);
    if holder.kind == HolderKind::Agent {
        let standing = standings
            .iter()
            .find(|standing| standing.agent == holder.id);
        for team in teams.iter().filter(|team| {
            standing.is_some_and(|standing| standing.teams.contains(&team.created.id))
        }) {
            agents.extend(crate::budgets_members::covered(
                &Holder {
                    kind: HolderKind::Team,
                    id: team.created.id.clone(),
                },
                &standings,
            ));
        }
    }
    let sessions = crate::runtime_api::session_agents(state, &agents)?;
    let read = ViewRead {
        holder,
        zone: &zone,
        standings: &standings,
        teams: &teams,
        sessions: sessions.as_ref(),
        at_ms: jiff::Timestamp::now().as_millisecond(),
    };
    with_budgets(state, |store| {
        store.reconcile_context(sessions.as_ref())?;
        view_held(store.held(), &read)
    })
}

struct ViewRead<'a> {
    holder: &'a Holder,
    zone: &'a str,
    standings: &'a [crate::budgets_state::Standing],
    teams: &'a [crate::teams_state::Team],
    sessions: Option<&'a std::collections::BTreeMap<String, crate::runtime_store::SessionActivity>>,
    at_ms: i64,
}

fn view_held(
    held: &crate::budgets_state::Held,
    read: &ViewRead<'_>,
) -> Result<BudgetsView, ServerError> {
    let ViewRead {
        holder,
        zone,
        standings,
        teams,
        sessions,
        at_ms,
    } = *read;
    let collection = held.limit_set(holder);
    let effective = collection.map_or_else(Vec::new, |limits| held.effective_limits(limits));
    let agents = crate::budgets_members::covered(holder, standings);
    let used = effective
        .iter()
        .map(|limit| current_usage(held, limit, &agents, zone, at_ms, sessions))
        .collect::<Result<Vec<_>, _>>()?;
    let mut unavailable = source_gaps(held, &agents, zone, at_ms)?;
    if let Some(reason) = used.iter().find_map(|used| {
        (used.unit == Measure::ContextPercent)
            .then_some(used.unavailable.as_ref())
            .flatten()
    }) {
        unavailable.push(UnitUnavailable {
            unit: Measure::ContextPercent,
            reason: reason.clone(),
        });
    }
    let unconfirmed: Vec<_> = held
        .unconfirmed
        .iter()
        .filter(|pending| pending.requested.holder == *holder)
        .cloned()
        .collect();
    let within = if holder.kind == HolderKind::Agent {
        let standing = standings
            .iter()
            .find(|standing| standing.agent == holder.id);
        teams
            .iter()
            .filter(|team| {
                standing.is_some_and(|standing| standing.teams.contains(&team.created.id))
            })
            .map(|team| {
                let holder = Holder {
                    kind: HolderKind::Team,
                    id: team.created.id.clone(),
                };
                let agents = crate::budgets_members::covered(&holder, standings);
                let limits = held
                    .limit_set(&holder)
                    .map_or_else(Vec::new, |limits| held.effective_limits(limits));
                let used = limits
                    .iter()
                    .map(|limit| current_usage(held, limit, &agents, zone, at_ms, sessions))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(Within {
                    team: holder.id,
                    name: team.created.name.clone(),
                    limits,
                    used,
                })
            })
            .collect::<Result<Vec<_>, ServerError>>()?
    } else {
        Vec::new()
    };
    Ok(BudgetsView {
        holder: holder.clone(),
        limits: collection.map_or_else(Vec::new, |limits| limits.limits.clone()),
        warn_at: collection.and_then(|limits| limits.warn_at.clone()),
        zone: zone.to_owned(),
        version: collection.map_or(0, |limits| limits.version),
        by: collection.map(|limits| limits.by.clone()),
        at: collection.map(|limits| limits.at),
        used,
        unavailable,
        within,
        effective_limits: (!unconfirmed.is_empty()).then_some(effective),
        unconfirmed,
    })
}

fn current_usage(
    held: &crate::budgets_state::Held,
    limit: &Limit,
    agents: &std::collections::BTreeSet<String>,
    zone: &str,
    at_ms: i64,
    sessions: Option<&std::collections::BTreeMap<String, crate::runtime_store::SessionActivity>>,
) -> Result<Used, ServerError> {
    if limit.unit == Measure::ContextPercent {
        return Ok(held
            .context_availability
            .used_live(limit, agents, at_ms, sessions));
    }
    crate::budgets_usage::figure_with_sessions(held, limit, agents, zone, at_ms, None, sessions)
        .map_err(|reason| ServerError::Budget(BudgetError::BudgetsUnavailable { reason }))
}

#[cfg(test)]
#[path = "budgets_live_context_tests.rs"]
mod live_context_tests;
