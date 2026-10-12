//! Native feed records charge once before their source cursor is advanced.

use crate::error_agents::AgentsError;
use std::sync::Arc;

use lys_runner::tracking::{Measure, RECORD_VERSION, Unavailable, UsageRecord};
use lys_runner::tracking_store::{Body, FEED_FORMAT, FeedPage};

use crate::budgets_api::{with_budgets, with_budgets_mut};
use crate::budgets_state::Usage;
mod compaction;
use crate::error::ServerError;
use crate::error_budget::BudgetError;
use crate::routes::AppState;
#[cfg(test)]
pub(crate) use compaction::membership_probe;
use compaction::{compaction_holders, source_agent};

fn refused(reason: impl Into<String>) -> ServerError {
    ServerError::Agents(AgentsError::Budget(BudgetError::BudgetsUnavailable {
        reason: reason.into(),
    }))
}

/// Keep each charge durably, then keep the refusals and cursor together.
pub async fn keep_page(
    state: &Arc<AppState>,
    machine: &str,
    page: FeedPage,
) -> Result<(), ServerError> {
    if page.format != FEED_FORMAT || page.cursor.is_empty() {
        return Err(refused("runner feed format or cursor is invalid"));
    }
    let mut refusals = Vec::new();
    let mut kept = false;
    // Read once for the page, before any use is kept.
    let windows = crate::configuration_api::organisation(state)?.model_windows;
    for entry in page.entries {
        match entry.body {
            Body::Usage(record) => {
                if entry.session != record.session {
                    return Err(refused(
                        "runner feed entry and usage name different sessions",
                    ));
                }
                let agent = crate::runtime_api::with_runtime(state, |store| {
                    let tracked = store
                        .session(&record.session)
                        .ok_or(ServerError::RuntimeSessionUnknown)?;
                    if tracked.machine != machine {
                        return Err(refused(
                            "runner feed usage names a session on another machine",
                        ));
                    }
                    tracked
                        .agent
                        .clone()
                        .ok_or_else(|| refused("runner feed usage has no tracked agent"))
                })?;
                let usage = with_budgets(state, |store| {
                    convert(machine, &agent, &record, store.held(), &windows)
                })?;
                crate::budgets_enforce::keep(state, usage).await?;
                crate::budgets_act::review(state, &record.session, &format!("feed-{}", entry.seq))
                    .await?;
                kept = true;
            }
            Body::Refusal(record) => refusals.push(record),
            Body::Operation(outcome) => {
                if outcome.session != entry.session {
                    return Err(refused(
                        "runner feed operation and entry name different sessions",
                    ));
                }
                let crossing = with_budgets(state, |store| {
                    Ok(store.held().crossings.holds(&outcome.operation))
                })?;
                if crossing {
                    let observed = i64::try_from(outcome.at)
                        .map_err(|error| refused(format!("runner outcome instant: {error}")))?;
                    with_budgets_mut(state, |store| {
                        let acted = crate::budgets_crossing::Acted::from_runner(&outcome, observed);
                        if store
                            .held()
                            .crossings
                            .acted
                            .get(&outcome.operation)
                            .is_none_or(|prior| later_control_outcome(prior, &acted))
                        {
                            store.acted(acted)?;
                        }
                        Ok(())
                    })?;
                }
                if let Some(goals) = &state.goals {
                    let holders = if crate::goals_store::actual_compaction(&outcome) {
                        let agent = crate::runtime_api::with_runtime(state, |store| {
                            source_agent(store, machine, &outcome)
                        })?;
                        if goals.with(|store| Ok(store.compaction_kept(&outcome)))? {
                            Vec::new()
                        } else {
                            compaction_holders(state, &agent)?
                        }
                    } else {
                        Vec::new()
                    };
                    let changed = goals.with(|store| {
                        store.control_answer(crate::goals_store::ControlAnswer {
                            outcome: &outcome,
                            holders: &holders,
                            at: crate::session::now(),
                        })
                    })?;
                    if changed {
                        goals.changed.notify_one();
                        kept = true;
                    }
                }
            }
            Body::Managed(record) if record.event == "control_boundary" => {
                if record.binding.session != entry.session {
                    return Err(refused(
                        "managed boundary and feed entry name different sessions",
                    ));
                }
                crate::budgets_act::review(state, &entry.session, &format!("feed-{}", entry.seq))
                    .await?;
            }
            Body::Coverage(_) | Body::Boundary(_) | Body::Injection(_) | Body::Managed(_) => {}
            Body::Commit(_) => {
                return Err(refused("a runner feed page unexpectedly contains a commit"));
            }
        }
    }
    with_budgets_mut(state, |store| {
        store.read_feed(machine, refusals, page.cursor)
    })?;
    if kept {
        // A use arrives on the runner's feed, not on a request: a screen
        // showing an agent's usage or its calls asks again, once for the page.
        state.changes.signal()?;
    }
    Ok(())
}

