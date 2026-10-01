//! Confirming a legacy budget version, and the sources whose usage is not
//! yet reported.

use super::{UnitUnavailable, authorised, with_budgets_mut};
use crate::budgets_limits::Limit;
use crate::budgets_state::{Act, Budget, Holder, HolderKind, Length, Measure};
use crate::error::ServerError;
use crate::error_budget::BudgetError;
use crate::routes::{AppState, signed_in};
use crate::session::now;
use axum::Json;
use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use serde::Deserialize;
use std::sync::Arc;

/// The exact legacy budget version an administrator confirms.
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ConfirmBody {
    measure: Measure,
    version: u64,
}

pub(super) async fn confirm(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    body: Result<Json<ConfirmBody>, JsonRejection>,
) -> Result<Json<Budget>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    crate::routes::administrator(&state, &actor)?;
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

pub(super) fn source_gaps(
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
                crate::budgets_usage::source_figure(held, &limit, agents, zone, at_ms).map_err(
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
