//! Warning slots and the wake occurrence are rendered before their operation is asked.
//! A context warning says where the session stands and where it compacts, in
//! percent of its declared window and in tokens.
use super::{
    Act, AppState, Arc, Crossing, Operation, OperationRequest, ServerError, Undelivered, operate,
    with_budgets,
};
use crate::budgets_state::Measure;
use crate::words_state::Slot;
use std::collections::BTreeMap;

/// Where a context notice is typed, and the compaction level above it.
struct NoticePlace {
    slot: Slot,
    compact: Option<serde_json::Number>,
}

/// The highest of two or more warnings under a compaction is the final one,
/// typed in the preparation slot; every other is a progress warning.
fn notice_place(state: &AppState, crossing: &Crossing) -> Result<NoticePlace, ServerError> {
    with_budgets(state, |store| {
        let Some(collection) = store.held().limit_set_in_force(&crossing.holder) else {
            return Ok(NoticePlace {
                slot: Slot::ContextWarning,
                compact: None,
            });
        };
        let limits = store.held().effective_limits(&collection);
        let crossing_limit = lys_runner::tracking_budget::scaled_exact(&crossing.limit, 6)
            .ok_or_else(|| ServerError::RequestMalformed {
                reason: "warning limit is not representable".to_owned(),
            })?;
        let scaled = |limit: &crate::budgets_limits::Limit| {
            limit
                .scaled()
                .map_err(|error| ServerError::RequestMalformed {
                    reason: error.words,
                })
        };
        let mut next_compact: Option<(u64, &serde_json::Number)> = None;
        for limit in &limits {
            if limit.unit == Measure::ContextPercent && limit.act == Act::Compact {
                let amount = scaled(limit)?;
                if amount > crossing_limit && next_compact.is_none_or(|(next, _)| amount < next) {
                    next_compact = Some((amount, &limit.amount));
                }
            }
        }
        let Some((compact, compact_amount)) = next_compact else {
            return Ok(NoticePlace {
                slot: Slot::ContextWarning,
                compact: None,
            });
        };
        let mut warnings = Vec::new();
        for limit in &limits {
            if limit.unit == Measure::ContextPercent && limit.act == Act::Notice {
                let amount = scaled(limit)?;
                if amount < compact {
                    warnings.push(amount);
                }
            }
        }
        Ok(NoticePlace {
            slot: if warnings.len() > 1 && warnings.into_iter().max() == Some(crossing_limit) {
                Slot::Preparation
            } else {
                Slot::ContextWarning
            },
            compact: Some(compact_amount.clone()),
        })
    })
}