/// The context windows a person declared, by model: a window in tokens, or
/// none for a model declared as side work, whose calls never set an agent's
/// context.
pub type Windows = std::collections::BTreeMap<String, Option<u64>>;

/// A measured context figure and the declaration used to compute it.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct ContextFigure {
    /// The fraction of the declared window in use, as a whole percent.
    pub percent: u64,
    /// The declaration used: `profile` or `model_windows`.
    pub source: &'static str,
}

/// Use the run's declaration first, then its model's organisation default.
///
/// # Errors
/// Refuses a context beyond its profile window or an invalid model window.
pub fn context_figure(
    record: &UsageRecord,
    context: u64,
    windows: &Windows,
    unavailable: &mut Vec<Unavailable>,
) -> Result<Option<ContextFigure>, ServerError> {
    let mut gap = |reason: String| {
        unavailable.push(Unavailable {
            figure: "context_percent".to_owned(),
            reason,
        });
        Ok(None)
    };
    let model = record.model.as_deref();
    let (window, source) = if record.context_window > 0 {
        if context > record.context_window {
            return Err(refused(
                "native context must fit a positive declared window",
            ));
        }
        (record.context_window, "profile")
    } else {
        match model.and_then(|model| windows.get(model)) {
            Some(Some(0)) => return Err(refused("model_windows must declare positive tokens")),
            Some(Some(tokens)) if context <= *tokens => (*tokens, "model_windows"),
            Some(Some(tokens)) => {
                return gap(format!(
                    "the call's context of {context} tokens is larger than the window of {tokens} declared for its model"
                ));
            }
            Some(None) => return gap(
                "the call's model is declared as side work, which never sets an agent's context"
                    .to_owned(),
            ),
            None => {
                return gap(format!(
                    "no context window is declared for model {}",
                    model.unwrap_or("(not named)")
                ));
            }
        }
    };
    u64::try_from(u128::from(context) * 100 / u128::from(window))
        .map(|percent| Some(ContextFigure { percent, source }))
        .map_err(|error| refused(format!("native context percentage: {error}")))
}

