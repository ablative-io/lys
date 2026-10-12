//! Warning slots and the wake occurrence are rendered before their operation is asked.
use super::*;
use crate::budgets_state::Measure;
use crate::words_state::Slot;
use std::collections::BTreeMap;

fn notice_slot(state: &AppState, crossing: &Crossing) -> Result<Slot, ServerError> {
    with_budgets(state, |store| {
        let Some(collection) = store.held().limit_set(&crossing.holder) else {
            return Ok(Slot::ContextWarning);
        };
        let limits = store.held().effective_limits(collection);
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
        let mut next_compact = None;
        for limit in &limits {
            if limit.unit == Measure::ContextPercent && limit.act == Act::Compact {
                let amount = scaled(limit)?;
                if amount > crossing_limit && next_compact.is_none_or(|next| amount < next) {
                    next_compact = Some(amount);
                }
            }
        }
        let Some(compact) = next_compact else {
            return Ok(Slot::ContextWarning);
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
        Ok(
            if warnings.len() > 1 && warnings.into_iter().max() == Some(crossing_limit) {
                Slot::Preparation
            } else {
                Slot::ContextWarning
            },
        )
    })
}

pub(super) fn slot_words(
    state: &Arc<AppState>,
    crossing: &Crossing,
) -> Result<Option<String>, ServerError> {
    let slot = match crossing.act {
        Act::Notice if crossing.measure == Measure::ContextPercent => notice_slot(state, crossing)?,
        _ => return Ok(None),
    };
    let mut numbers = BTreeMap::new();
    if let Some(figure) = &crossing.figure {
        numbers.insert("context_percent".to_owned(), figure.to_string());
    }
    numbers.insert("limit".to_owned(), crossing.limit.to_string());
    crate::words_api::deliverable(
        state,
        slot,
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
