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

mod dispatch;
use dispatch::{Dispatch, dispatch};

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
}

/// Capture indexed activity, resolve only crossed compaction targets, then commit at one revision.
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
    let agent = usage.agent.clone();
    let zone = crate::configuration_api::organisation(state)?.zone;
    let selected = session_coverage(state, &agent, &standings)?;
    let sessions = crate::runtime_api::session_agents(state, &selected)?;
    let targets: BTreeMap<_, _> = selected
        .iter()
        .map(|agent| {
            let live = sessions
                .as_ref()
                .and_then(|sessions| sessions.get(agent))
                .map(|activity| activity.live_sessions().iter().cloned().collect())
                .unwrap_or_default();
            (agent.clone(), Target { live })
        })
        .collect();
    let mut incoming = Some(usage);
    loop {
        let snapshot = with_budgets(state, |store| {
            let usage = incoming
                .as_ref()
                .ok_or_else(|| unavailable("the usage was already committed"))?;
            if store.held().charged.contains(&usage.event) {
                return Ok(None);
            }
            Ok(Some((
                store.revision(),
                crossings(
                    store.held(),
                    usage,
                    &standings,
                    &targets,
                    &zone,
                    sessions.as_ref(),
                )?,
            )))
        })?;
        let Some((revision, mut assessed)) = snapshot else {
            return crate::budgets_act::settle_for(state, &agent).await;
        };
        resolve_commands(state, &mut assessed.crossed)?;
        let committed = with_budgets_mut(state, |store| {
            commit_assessment(
                store,
                &mut incoming,
                revision,
                assessed,
                sessions.as_ref(),
                &agent,
            )
        })?;
        if let Some(pending) = committed {
            return crate::budgets_act::settle_crossings(state, pending).await;
        }
    }
}

fn resolve_commands(state: &AppState, crossed: &mut [Crossing]) -> Result<(), ServerError> {
    let mut commands = BTreeMap::new();
    for crossing in crossed {
        if crossing.act == Act::Compact {
            if !commands.contains_key(&crossing.agent) {
                let command = crate::runner_api::session_settings(state, &crossing.agent)?
                    .and_then(|settings| settings.compact);
                commands.insert(crossing.agent.clone(), command);
            }
            crossing.text = commands.get(&crossing.agent).cloned().flatten();
        }
    }
    Ok(())
}

fn commit_assessment(
    store: &mut crate::budgets_store::BudgetStore,
    incoming: &mut Option<Usage>,
    revision: u64,
    assessed: Assessment,
    sessions: Option<&BTreeMap<String, crate::runtime_store::SessionActivity>>,
    agent: &str,
) -> Result<Option<Vec<Crossing>>, ServerError> {
    if store.revision() != revision {
        return Ok(None);
    }
    let mut usage = incoming
        .take()
        .ok_or_else(|| unavailable("the usage was already committed"))?;
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
    let other: Vec<_> = assessed
        .crossed
        .iter()
        .filter(|crossing| crossing.agent != *agent)
        .cloned()
        .collect();
    store.charge(Usage {
        crossed: assessed.crossed,
        ..usage
    })?;
    store.reconcile_context(sessions)?;
    let mut pending = store.held().crossings.unsettled_for(agent);
    pending.extend(other);
    Ok(Some(pending))
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