/// Convert cumulative time only once; native snapshots never charge their token totals.
///
/// # Errors
/// Names an invalid usage identity, instant, token total or declared context window.
pub fn convert(
    machine: &str,
    agent: &str,
    record: &UsageRecord,
    prior: &crate::budgets_state::Held,
    windows: &Windows,
) -> Result<Usage, ServerError> {
    if record.version != RECORD_VERSION || record.id.is_empty() || record.session.is_empty() {
        return Err(refused(
            "native usage record version, identity or session is invalid",
        ));
    }
    let at_ms = i64::try_from(record.observed_at)
        .map_err(|error| refused(format!("native observation instant: {error}")))?;
    let mut unavailable = record.unavailable.clone();
    let snapshot = record.measure == Measure::Snapshot;
    let tokens = if snapshot {
        0
    } else {
        tokens(record, &mut unavailable)?
    };
    let reported_running_ms = snapshot.then_some(record.figures.running_ms).flatten();
    let running_ms = if let Some(current) = reported_running_ms {
        let previous = prior
            .index
            .baseline(agent, &record.session, at_ms)
            .map(|position| {
                #[cfg(test)]
                crate::budgets_work::visit(crate::budgets_work::Work::Running);
                prior
                    .uses
                    .get(position)
                    .ok_or_else(|| {
                        refused(format!("running index names missing record {position}"))
                    })?
                    .reported_running_ms
                    .ok_or_else(|| {
                        refused("running index names a record without a cumulative report")
                    })
            })
            .transpose()?;
        if let Some(delta) = current.checked_sub(previous.unwrap_or(0)) {
            delta
        } else {
            unavailable.push(Unavailable {
                figure: "running_ms".to_owned(),
                reason: "native cumulative running time reset; the interval spend is unavailable"
                    .to_owned(),
            });
            0
        }
    } else {
        0
    };
    let context_percent = match record.figures.context_tokens {
        Some(context) => {
            context_figure(record, context, windows, &mut unavailable)?.map(|figure| figure.percent)
        }
        None => None,
    };
    let usage = Usage {
        event: format!("native:{machine}:{}", record.id),
        agent: agent.to_owned(),
        at_ms,
        tokens,
        running_ms,
        dollars_micros: record.figures.dollars_micros,
        account: record.account.clone(),
        plan_windows: snapshot.then(|| record.figures.plan_windows.clone()),
        reported_running_ms,
        native_snapshot: snapshot,
        unavailable,
        session: Some(record.session.clone()),
        context_percent,
        // A spend record the proxy's usage file gave names its run: it counts one call.
        call: record.run.as_ref().filter(|_| !snapshot).map(|run| {
            Box::new(crate::budgets_state::CallSeen {
                id: record.id.clone(),
                model: record.model.clone(),
                input_tokens: record.figures.input_tokens,
                output_tokens: record.figures.output_tokens,
                cache_creation_tokens: record.figures.cache_creation_tokens,
                cache_read_tokens: record.figures.cache_read_tokens,
                run: run.clone(),
                record: record.record.clone(),
            })
        }),
        ..Usage::default()
    };
    crate::budgets_enforce::checked(&usage)?;
    Ok(usage)
}

fn tokens(record: &UsageRecord, unavailable: &mut Vec<Unavailable>) -> Result<u64, ServerError> {
    let fields = [
        record.figures.input_tokens,
        record.figures.output_tokens,
        record.figures.cache_creation_tokens,
        record.figures.cache_read_tokens,
    ];
    let mut total = 0_u64;
    for (index, field) in fields.into_iter().enumerate() {
        match field {
            Some(count) => {
                total = total
                    .checked_add(count)
                    .ok_or_else(|| refused("native token spend overflows"))?;
            }
            None if index == 2 && record.adapter == lys_runner::tracking::CODEX_ADAPTER => {}
            None => {
                unavailable.push(Unavailable {
                    figure: "tokens".to_owned(),
                    reason: "native response does not report every token spend component"
                        .to_owned(),
                });
                return Ok(0);
            }
        }
    }
    Ok(total)
}

fn later_control_outcome(
    prior: &crate::budgets_crossing::Acted,
    next: &crate::budgets_crossing::Acted,
) -> bool {
    use crate::budgets_crossing::Stands;
    let advances = match prior.stands {
        Stands::Confirmed => next.stands == Stands::Confirmed,
        Stands::Uncertain | Stands::Refused => {
            !matches!(next.stands, Stands::Accepted | Stands::Delivered)
        }
        Stands::Delivered => next.stands != Stands::Accepted,
        Stands::Accepted => true,
        Stands::Told => next.stands == Stands::Told,
    };
    prior != next && next.at_ms >= prior.at_ms && advances
}

#[cfg(test)]
#[path = "budgets_feed_tests.rs"]
mod tests;

#[cfg(test)]
pub(crate) mod entry_tests {
    include!("../tests/budgets_feed_entries.rs");
    budgets_feed_entries!();
}
