//! Delivery reads current goal words and session admission before runner operations.

use super::super::{
    AppState, GoalError, Holder, Live, Operation, ServerError, Standing, goals, judged, now,
    with_runtime,
};
use super::control_recipient;
use crate::goals_store::{Deliver, Delivering};

pub(crate) fn boundary_reminders(
    state: &AppState,
    session: &str,
    queued: &[lys_runner::harness_control::QueuedReminder],
    at: u64,
) -> Result<Vec<lys_runner::harness_control::ReminderDecision>, ServerError> {
    use lys_runner::harness_control::{ReminderDecision, ReminderReference};
    let agent = with_runtime(state, |store| {
        let tracked = store
            .session(session)
            .ok_or(ServerError::RuntimeSessionUnknown)?;
        if tracked.stopped() {
            return Err(ServerError::RuntimeSessionUnknown);
        }
        tracked
            .agent
            .clone()
            .ok_or(ServerError::RuntimeSessionUnknown)
    })?;
    let mut current = std::collections::BTreeMap::new();
    if let Some(goals) = &state.goals {
        current = goals.with(|store| {
            let mut current = std::collections::BTreeMap::new();
            for pending in store.pending_for_session(session)? {
                current.insert(
                    pending.sent.operation.clone(),
                    (
                        pending.item.goal.holder.clone(),
                        pending.item.goal.responsible.clone(),
                        pending.active && pending.item.standing == Standing::Open,
                        ReminderReference {
                            goal: pending.item.goal.id.clone(),
                            occurrence: pending.fired.operation.clone(),
                            version: pending.version.to_owned(),
                            prior: pending.prior.map(str::to_owned),
                        },
                        crate::goals_store::occurrence_text(
                            pending.item,
                            pending.words,
                            pending.fired.due,
                            at,
                            pending.prior,
                        )?,
                    ),
                );
            }
            Ok(current)
        })?;
    }
    let mut decisions = Vec::with_capacity(queued.len());
    for pending in queued {
        let refuse = |reason: &str| ReminderDecision::Refuse {
            operation: pending.operation.clone(),
            reference: pending.reference.clone(),
            reason: reason.to_owned(),
        };
        let Some((holder, responsible, open, reference, text)) = current.remove(&pending.operation)
        else {
            decisions.push(refuse(
                "goal_occurrence_not_pending: the goal owner holds no unsent delivery",
            ));
            continue;
        };
        if !open {
            decisions.push(refuse(
                "goal_closed_or_inactive: queued words are no longer authorised",
            ));
            continue;
        }
        if reference.goal != pending.reference.goal
            || reference.occurrence != pending.reference.occurrence
        {
            decisions.push(refuse(
                "goal_occurrence_changed: the queued identity names another occurrence",
            ));
            continue;
        }
        let authorised = control_recipient(state, &holder, &responsible, &agent, true)?;
        if !authorised {
            decisions.push(refuse(
                "goal_authority_revoked: the session is no longer an admitted recipient",
            ));
            continue;
        }
        decisions.push(ReminderDecision::Deliver {
            operation: pending.operation.clone(),
            reference,
            text,
        });
    }
    Ok(decisions)
}

impl Deliver for Live<'_> {
    fn sessions(&self, holder: &Holder) -> Result<Vec<String>, String> {
        let agents = judged(self.0, holder).map_err(|error| error.to_string())?;
        if self.0.runtime.is_none() {
            return Ok(Vec::new());
        }
        with_runtime(self.0, |store| {
            Ok(store
                .live_ordered(std::ops::Bound::Unbounded)
                .filter(|tracked| {
                    tracked
                        .agent
                        .as_ref()
                        .is_some_and(|agent| agents.contains(agent))
                })
                .map(|tracked| tracked.session.clone())
                .collect())
        })
        .map_err(|error| error.to_string())
    }

    fn operate(&self, operation: Operation) -> Delivering<'_> {
        Box::pin(async move {
            use crate::runner_operate::Undelivered;
            use lys_runner::operations::OperationRequest;
            use lys_runner::{Act, Answer};
            let refused = |error: ServerError| Undelivered::Refused(error.to_string());
            let driven =
                crate::runner_sessions::driven(self.0, &operation.session).map_err(refused)?;
            let status = crate::runner_client::ask(
                self.0,
                &driven.machine,
                driven.runner.clone(),
                Act::ControlStatus {
                    session: operation.session.clone(),
                },
            )
            .await
            .map_err(|error| Undelivered::Unknown(error.to_string()))?;
            let managed = match status {
                Answer::ControlStatus { session, control } if session == operation.session => {
                    control.is_some()
                }
                other => {
                    return Err(Undelivered::Unknown(format!(
                        "goal control status answered {}",
                        crate::runner_sessions::kind(&other)
                    )));
                }
            };
            if !managed {
                return crate::runner_operate::operate(self.0, operation).await;
            }
            match crate::runner_client::ask(
                self.0,
                &driven.machine,
                driven.runner.clone(),
                Act::Outcome {
                    operation: operation.operation.clone(),
                },
            )
            .await
            {
                Ok(Answer::Operation { outcome }) => return Ok(outcome),
                Err(ServerError::Runner { refusal, .. }) if refusal == "operation_unknown" => {}
                Err(error) => return Err(Undelivered::Unknown(error.to_string())),
                Ok(other) => {
                    return Err(Undelivered::Unknown(format!(
                        "goal outcome answered {}",
                        crate::runner_sessions::kind(&other)
                    )));
                }
            }
            let request = goals(self.0)
                .map_err(refused)?
                .with(|store| {
                    let pending = store
                        .pending_operation(&operation.operation)?
                        .ok_or(GoalError::Unknown)?;
                    Ok(OperationRequest::GoalReminder {
                        text: crate::goals_store::occurrence_text(
                            pending.item,
                            pending.words,
                            pending.fired.due,
                            now(),
                            pending.prior,
                        )?,
                        reference: lys_runner::harness_control::ReminderReference {
                            goal: pending.item.goal.id.clone(),
                            occurrence: pending.fired.operation.clone(),
                            version: pending.version.to_owned(),
                            prior: pending.prior.map(str::to_owned),
                        },
                    })
                })
                .map_err(refused)?;
            crate::runner_operate::operate(
                self.0,
                Operation {
                    request,
                    ..operation
                },
            )
            .await
        })
    }
}
