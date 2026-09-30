//! A known threshold crossing asks each covered agent's act under a durable identity.

use std::collections::BTreeMap;
use std::sync::Arc;

use serde_json::Number;

use crate::budgets_api::{with_budgets, with_budgets_mut};
use crate::budgets_crossing::Crossing;
use crate::budgets_limits::{Limit, Limits};
use crate::budgets_state::{Act, Held, Measure, Standing, Usage};
use crate::error::ServerError;
use crate::error_budget::BudgetError;
use crate::routes::AppState;

fn unavailable(reason: impl std::fmt::Display) -> ServerError {
    ServerError::Budget(BudgetError::BudgetsUnavailable {
        reason: reason.to_string(),
    })
}

/// Validate reported fields before charging any event or asking any act.
pub fn checked(usage: &Usage) -> Result<(), ServerError> {
    if usage.event.is_empty()
        || usage.at_ms < 0
        || usage.context_percent.is_some_and(|value| value > 100)
        || usage.account.as_ref().is_some_and(String::is_empty)
    {
        return Err(ServerError::RequestMalformed { reason: "usage names an event and nonnegative observation instant, context is 0 to 100, and a reported account is nonempty".to_owned() });
    }
    if let Some(windows) = &usage.plan_windows {
        for (index, window) in windows.iter().enumerate() {
            if windows[..index]
                .iter()
                .any(|earlier| earlier.duration_minutes == window.duration_minutes)
                || window
                    .duration_minutes
                    .checked_mul(60_000)
                    .is_none_or(|duration| window.resets_at_ms < duration)
                || window.duration_minutes == 0
                || window.resets_at_ms > i64::MAX.unsigned_abs()
                || lys_runner::tracking_budget::percent(&serde_json::Value::Number(
                    window.used_percent.clone(),
                ))
                .is_none()
            {
                return Err(ServerError::RequestMalformed { reason: "a reported plan window names a positive duration, representable reset instant and percentage from 0 to 100".to_owned() });
            }
        }
    }
    Ok(())
}

/// Unknown figures make no claim about being under a stop threshold.
pub fn reached(unit: Measure, figure: &Number, threshold: &Number) -> Result<bool, String> {
    if !matches!(unit, Measure::PlanPercent | Measure::ContextPercent) {
        let places = if unit == Measure::Dollars { 6 } else { 0 };
        let figure = lys_runner::tracking_budget::scaled_exact(figure, places)
            .ok_or("reported spend is not representable")?;
        let threshold = lys_runner::tracking_budget::scaled_exact(threshold, places)
            .ok_or("spend limit is not representable")?;
        return Ok(figure >= threshold);
    }
    Ok(lys_runner::tracking_budget::compare(figure, threshold)? != std::cmp::Ordering::Less)
}

/// A warning threshold rounds upward, so a monetary notice never happens too early.
fn warning(limit: &Limit, percent: &Number) -> Result<Number, String> {
    let percentage = lys_runner::tracking_budget::scaled_exact(percent, 6)
        .ok_or("warning percentage is not representable")?;
    let amount = u128::from(limit.scaled().map_err(|error| error.words)?)
        .checked_mul(u128::from(percentage))
        .ok_or("warning threshold overflows")?;
    let threshold = amount
        .checked_add(99_999_999)
        .ok_or("warning rounding overflows")?
        / 100_000_000;
    crate::budgets_usage::number(
        limit.unit,
        u64::try_from(threshold).map_err(|error| error.to_string())?,
    )
}

struct Target {
    live: Vec<String>,
    compact: Option<String>,
}

