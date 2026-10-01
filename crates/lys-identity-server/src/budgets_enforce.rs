//! A known threshold crossing asks each covered agent's act under a durable identity.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use serde_json::Number;

mod levels;
mod validation;
use levels::{Levels, levels};
pub use validation::checked;

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
    let selected = session_coverage(state, &usage.agent, &standings)?;
    let sessions = crate::runtime_api::session_agents(state, &selected)?;
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
        let assessed = crossings(
            store.held(),
            &usage,
            &standings,
            &targets,
            &zone,
            sessions.as_ref(),
        )?;
        let mut usage = usage;
        if let Some(reason) = assessed.missing
            && !usage
                .unavailable
                .iter()
                .any(|gap| gap.figure == "context_percent" && gap.reason == reason)
        {
            usage.unavailable.push(lys_runner::tracking::Unavailable {
                figure: "context_percent".to_owned(),
                reason: reason.to_owned(),
            });
        }
        store.charge(Usage {
            crossed: assessed.crossed,
            ..usage
        })?;
        Ok(())
    })?;
    crate::budgets_act::settle(state).await
}

fn session_coverage(
    state: &AppState,
    agent: &str,
    standings: &[Standing],
) -> Result<BTreeSet<String>, ServerError> {
    with_budgets(state, |store| {
        Ok(store
            .held()
            .limit_sets
            .iter()
            .map(|collection| crate::budgets_members::covered(&collection.holder, standings))
            .filter(|agents| agents.contains(agent))
            .flatten()
            .collect())
    })
}

struct Assessment {
    crossed: Vec<Crossing>,
    missing: Option<&'static str>,
}

struct LimitAt<'a> {
    collection: &'a Limits,
    limit: &'a Limit,
    index: usize,
}

fn crossings(
    held: &Held,
    usage: &Usage,
    standings: &[Standing],
    targets: &BTreeMap<String, Target>,
    zone: &str,
    sessions: Option<&std::collections::BTreeMap<String, crate::runtime_store::SessionActivity>>,
) -> Result<Assessment, ServerError> {
    let mut assessed = Assessment {
        crossed: Vec::new(),
        missing: None,
    };
    for collection in &held.limit_sets {
        let agents = crate::budgets_members::covered(&collection.holder, standings);
        if !agents.contains(&usage.agent) {
            continue;
        }
        for (index, limit) in held.effective_limits(collection).iter().enumerate() {
            let Some(level) = levels(held, usage, limit, &agents, zone, sessions)? else {
                continue;
            };
            if let Some(reason) = level.missing {
                assessed.missing = crate::budgets_context::missing(usage);
                if limit.act == Act::Stop {
                    assessed.crossed.extend(dispatch(
                        held,
                        usage,
                        &agents,
                        targets,
                        &Dispatch {
                            collection,
                            limit,
                            index,
                            figure: None,
                            missing: Some(reason),
                            account: None,
                            act: Act::Stop,
                            is_warning: false,
                            mark: &format!("unavailable {}", usage.event),
                        },
                    )?);
                }
            } else {
                assessed.crossed.extend(measured_crossings(
                    held,
                    usage,
                    &agents,
                    targets,
                    &LimitAt {
                        collection,
                        limit,
                        index,
                    },
                    level,
                )?);
            }
        }
    }
    Ok(assessed)
}

fn measured_crossings(
    held: &Held,
    usage: &Usage,
    agents: &std::collections::BTreeSet<String>,
    targets: &BTreeMap<String, Target>,
    position: &LimitAt<'_>,
    level: Levels,
) -> Result<Vec<Crossing>, ServerError> {
    let LimitAt {
        collection,
        limit,
        index,
    } = *position;
    let Levels {
        before,
        figure,
        since,
        account,
        missing,
    } = level;
    if let Some(reason) = missing {
        return Err(unavailable(reason));
    }
    let mut crossed = Vec::new();
    let figure = figure.ok_or_else(|| unavailable("a measured level has no figure"))?;
    let mark = if limit.unit == Measure::ContextPercent {
        format!("rise {}", usage.event)
    } else {
        format!(
            "period {}",
            since.ok_or_else(|| unavailable("periodic limit has no reported period start"))?
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
            agents,
            targets,
            &Dispatch {
                collection,
                limit,
                index,
                figure: Some(&figure),
                missing: None,
                account: account.as_deref(),
                act,
                is_warning,
                mark: &mark,
            },
        )?);
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
    let selected = session_coverage(state, agent, &standings)?;
    let sessions = crate::runtime_api::session_agents(state, &selected)?;
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
                let used = crate::budgets_usage::figure_with_sessions(
                    store.held(),
                    &limit,
                    &agents,
                    &zone,
                    at_ms,
                    None,
                    sessions.as_ref(),
                )
                .map_err(unavailable)?;
                let figure = match used.figure {
                    Some(figure) => figure,
                    // Running is needed to observe the next window after its recorded reset.
                    None if limit.unit == Measure::PlanPercent
                        && used.since_ms.is_some_and(|boundary| boundary <= at_ms) =>
                    {
                        0.into()
                    }
                    None => {
                        return Err(unavailable(used.unavailable.as_deref().unwrap_or(
                            "a periodic Stop limit has no figure and no source reason",
                        )));
                    }
                };
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

struct Dispatch<'a> {
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
                Act::Notice => Some(
                    notice(
                        limit,
                        figure.ok_or_else(|| unavailable("a notice needs a measured figure"))?,
                        account,
                    )
                    .map_err(unavailable)?,
                ),
                Act::Stop | Act::Tell => None,
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
