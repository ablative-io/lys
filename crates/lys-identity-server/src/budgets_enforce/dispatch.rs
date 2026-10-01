//! What a crossing does to the session it covers: the notice it types, and
//! the tell or stop sent to the runner.

use super::*;
use crate::budgets_api::{with_budgets, with_budgets_mut};
use crate::budgets_crossing::Crossing;
use crate::budgets_limits::{Limit, Limits};
use crate::budgets_state::{Act, Held, Measure, Standing, Usage};
use crate::error::ServerError;
use crate::error_budget::BudgetError;
use crate::routes::AppState;
use levels::{Levels, levels};
use serde_json::Number;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

pub(super) fn notice(
    limit: &Limit,
    figure: &Number,
    account: Option<&str>,
) -> Result<String, String> {
    Ok(match limit.unit {
        Measure::ContextPercent => format!(
            "Lys: context is at {figure}%; the budget is {}%.",
            limit.amount
        ),
        Measure::PlanPercent => format!(
            "Lys: account {} is at {figure}% of its shared {} plan; the budget is {}%.",
            account.ok_or("a reported plan figure names its account")?,
            limit
                .period
                .ok_or("a reported plan figure names its period")?
                .name(),
            limit.amount
        ),
        Measure::Dollars => format!(
            "Lys: reported dollar spend is ${figure}; the budget is ${}.",
            limit.amount
        ),
        Measure::Tokens | Measure::RunningMs => format!(
            "Lys: {} is at {figure}; the budget is {}.",
            limit.unit.name(),
            limit.amount
        ),
    })
}

pub(super) struct Dispatch<'a> {
    collection: &'a Limits,
    limit: &'a Limit,
    index: usize,
    figure: Option<&'a Number>,
    missing: Option<&'a str>,
    account: Option<&'a str>,
    act: Act,
    is_warning: bool,
    mark: &'a str,
}

pub(super) fn dispatch(
    held: &Held,
    usage: &Usage,
    agents: &std::collections::BTreeSet<String>,
    targets: &BTreeMap<String, Target>,
    request: &Dispatch<'_>,
) -> Result<Vec<Crossing>, ServerError> {
    let Dispatch {
        collection,
        limit,
        index,
        figure,
        missing,
        account,
        act,
        is_warning,
        mark,
    } = *request;
    let mut crossed = Vec::new();
    let recipients = if limit.unit == Measure::ContextPercent {
        std::collections::BTreeSet::from([usage.agent.clone()])
    } else {
        agents.clone()
    };
    for agent in recipients {
        let target = targets
            .get(&agent)
            .ok_or_else(|| unavailable("a covered agent has no target snapshot"))?;
        let sessions = if act == Act::Tell
            || (limit.unit != Measure::ContextPercent && target.live.is_empty())
        {
            vec![None]
        } else if limit.unit == Measure::ContextPercent && usage.session.is_some() {
            vec![usage.session.clone()]
        } else {
            target.live.iter().cloned().map(Some).collect()
        };
        for session in sessions {
            let identity = format!("limit {index} warning {is_warning} recipient {agent} {mark}");
            let operation = Crossing::id(
                &collection.holder,
                limit.unit,
                collection.version,
                &identity,
                session.as_deref().unwrap_or_default(),
            );
            if held.crossings.holds(&operation) {
                continue;
            }
            let text = match act {
                Act::Notice => Some(
                    notice(
                        limit,
                        figure.ok_or_else(|| unavailable("a notice needs a measured figure"))?,
                        account,
                    )
                    .map_err(unavailable)?,
                ),
                Act::Compact | Act::Stop | Act::Tell => None,
            };
            crossed.push(Crossing {
                operation,
                holder: collection.holder.clone(),
                measure: limit.unit,
                version: collection.version,
                limit: limit.amount.clone(),
                figure: figure.cloned(),
                unavailable: missing.map(str::to_owned),
                limit_index: u64::try_from(index).map_err(unavailable)?,
                warning: is_warning,
                account: account.map(str::to_owned),
                act,
                agent: agent.clone(),
                session,
                text,
                at_ms: usage.at_ms,
            });
        }
    }
    Ok(crossed)
}