/// Capture every member's actual sessions before storing a crossing.
pub async fn keep(state: &Arc<AppState>, usage: Usage) -> Result<(), ServerError> {
    checked(&usage)?;
    crate::budgets_migration::require_committed(state)?;
    if let Some(session) = &usage.session {
        crate::runner_sessions::usage_session(state, &usage.agent, session)?;
    }
    let standings = crate::budgets_members::standings(state)?;
    if !standings
        .iter()
        .any(|standing| standing.agent == usage.agent)
    {
        return Err(ServerError::AgentNotVisible);
    }
    let zone = crate::configuration_api::organisation(state)?.zone;
    let mut targets = BTreeMap::new();
    for standing in &standings {
        let live = if state.runtime.is_some() {
            crate::runtime_api::with_runtime(state, |store| {
                Ok(store
                    .sessions()
                    .iter()
                    .filter(|tracked| {
                        tracked.agent.as_deref() == Some(standing.agent.as_str())
                            && !tracked.stopped()
                    })
                    .map(|tracked| tracked.session.clone())
                    .collect())
            })?
        } else {
            Vec::new()
        };
        let compact = crate::runner_api::session_settings(state, &standing.agent)?
            .and_then(|settings| settings.compact);
        targets.insert(standing.agent.clone(), Target { live, compact });
    }
    with_budgets_mut(state, |store| {
        if store.held().charged.contains(&usage.event) {
            return Ok(());
        }
        let crossed = crossings(store.held(), &usage, &standings, &targets, &zone)?;
        store.charge(Usage { crossed, ..usage })?;
        Ok(())
    })?;
    crate::budgets_act::settle(state).await
}

fn crossings(
    held: &Held,
    usage: &Usage,
    standings: &[Standing],
    targets: &BTreeMap<String, Target>,
    zone: &str,
) -> Result<Vec<Crossing>, ServerError> {
    let mut crossed = Vec::new();
    for collection in &held.limit_sets {
        let agents = crate::budgets_members::covered(&collection.holder, standings);
        if !agents.contains(&usage.agent) {
            continue;
        }
        for (index, limit) in held.effective_limits(collection).iter().enumerate() {
            let Some(Levels {
                before,
                figure,
                since,
                account,
            }) = levels(held, usage, limit, &agents, zone)?
            else {
                continue;
            };
            let mark = if limit.unit == Measure::ContextPercent {
                format!("rise {}", usage.event)
            } else {
                format!(
                    "period {}",
                    since.ok_or_else(|| unavailable(
                        "periodic limit has no reported period start"
                    ))?
                )
            };
            let mut thresholds = vec![(limit.amount.clone(), limit.act, false)];
            if let Some(percent) = &collection.warn_at {
                thresholds.insert(
                    0,
                    (
                        warning(limit, percent).map_err(unavailable)?,
                        Act::Tell,
                        true,
                    ),
                );
            }
            for (threshold, act, is_warning) in thresholds {
                if !reached(limit.unit, &figure, &threshold).map_err(unavailable)? {
                    continue;
                }
                if !is_warning
                    && before
                        .as_ref()
                        .map(|before| reached(limit.unit, before, &threshold))
                        .transpose()
                        .map_err(unavailable)?
                        .unwrap_or(false)
                {
                    continue;
                }
                let mark = if is_warning && limit.unit == Measure::ContextPercent {
                    format!(
                        "session {}",
                        usage
                            .session
                            .as_deref()
                            .ok_or_else(|| unavailable("a context warning names its session"))?
                    )
                } else {
                    mark.clone()
                };
                crossed.extend(dispatch(
                    held,
                    usage,
                    &agents,
                    targets,
                    &Dispatch {
                        collection,
                        limit,
                        index,
                        figure: &figure,
                        account: account.as_deref(),
                        act,
                        is_warning,
                        mark: &mark,
                    },
                )?);
            }
        }
    }
    Ok(crossed)
}

