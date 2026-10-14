//! Native feed records charge once before their source cursor is advanced.

use crate::error_agents::AgentsError;
use std::sync::Arc;

use lys_runner::tracking::{Measure, RECORD_VERSION, Unavailable, UsageRecord};
use lys_runner::tracking_store::{Body, FEED_FORMAT, FeedEntry, FeedPage};

use crate::budgets_api::{with_budgets, with_budgets_mut};
use crate::budgets_state::Usage;
mod compaction;
use crate::error::ServerError;
use crate::error_budget::BudgetError;
use crate::routes::AppState;
#[cfg(test)]
pub(crate) use compaction::membership_probe;
use compaction::{compaction_holders, source_agent};
#[path = "budgets_feed_usage.rs"]
mod usage_entries;

fn refused(reason: impl Into<String>) -> ServerError {
    ServerError::Agents(AgentsError::Budget(BudgetError::BudgetsUnavailable {
        reason: reason.into(),
    }))
}

/// Keep each charge durably, then keep the refusals and cursor together.
///
/// # Errors
/// Refuses an invalid feed format, cursor or machine binding, unavailable
/// organisation and folded stores, or durable charge, outcome, refusal and
/// cursor writes and change signals that fail. Invalid entries are named
/// in durable refusals, and the remaining page is kept.
pub async fn keep_page(
    state: &Arc<AppState>,
    machine: &str,
    page: FeedPage,
) -> Result<(), ServerError> {
    if page.format != FEED_FORMAT || page.cursor.is_empty() {
        return Err(refused("runner feed format or cursor is invalid"));
    }
    let runner = crate::refusals_follow::bound_runner(state, machine)?;
    let mut refusals = Vec::new();
    let mut kept = false;
    // Read once for the page, before any use is kept.
    let windows = crate::configuration_api::organisation(state)?.model_windows;
    for entry in page.entries {
        if crate::refusals_follow::bound_runner(state, machine)? != runner {
            return Err(refused("runner feed binding changed during its page"));
        }
        if let Some(reason) = entry_problem(state, machine, &runner, &entry)? {
            refusals.push(skipped_entry(state, machine, &entry, &reason)?);
            kept = true;
            continue;
        }
        match &entry.body {
            Body::Usage(record) => {
                let result = usage_entries::keep(
                    state,
                    machine,
                    (entry.seq, entry.at, &entry.session),
                    record,
                    &windows,
                )
                .await;
                match result {
                    Ok(Some(skipped)) => refusals.push(skipped),
                    Ok(None) => {}
                    Err(
                        error @ (ServerError::RuntimeSessionUnknown
                        | ServerError::AgentNotVisible
                        | ServerError::RequestMalformed { .. }),
                    ) => {
                        refusals.push(skipped_entry(state, machine, &entry, &error.to_string())?);
                    }
                    Err(error) => return Err(error),
                }
                kept = true;
            }
            Body::Refusal(record) => refusals.push(record.clone()),
            Body::Operation(outcome) => {
                let crossing = with_budgets(state, |store| {
                    Ok(store.held().crossings.holds(&outcome.operation))
                })?;
                if crossing {
                    let observed = i64::try_from(outcome.at)
                        .map_err(|error| refused(format!("runner outcome instant: {error}")))?;
                    with_budgets_mut(state, |store| {
                        let acted = crate::budgets_crossing::Acted::from_runner(outcome, observed);
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
                    let holders = if crate::goals_store::actual_compaction(outcome) {
                        let agent = crate::runtime_api::with_runtime(state, |store| {
                            source_agent(store, machine, outcome)
                        })?;
                        if goals.with(|store| Ok(store.compaction_kept(outcome)))? {
                            Vec::new()
                        } else {
                            compaction_holders(state, &agent)?
                        }
                    } else {
                        Vec::new()
                    };
                    let changed = goals.with(|store| {
                        store.control_answer(crate::goals_store::ControlAnswer {
                            outcome,
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
                crate::budgets_act::review(state, &entry.session, &format!("feed-{}", entry.seq))
                    .await?;
            }
            Body::Coverage(_) | Body::Boundary(_) | Body::Injection(_) | Body::Managed(_) => {}
            Body::Commit(_) => return Err(refused("runner feed commit passed entry validation")),
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

fn entry_problem(
    state: &AppState,
    machine: &str,
    runner: &crate::runner_client::RunnerRecord,
    entry: &FeedEntry,
) -> Result<Option<String>, ServerError> {
    use crate::runner_client::RunnerRecord;
    use lys_runner::refusals::REFUSAL_VERSION;
    let reason = match &entry.body {
        Body::Usage(record) => match runner {
            RunnerRecord::Dialled {
                runner: Some(writer),
                ..
            } if writer != &record.runner => Some(format!(
                "runner feed usage writer {} differs from bound runner {writer}",
                record.runner
            )),
            _ => None,
        },
        Body::Operation(outcome) if outcome.session != entry.session => {
            Some("runner feed operation and entry name different sessions".to_owned())
        }
        Body::Operation(outcome) if i64::try_from(outcome.at).is_err() => {
            Some("runner outcome instant exceeds the supported Unix milliseconds".to_owned())
        }
        Body::Refusal(record)
            if record.session != entry.session || record.version != REFUSAL_VERSION =>
        {
            Some("runner refusal version or session differs from its feed entry".to_owned())
        }
        Body::Managed(record)
            if record.event == "control_boundary" && record.binding.session != entry.session =>
        {
            Some("managed boundary and feed entry name different sessions".to_owned())
        }
        Body::Commit(_) => Some("a runner feed page unexpectedly contains a commit".to_owned()),
        _ => None,
    };
    let attributed = matches!(&entry.body, Body::Usage(_) | Body::Operation(_))
        || matches!(&entry.body, Body::Managed(record) if record.event == "control_boundary");
    if reason.is_some() || !attributed {
        return Ok(reason);
    }
    let tracked = crate::runtime_api::with_runtime(state, |store| {
        Ok(store
            .session(&entry.session)
            .map(|tracked| (tracked.machine.clone(), tracked.agent.clone())))
    })?;
    if let Some((owner, _)) = &tracked
        && owner != machine
    {
        let standing = crate::network_api::with_network(state, |store| {
            Ok(match store.machine(owner) {
                Some(machine) if machine.retired.is_some() => "retired",
                Some(_) => "active",
                None => "missing",
            })
        })?;
        return Ok(Some(format!(
            "runner feed entry names a session on another machine: writer_machine={machine}; session_machine={owner}; {standing}"
        )));
    }
    if (matches!(&entry.body, Body::Managed(record) if record.event == "control_boundary")
        || (state.goals.is_some()
            && matches!(&entry.body, Body::Operation(outcome) if crate::goals_store::actual_compaction(outcome))))
        && tracked.as_ref().is_none_or(|(_, agent)| agent.is_none())
    {
        return Ok(Some(
            "runner control has no tracked session and agent".to_owned(),
        ));
    }
    Ok(None)
}

fn skipped_entry(
    state: &AppState,
    machine: &str,
    entry: &FeedEntry,
    reason: &str,
) -> Result<lys_runner::refusals::RefusalRecord, ServerError> {
    use lys_runner::refusals::{REFUSAL_VERSION, RefusalRecord};
    let agent = crate::runtime_api::with_runtime(state, |store| {
        Ok(store
            .session(&entry.session)
            .and_then(|tracked| tracked.agent.clone()))
    })?;
    let (agent, attribution) = match agent {
        Some(agent) => (agent, "tracked agent"),
        // The refusal schema uses an empty agent for an unattributed entry; name the absence.
        None => (String::new(), "no tracked agent"),
    };
    let coordinate = format!(
        "seq={}; session={}; followed_machine={machine}",
        entry.seq, entry.session
    );
    (state.say)(&format!(
        "budgets: feed_entry_skipped {coordinate}: {reason}; {attribution}"
    ));
    Ok(RefusalRecord {
        version: REFUSAL_VERSION,
        source: format!("identity-feed:{machine}"),
        attempt: format!("feed-{}", entry.seq),
        session: entry.session.clone(),
        agent,
        at: entry.at,
        tool: "feed".to_owned(),
        target: coordinate.clone(),
        policy_version: 0,
        rule: None,
        check: "feed_entry_skipped".to_owned(),
        grantable: false,
        permission: None,
        grantor: None,
        words: format!("{coordinate}: {reason}; {attribution}"),
    })
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
    /// The declaration used: `profile` or `model`.
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
            Some(Some(tokens)) if context <= *tokens => (*tokens, "model"),
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

/// Convert spend and cumulative time once; snapshots never charge their token totals.
/// Refuses invalid usage identities, instants, totals or declared context windows.
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
    let context = match record.figures.context_tokens {
        Some(context) => context_figure(record, context, windows, &mut unavailable)?,
        None => None,
    };
    let mut usage = Usage {
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
        context_percent: context.as_ref().map(|figure| figure.percent),
        context_source: context.map(|figure| figure.source.to_owned()),
        // A spend record the proxy's usage file gave names its run: it counts one call.
        call: record.run.as_ref().filter(|_| !snapshot).map(|run| {
            Box::new(crate::budgets_state::CallSeen {
                id: record.id.clone(),
                model: record.model.clone(),
                input_tokens: record.figures.input_tokens,
                output_tokens: record.figures.output_tokens,
                cache_creation_tokens: record.figures.cache_creation_tokens.or_else(|| {
                    (record.adapter == lys_runner::tracking::CODEX_ADAPTER).then_some(0)
                }),
                cache_read_tokens: record.figures.cache_read_tokens,
                run: run.clone(),
                record: record.record.clone(),
            })
        }),
        ..Usage::default()
    };
    usage.clear_unobserved_windows();
    crate::budgets_enforce::checked(&usage)?;
    Ok(usage)
}

fn tokens(record: &UsageRecord, unavailable: &mut Vec<Unavailable>) -> Result<u64, ServerError> {
    let fields = [
        ("input_tokens", record.figures.input_tokens),
        ("output_tokens", record.figures.output_tokens),
        (
            "cache_creation_tokens",
            record
                .figures
                .cache_creation_tokens
                .or_else(|| (record.adapter == lys_runner::tracking::CODEX_ADAPTER).then_some(0)),
        ),
        ("cache_read_tokens", record.figures.cache_read_tokens),
    ];
    let mut total = 0_u64;
    for (name, field) in fields {
        match field {
            Some(count) => {
                total = total
                    .checked_add(count)
                    .ok_or_else(|| refused("native token spend overflows"))?;
            }
            None => {
                unavailable.push(Unavailable {
                    figure: "tokens".to_owned(),
                    reason: format!("native response does not report {name}"),
                });
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
#[path = "budgets_codex_tests.rs"]
mod codex_tests;

#[cfg(test)]
#[macro_use]
#[path = "../tests/budgets_feed_entries.rs"]
mod entry_test_definitions;

#[cfg(test)]
pub(crate) mod entry_tests {
    budgets_feed_entries!();
}
