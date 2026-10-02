//! Native feed records charge once before their source cursor is advanced.

use std::sync::Arc;

use lys_runner::tracking::{Measure, RECORD_VERSION, Unavailable, UsageRecord};
use lys_runner::tracking_store::{Body, FEED_FORMAT, FeedPage};

use crate::budgets_api::{with_budgets, with_budgets_mut};
use crate::budgets_state::Usage;
use crate::error::ServerError;
use crate::error_budget::BudgetError;
use crate::routes::AppState;

fn refused(reason: impl Into<String>) -> ServerError {
    ServerError::Budget(BudgetError::BudgetsUnavailable {
        reason: reason.into(),
    })
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
                    convert(machine, &agent, &record, store.held())
                })?;
                crate::budgets_enforce::keep(state, usage).await?;
            }
            Body::Refusal(record) => refusals.push(record),
            Body::Coverage(_) | Body::Boundary(_) | Body::Operation(_) | Body::Injection(_) => {}
            Body::Commit(_) => {
                return Err(refused("a runner feed page unexpectedly contains a commit"));
            }
        }
    }
    with_budgets_mut(state, |store| {
        store.read_feed(machine, refusals, page.cursor)
    })
}

/// Convert cumulative time only once; native snapshots never charge their token totals.
pub fn convert(
    machine: &str,
    agent: &str,
    record: &UsageRecord,
    prior: &crate::budgets_state::Held,
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
    let context_percent = record
        .figures
        .context_tokens
        .map(|context| {
            if record.context_window == 0 || context > record.context_window {
                return Err(refused(
                    "native context must fit a positive declared window",
                ));
            }
            u64::try_from(u128::from(context) * 100 / u128::from(record.context_window))
                .map_err(|error| refused(format!("native context percentage: {error}")))
        })
        .transpose()?;
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
