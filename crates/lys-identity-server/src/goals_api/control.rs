//! Boundary authority reads current saved words and current recipient admission.

use super::{
    AgentId, AppState, GoalError, HolderKind, IdentityId, ServerError, Standing, TeamError,
    with_directory, with_runtime,
};
use super::{Holder, Live, Operation, goals, judged, now};
use crate::goals_store::{Deliver, Delivering};
use std::str::FromStr;

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

pub(crate) fn control_recipient(
    state: &AppState,
    holder: &Holder,
    responsible: &str,
    agent: &str,
    require_active: bool,
) -> Result<bool, ServerError> {
    let admitted = with_directory(state, |directory| {
        let id = AgentId::from_str(agent).map_err(|error| GoalError::Unavailable {
            reason: error.to_string(),
        })?;
        let record = directory
            .projection()?
            .record(IdentityId::Agent(id))
            .ok_or(ServerError::AgentNotVisible)?;
        if require_active && record.state() != lys_identity::LifecycleState::Active {
            return Ok(false);
        }
        Ok(match holder.kind {
            HolderKind::Agent => {
                holder.id == agent
                    && record
                        .responsible()
                        .is_some_and(|person| person.to_string() == responsible)
            }
            HolderKind::Team => true,
        })
    })?;
    if admitted && holder.kind == HolderKind::Team {
        crate::teams_api::with_teams(state, |store| {
            let team = store.team(&holder.id).ok_or(TeamError::Unknown)?;
            Ok(team.retired.is_none()
                && team.created.owner == responsible
                && team.members.iter().any(|member| member == agent)
                && !team.held.iter().any(|held| held.member == agent))
        })
    } else {
        Ok(admitted)
    }
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ResendBody {
    operation: String,
    prior: String,
    session: String,
}

pub(super) async fn resend(
    axum::extract::State(state): axum::extract::State<std::sync::Arc<AppState>>,
    headers: axum::http::HeaderMap,
    axum::extract::Path(goal): axum::extract::Path<String>,
    body: Result<axum::Json<ResendBody>, axum::extract::rejection::JsonRejection>,
) -> Result<axum::Json<serde_json::Value>, ServerError> {
    use crate::receipts_api::control::{
        managed_status, read_receipt, record_decision, service_target,
    };
    use lys_runner::operations::{OperationState, Reconciled, Reconciliation};
    let refused = |name: &str, words: &str| ServerError::Runner {
        refusal: name.to_owned(),
        words: words.to_owned(),
    };
    let actor = super::signed_in(&state, &headers)?;
    let person = with_directory(&state, |directory| {
        super::own_person(directory.projection()?, &actor)
    })?
    .to_string();
    let axum::Json(body) = body.map_err(|error| super::malformed(error.body_text()))?;
    lys_identity::OperationId::from_str(&body.operation)
        .map_err(|error| super::malformed(error.to_string()))?;
    if body.session.is_empty() || body.prior.is_empty() {
        return Err(super::malformed(
            "resend must name its prior operation and intended session",
        ));
    }
    let goals = goals(&state)?;
    let (resent, source, holder) = goals.with(|store| {
        let item = store.item(&goal).ok_or(GoalError::Unknown)?;
        if item.goal.responsible != person {
            return Err(GoalError::Unknown.into());
        }
        let holder = item.goal.holder.clone();
        let (resent, source) = store.prepare_resend(
            &goal,
            &body.prior,
            &body.operation,
            &body.session,
            &person,
            now(),
        )?;
        Ok((resent, source, holder))
    })?;
    let agent = with_runtime(&state, |store| {
        let tracked = store
            .session(&body.session)
            .ok_or(ServerError::RuntimeSessionUnknown)?;
        if tracked.stopped() {
            return Err(ServerError::RuntimeSessionUnknown);
        }
        tracked
            .agent
            .clone()
            .ok_or(ServerError::RuntimeSessionUnknown)
    })?;
    if !control_recipient(&state, &holder, &person, &agent, true)? {
        return Err(refused(
            "goal_authority_revoked",
            "the intended session is no longer an admitted recipient",
        ));
    }
    let driven = service_target(&state, &body.session)?;
    let status = managed_status(&state, &driven).await?;
    crate::budgets_act::resend_allowed(&state, &agent, &body.session, &status)?;
    let original = service_target(&state, &source)?;
    let prior = read_receipt(&state, &original, &body.prior).await?;
    if prior.state != OperationState::Uncertain
        || prior
            .reference
            .as_ref()
            .is_some_and(|reference| reference.goal != goal)
    {
        return Err(refused(
            "control_not_uncertain",
            "the prior receipt does not name uncertain delivery of this goal",
        ));
    }
    let choice = Reconciliation::Resent {
        occurrence: body.operation.clone(),
    };
    if prior.reconciled.as_ref().is_some_and(|kept| {
        kept.operation != body.operation || kept.by != person || kept.decision != choice
    }) {
        return Err(refused(
            "control_already_reconciled",
            "a different person decision is already recorded",
        ));
    }
    // Authority is re-read after runner IO, and the dispatcher judges it again before any write.
    if !control_recipient(&state, &holder, &person, &agent, true)? {
        return Err(refused(
            "goal_authority_revoked",
            "the intended recipient changed before the occurrence was recorded",
        ));
    }
    let fired = goals.with(|store| store.resend(resent))?;
    let decision = record_decision(
        &state,
        &original,
        &prior,
        Reconciled {
            operation: body.operation,
            by: person,
            at: u64::try_from(jiff::Timestamp::now().as_millisecond())
                .map_err(|error| super::malformed(error.to_string()))?,
            decision: choice,
        },
    )
    .await;
    goals.changed.notify_one();
    state.changes.signal()?;
    let decision = decision.map_err(|error| ServerError::Runner {
        refusal: "goal_resend_kept_reconciliation_unavailable".to_owned(),
        words: format!("occurrence {} is kept under its distinct delivery; the original receipt's personal decision is unconfirmed: {error}", fired.operation),
    })?;
    let sent = fired.sent.first().ok_or_else(|| {
        refused(
            "goal_resend_invalid",
            "the saved resend has no intended delivery",
        )
    })?;
    Ok(axum::Json(
        serde_json::json!({"goal":goal,"occurrence":fired.operation,"operation":sent.operation,"session":sent.session,"prior":prior.operation,"reconciliation":decision}),
    ))
}
