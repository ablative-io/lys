//! Agent gifts require a live grant and a covering holding at the append.

use std::collections::BTreeSet;

use axum::http::HeaderMap;
use lys_identity::grants::{Action, ExerciseRequest, Resource, Route};
use lys_identity::{AgentId, IdentityId};

use crate::budgets_api::{authorised, with_budgets};
use crate::budgets_limits::Limits;
use crate::budgets_state::{Holder, HolderKind, Measure};
use crate::budgets_store::BudgetStore;
use crate::error::ServerError;
use crate::error_budget::BudgetError;
use crate::grants::{Decision, decide, with_grants};
use crate::routes::{AppState, signed_in, with_directory};

pub(crate) enum Giver {
    Person(String),
    Agent(AgentId),
}

impl Giver {
    pub(crate) fn by(&self) -> String {
        match self {
            Self::Person(person) => person.clone(),
            Self::Agent(agent) => agent.to_string(),
        }
    }
}

pub(crate) fn giver(
    state: &AppState,
    headers: &HeaderMap,
    request: (&str, &str, &[u8]),
    holder: &Holder,
) -> Result<Giver, ServerError> {
    if let Some(agent) = with_directory(state, |directory| {
        crate::agent_signature::signed_agent(state, directory.projection()?, headers, request)
    })? {
        return Ok(Giver::Agent(agent));
    }
    let actor = signed_in(state, headers)?;
    let caller = with_directory(state, |directory| {
        crate::caller_admission::active_caller(directory.projection()?, &actor)
    })?;
    if let IdentityId::Agent(agent) = caller {
        return Ok(Giver::Agent(agent));
    }
    let by = authorised(state, &actor, holder)?;
    if holder.kind == HolderKind::Person && !crate::routes::is_administrator(state, &actor)? {
        return Err(ServerError::NotPermitted {
            reason: format!(
                "a person's budget limits that person, so only an administrator sets the budget on person {}",
                holder.id,
            ),
        });
    }
    Ok(Giver::Person(by))
}

pub(crate) fn append(
    state: &AppState,
    giver: &Giver,
    limits: Limits,
    expected: u64,
    zone: &str,
    agents: &BTreeSet<String>,
    at_ms: i64,
) -> Result<(), ServerError> {
    crate::budgets_migration::require_committed(state)?;
    crate::budgets_migration::advance(state)?;
    match giver {
        Giver::Person(_) => with_budgets(state, |store| {
            validate(store, &limits, expected, zone, agents, at_ms)?;
            store.set_limits(limits, expected)?;
            Ok(())
        }),
        Giver::Agent(agent) => with_grants(state, |mut judged| {
            let request = ExerciseRequest {
                caller: IdentityId::Agent(*agent),
                route: Route::Api,
                resource: Resource::new(
                    "budget",
                    &format!("{}.{}", kind(limits.holder.kind), limits.holder.id),
                )?,
                action: Action::new("write")?,
            };
            judged.apps.admit_kind(None, "budget")?;
            judged.apps.admit_action("budget", "write")?;
            decide(&mut judged, &request, limits.at, None, Decision::Explain)?;
            with_budgets(state, |store| {
                crate::budgets_holding::holds(
                    store.held(),
                    &Holder {
                        kind: HolderKind::Agent,
                        id: agent.to_string(),
                    },
                    &limits.limits,
                    zone,
                )?;
                validate(store, &limits, expected, zone, agents, at_ms)?;
                decide(&mut judged, &request, limits.at, None, Decision::Exercise)?;
                store.set_limits(limits, expected)?;
                Ok(())
            })
        }),
    }
}

fn kind(kind: HolderKind) -> &'static str {
    match kind {
        HolderKind::Agent => "agent",
        HolderKind::Team => "team",
        HolderKind::Person => "person",
    }
}

fn validate(
    store: &BudgetStore,
    limits: &Limits,
    expected: u64,
    zone: &str,
    agents: &BTreeSet<String>,
    at_ms: i64,
) -> Result<(), ServerError> {
    let old = store.held().limit_set(&limits.holder);
    let held = old.map_or(0, |limits| limits.version);
    if held != expected {
        return Err(BudgetError::BudgetVersionConflict { held, expected }.into());
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
            return Err(BudgetError::BudgetRefused {
                refusal: "BudgetZoneRefused",
                words: "new limits use the organisation zone; only an existing explicit migrated zone may be retained".to_owned(),
            }.into());
        }
        if matches!(limit.unit, Measure::Dollars | Measure::PlanPercent) {
            let used =
                crate::budgets_usage::source_figure(store.held(), limit, agents, zone, at_ms)
                    .map_err(|reason| BudgetError::BudgetsUnavailable { reason })?;
            if let Some(reason) = used.unavailable {
                return Err(BudgetError::BudgetRefused {
                    refusal: "BudgetUnitUnavailable",
                    words: format!("{} in {:?}: {reason}", limit.unit.name(), limit.period),
                }
                .into());
            }
        }
    }
    Ok(())
}