/// A count of tokens with its thousands grouped: `400,000`.
pub(crate) fn grouped(tokens: u64) -> String {
    let digits = tokens.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index) % 3 == 0 {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

/// Where the session stands: `20% of your context window (200,000 of
/// 1,000,000 tokens)`, or the percentage alone when the tokens were not
/// reported with it.
pub(crate) fn context_figures(
    percent: &serde_json::Number,
    tokens: Option<u64>,
    window: Option<u64>,
) -> String {
    match (tokens, window) {
        (Some(tokens), Some(window)) => format!(
            "{percent}% of your context window ({} of {} tokens)",
            grouped(tokens),
            grouped(window)
        ),
        _ => format!("{percent}% of your context window"),
    }
}

/// The compaction level: `40% (400,000 tokens)` of a known window, rounded
/// down to a whole token, or the percentage alone.
///
/// # Errors
/// Names a compaction level that is not a representable percentage.
pub(crate) fn compaction_figures(
    compact: &serde_json::Number,
    window: Option<u64>,
) -> Result<String, String> {
    let Some(window) = window else {
        return Ok(format!("{compact}%"));
    };
    let micros = lys_runner::tracking_budget::scaled_exact(compact, 6)
        .ok_or("the compaction level is not a representable percentage")?;
    let tokens = u128::from(window) * u128::from(micros) / 100_000_000;
    let tokens = u64::try_from(tokens).map_err(|error| error.to_string())?;
    Ok(format!("{compact}% ({} tokens)", grouped(tokens)))
}

pub(super) fn slot_words(
    state: &Arc<AppState>,
    crossing: &Crossing,
) -> Result<Option<String>, ServerError> {
    let place = match crossing.act {
        Act::Notice if crossing.measure == Measure::ContextPercent => {
            notice_place(state, crossing)?
        }
        _ => return Ok(None),
    };
    let mut numbers = BTreeMap::new();
    if let Some(figure) = &crossing.figure {
        numbers.insert("context_percent".to_owned(), figure.to_string());
        numbers.insert(
            "context_figures".to_owned(),
            context_figures(figure, crossing.context_tokens, crossing.context_window),
        );
    }
    numbers.insert("limit".to_owned(), crossing.limit.to_string());
    let compaction = place
        .compact
        .as_ref()
        .map(|compact| compaction_figures(compact, crossing.context_window))
        .transpose()
        .map_err(|reason| ServerError::RequestMalformed { reason })?;
    numbers.insert(
        "compaction".to_owned(),
        compaction.as_ref().map_or_else(
            || "no compaction is set for this agent".to_owned(),
            |figures| format!("compaction is at {figures}"),
        ),
    );
    if let Some(figures) = compaction {
        numbers.insert("compaction_figures".to_owned(), figures);
    }
    crate::words_api::deliverable(
        state,
        place.slot,
        Some(&crossing.agent),
        crossing.session.as_deref(),
        numbers,
        None,
    )
    .map(|delivered| Some(delivered.text))
}

pub(crate) async fn queue_wake(
    state: &Arc<AppState>,
    agent: &str,
    session: &str,
    compaction: &str,
) -> Result<(), ServerError> {
    let wake = format!("wake-up-{compaction}");
    let driven = crate::runner_sessions::driven(state, session)?;
    match crate::runner_client::ask(
        state,
        &driven.machine,
        driven.runner,
        lys_runner::Act::Outcome {
            operation: wake.clone(),
        },
    )
    .await
    {
        Ok(lys_runner::Answer::Operation { outcome }) => return wake_outcome(outcome),
        Err(ServerError::Runner { refusal, .. }) if refusal == "operation_unknown" => {}
        Err(error) => {
            return Err(ServerError::Runner {
                refusal: "wake_up_unavailable".to_owned(),
                words: error.to_string(),
            });
        }
        Ok(other) => {
            return Err(ServerError::Runner {
                refusal: "wake_up_answer_invalid".to_owned(),
                words: format!(
                    "wake receipt answered {}",
                    crate::runner_sessions::kind(&other)
                ),
            });
        }
    }
    let mut numbers = BTreeMap::new();
    numbers.insert(
        "message".to_owned(),
        "Compaction is complete. Continue from your checkpoint.".to_owned(),
    );
    numbers.insert("operation".to_owned(), compaction.to_owned());
    let text = crate::words_api::deliverable(
        state,
        Slot::WakeUp,
        Some(agent),
        Some(session),
        numbers,
        None,
    )?
    .text;
    let operation = Operation {
        operation: wake,
        session: session.to_owned(),
        request: OperationRequest::WakeUp {
            text,
            compaction: compaction.to_owned(),
        },
    };
    let outcome = operate(state, operation)
        .await
        .map_err(|error| ServerError::Runner {
            refusal: match &error {
                Undelivered::Refused(_) => "wake_up_refused",
                Undelivered::Unknown(_) => "wake_up_unavailable",
            }
            .to_owned(),
            words: match error {
                Undelivered::Refused(words) | Undelivered::Unknown(words) => words,
            },
        })?;
    wake_outcome(outcome)
}

fn wake_outcome(outcome: lys_runner::operations::OperationOutcome) -> Result<(), ServerError> {
    if matches!(
        outcome.state,
        lys_runner::operations::OperationState::Refused
            | lys_runner::operations::OperationState::Uncertain
    ) {
        return Err(ServerError::Runner {
            refusal: "wake_up_unconfirmed".to_owned(),
            words: outcome.words,
        });
    }
    Ok(())
}

#[cfg(test)]
#[path = "words_tests.rs"]
mod tests;