/// Fresh starts require an available figure below every periodic stop limit.
pub fn admit_at(state: &AppState, agent: &str, at_ms: i64) -> Result<(), ServerError> {
    if state.budgets.is_none() {
        return Ok(());
    }
    let zone = crate::configuration_api::organisation(state)?.zone;
    let standings = crate::budgets_members::standings(state)?;
    with_budgets(state, |store| {
        for collection in &store.held().limit_sets {
            let agents = crate::budgets_members::covered(&collection.holder, &standings);
            if !agents.contains(agent) {
                continue;
            }
            for limit in store.held().effective_limits(collection) {
                if limit.act != Act::Stop || limit.period.is_none() {
                    continue;
                }
                let used =
                    crate::budgets_usage::figure(store.held(), &limit, &agents, &zone, at_ms, None)
                        .map_err(unavailable)?;
                let figure = used.figure.ok_or_else(|| {
                    unavailable(
                        used.unavailable
                            .as_deref()
                            .unwrap_or("a periodic Stop limit has no figure and no source reason"),
                    )
                })?;
                if reached(limit.unit, &figure, &limit.amount).map_err(unavailable)? {
                    let reset = if limit.unit == Measure::PlanPercent {
                        used.since_ms.and_then(|start| {
                            start.checked_add(
                                if limit.period == Some(crate::budgets_state::Length::FiveHour) {
                                    18_000_000
                                } else {
                                    604_800_000
                                },
                            )
                        })
                    } else {
                        crate::budgets_usage::reset(&limit, &zone, at_ms).map_err(unavailable)?
                    };
                    return Err(ServerError::Budget(BudgetError::BudgetExhausted {
                        words: format!(
                            "{} limit {} for {} on {:?} {}; reported {figure}; resets at {} milliseconds since the Unix epoch",
                            limit.unit.name(),
                            limit.amount,
                            limit
                                .period
                                .map_or("none", crate::budgets_state::Length::name),
                            collection.holder.kind,
                            collection.holder.id,
                            reset.ok_or_else(|| unavailable(
                                "an exhausted period has no reset instant"
                            ))?
                        ),
                    }));
                }
            }
        }
        Ok(())
    })
}

fn notice(limit: &Limit, figure: &Number, account: Option<&str>) -> Result<String, String> {
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

struct Levels {
    before: Option<Number>,
    figure: Number,
    since: Option<i64>,
    account: Option<String>,
}

fn levels(
    held: &Held,
    usage: &Usage,
    limit: &Limit,
    agents: &std::collections::BTreeSet<String>,
    zone: &str,
) -> Result<Option<Levels>, ServerError> {
    if limit.unit == Measure::ContextPercent {
        let (Some(session), Some(context)) = (&usage.session, usage.context_percent) else {
            return Ok(None);
        };
        return Ok(Some(Levels {
            before: held
                .crossings
                .context
                .get(session)
                .copied()
                .map(Number::from),
            figure: context.into(),
            since: None,
            account: None,
        }));
    }
    let before = crate::budgets_usage::figure(held, limit, agents, zone, usage.at_ms, None)
        .map_err(unavailable)?;
    let after = crate::budgets_usage::figure(held, limit, agents, zone, usage.at_ms, Some(usage))
        .map_err(unavailable)?;
    let Some(figure) = after.figure else {
        return Ok(None);
    };
    Ok(Some(Levels {
        before: before.figure,
        figure,
        since: after.since_ms,
        account: after.account,
    }))
}

struct Dispatch<'a> {
    collection: &'a Limits,
    limit: &'a Limit,
    index: usize,
    figure: &'a Number,
    account: Option<&'a str>,
    act: Act,
    is_warning: bool,
    mark: &'a str,
}

fn dispatch(
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
        } else if limit.unit == Measure::ContextPercent {
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
            if held
                .crossings
                .crossed
                .iter()
                .any(|crossing| crossing.operation == operation)
            {
                continue;
            }
            let text = match act {
                Act::Compact => target.compact.clone(),
                Act::Notice => Some(notice(limit, figure, account).map_err(unavailable)?),
                Act::Stop | Act::Tell => None,
            };
            crossed.push(Crossing {
                operation,
                holder: collection.holder.clone(),
                measure: limit.unit,
                version: collection.version,
                limit: limit.amount.clone(),
                figure: figure.clone(),
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
