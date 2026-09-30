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

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::routing::{get, post};
use axum::{Json, Router};
use lys_identity::{Actor, AgentId, IdentityId};
use serde::{Deserialize, Serialize};

use crate::budgets_limits::{Limit, Limits};
use crate::budgets_state::{Act, Budget, Holder, HolderKind, Length, Measure};
use crate::budgets_store::BudgetStore;
use crate::budgets_usage::Used;
use crate::error::ServerError;
use crate::error_budget::BudgetError;
use crate::read_api::own_person;
use crate::routes::{AppState, signed_in, with_directory};
use crate::session::now;

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
    let administrator = state.admission.administrator(actor).is_ok();
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
    let actor = signed_in(&state, &headers)?;
    let holder = Holder {
        kind: kind_of(&kind)?,
        id,
    };
    authorised(&state, &actor, &holder)?;
    view(&state, &holder).map(Json)
}

async fn set(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((kind, id)): Path<(String, String)>,
    body: Result<Json<BudgetBody>, JsonRejection>,
) -> Result<Json<BudgetsView>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    let Json(body) = body.map_err(|rejected| {
        ServerError::Budget(BudgetError::BudgetRefused {
            refusal: "budget_malformed",
            words: rejected.body_text(),
        })
    })?;
    let holder = Holder {
        kind: kind_of(&kind)?,
        id,
    };
    let by = authorised(&state, &actor, &holder)?;
    if holder.kind == HolderKind::Person && state.admission.administrator(&actor).is_err() {
        return Err(ServerError::NotPermitted {
            reason: format!(
                "a person's budget limits that person, so only an administrator sets the budget on person {}",
                holder.id
            ),
        });
    }
    let limits = Limits {
        holder: holder.clone(),
        limits: body.limits,
        warn_at: body.warn_at,
        version: body.version,
        by,
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
    with_budgets(&state, |store| {
        let old = store.held().limit_set(&holder);
        let held = old.map_or(0, |limits| limits.version);
        if held != body.version {
            return Err(ServerError::Budget(BudgetError::BudgetVersionConflict {
                held,
                expected: body.version,
            }));
        }
        for limit in &limits.limits {
            if limit.zone.is_some()
                && old.is_none_or(|old| {
                    !old.limits.iter().any(|earlier| {
                        earlier.unit == limit.unit
                            && earlier.period == limit.period
                            && earlier.zone == limit.zone
                    })
                })
            {
                return Err(ServerError::Budget(BudgetError::BudgetRefused{ refusal: "BudgetZoneRefused", words: "new limits use the organisation zone; only an existing explicit migrated zone may be retained".to_owned() }));
            }
            if matches!(limit.unit, Measure::Dollars | Measure::PlanPercent) {
                let used =
                    crate::budgets_usage::figure(store.held(), limit, &agents, &zone, at_ms, None)
                        .map_err(|reason| {
                            ServerError::Budget(BudgetError::BudgetsUnavailable { reason })
                        })?;
                if let Some(reason) = used.unavailable {
                    return Err(ServerError::Budget(BudgetError::BudgetRefused {
                        refusal: "BudgetUnitUnavailable",
                        words: format!("{} in {:?}: {reason}", limit.unit.name(), limit.period),
                    }));
                }
            }
        }
        Ok(())
    })?;
    crate::budgets_migration::require_committed(&state)?;
    with_budgets_mut(&state, |store| store.set_limits(limits, body.version))?;
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
    Path(id): Path<String>,
    body: Result<Json<BudgetBody>, JsonRejection>,
) -> Result<Json<BudgetsView>, ServerError> {
    set(State(state), headers, Path(("team".to_owned(), id)), body).await
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
    with_budgets(state, |store| {
        view_held(
            store.held(),
            holder,
            &zone,
            &standings,
            &teams,
            jiff::Timestamp::now().as_millisecond(),
        )
    })
}

pub(crate) fn view_held(
    held: &crate::budgets_state::Held,
    holder: &Holder,
    zone: &str,
    standings: &[crate::budgets_state::Standing],
    teams: &[crate::teams_state::Team],
    at_ms: i64,
) -> Result<BudgetsView, ServerError> {
    let collection = held.limit_set(holder);
    let effective = collection.map_or_else(Vec::new, |limits| held.effective_limits(limits));
    let agents = crate::budgets_members::covered(holder, standings);
    let used = effective
        .iter()
        .map(|limit| {
            crate::budgets_usage::figure(held, limit, &agents, zone, at_ms, None)
                .map_err(|reason| ServerError::Budget(BudgetError::BudgetsUnavailable { reason }))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let unavailable = source_gaps(held, &agents, zone, at_ms)?;
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
                    .map(|limit| {
                        crate::budgets_usage::figure(held, limit, &agents, zone, at_ms, None)
                            .map_err(|reason| {
                                ServerError::Budget(BudgetError::BudgetsUnavailable { reason })
                            })
                    })
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

/// The exact legacy budget version an administrator confirms.
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ConfirmBody {
    measure: Measure,
    version: u64,
}

async fn confirm(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    body: Result<Json<ConfirmBody>, JsonRejection>,
) -> Result<Json<Budget>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    state.admission.administrator(&actor)?;
    crate::budgets_migration::require_committed(&state)?;
    let Json(body) = body.map_err(|error| ServerError::RequestMalformed {
        reason: error.body_text(),
    })?;
    let holder = Holder {
        kind: HolderKind::Person,
        id,
    };
    let by = authorised(&state, &actor, &holder)?;
    with_budgets_mut(&state, |store| {
        if !store.held().unconfirmed.iter().any(|pending| {
            pending.requested.holder == holder && pending.requested.measure == body.measure
        }) {
            return Err(ServerError::Budget(BudgetError::BudgetRefused {
                refusal: "budget_invalid",
                words: "only an unconfirmed legacy personal budget can be confirmed".to_owned(),
            }));
        }
        let budget = store
            .held()
            .budget(&holder, body.measure)
            .cloned()
            .ok_or_else(|| {
                ServerError::Budget(BudgetError::BudgetRefused {
                    refusal: "budget_invalid",
                    words: format!(
                        "person `{}` has no {:?} budget to confirm",
                        holder.id, body.measure
                    ),
                })
            })?;
        store.set_confirmed(
            Budget {
                by,
                at: now(),
                ..budget
            },
            body.version,
        )
    })
    .map(Json)
}

fn source_gaps(
    held: &crate::budgets_state::Held,
    agents: &std::collections::BTreeSet<String>,
    zone: &str,
    at_ms: i64,
) -> Result<Vec<UnitUnavailable>, ServerError> {
    let mut unavailable = Vec::new();
    for (unit, periods) in [
        (Measure::Dollars, vec![Length::Week]),
        (Measure::PlanPercent, vec![Length::FiveHour, Length::Week]),
    ] {
        let probes = periods
            .into_iter()
            .map(|period| {
                let limit = Limit {
                    unit,
                    amount: 1.into(),
                    period: Some(period),
                    act: Act::Stop,
                    zone: None,
                };
                crate::budgets_usage::figure(held, &limit, agents, zone, at_ms, None).map_err(
                    |reason| ServerError::Budget(BudgetError::BudgetsUnavailable { reason }),
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        if probes.iter().all(|probe| probe.figure.is_none()) {
            let reasons: Vec<_> = probes
                .into_iter()
                .filter_map(|probe| probe.unavailable)
                .collect();
            unavailable.push(UnitUnavailable {
                unit,
                reason: reasons.join("; "),
            });
        }
    }
    Ok(unavailable)
}
