//! Boundary authority reads current saved words and current recipient admission.

use super::{
    AgentId, AppState, GoalError, HolderKind, IdentityId, ServerError, TeamError, with_directory,
    with_runtime,
};
use super::{Holder, goals, now};
use std::str::FromStr;

mod delivery;
pub(crate) use delivery::boundary_reminders;

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
        let projection = directory.projection()?;
        let record = projection
            .record(IdentityId::Agent(id))
            .ok_or(ServerError::AgentNotVisible)?;
        if require_active {
            let person = lys_identity::PersonId::from_str(responsible).map_err(|error| {
                GoalError::Unavailable {
                    reason: error.to_string(),
                }
            })?;
            let authority = ControlLifecycle {
                agent: record.state(),
                person: projection
                    .record(IdentityId::Person(person))
                    .map(lys_identity::projection::Record::state),
            };
            if !live_control_authority(authority) {
                return Ok(false);
            }
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
            Ok(team_control_recipient(team, responsible, agent))
        })
    } else {
        Ok(admitted)
    }
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = GoalResendBody)]
pub(crate) struct ResendBody {
    operation: String,
    prior: String,
    session: String,
}

#[derive(serde::Serialize, utoipa::ToSchema)]
#[schema(as = GoalResendView)]
pub(crate) struct ResendView {
    goal: String,
    occurrence: String,
    operation: String,
    session: String,
    prior: String,
    reconciliation: lys_runner::operations::ControlReceipt,
}

pub(super) async fn resend(
    axum::extract::State(state): axum::extract::State<std::sync::Arc<AppState>>,
    headers: axum::http::HeaderMap,
    axum::extract::Path(goal): axum::extract::Path<String>,
    body: Result<axum::Json<ResendBody>, axum::extract::rejection::JsonRejection>,
) -> Result<axum::Json<ResendView>, ServerError> {
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
    let (resent, source, holder, kept) = goals.with(|store| {
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
        let kept = store.resent_occurrence(&body.prior).is_some();
        Ok((resent, source, holder, kept))
    })?;
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
    let fired = if kept {
        crate::receipts_api::control::responsible(&state, &original, &prior, &person)?;
        resent.fired
    } else {
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
        // Authority is re-read after runner IO, and the dispatcher judges it again before any write.
        if !control_recipient(&state, &holder, &person, &agent, true)? {
            return Err(refused(
                "goal_authority_revoked",
                "the intended recipient changed before the occurrence was recorded",
            ));
        }
        goals.with(|store| store.resend(resent))?
    };
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
    Ok(axum::Json(ResendView {
        goal,
        occurrence: fired.operation,
        operation: sent.operation.clone(),
        session: sent.session.clone(),
        prior: prior.operation,
        reconciliation: decision,
    }))
}

#[derive(serde::Serialize, utoipa::ToSchema)]
#[schema(as = GoalResendOccurrence)]
pub(crate) struct ResendOccurrence {
    operation: String,
    session: String,
}

#[derive(serde::Serialize, utoipa::ToSchema)]
#[schema(as = GoalResendLookup)]
pub(crate) struct ResendLookup {
    goal: String,
    prior: String,
    occurrence: Option<ResendOccurrence>,
}

pub(super) async fn resend_lookup(
    axum::extract::State(state): axum::extract::State<std::sync::Arc<AppState>>,
    headers: axum::http::HeaderMap,
    axum::extract::Path((goal, prior)): axum::extract::Path<(String, String)>,
) -> Result<axum::Json<ResendLookup>, ServerError> {
    let actor = super::signed_in(&state, &headers)?;
    let person = with_directory(&state, |directory| {
        super::own_person(directory.projection()?, &actor)
    })?
    .to_string();
    let goals = goals(&state)?;
    let holder = goals.with(|store| {
        let item = store.item(&goal).ok_or(GoalError::Unknown)?;
        if item.goal.responsible != person {
            return Err(GoalError::Unknown.into());
        }
        Ok(item.goal.holder.clone())
    })?;
    let admitted = match holder.kind {
        HolderKind::Agent => control_recipient(&state, &holder, &person, &holder.id, false)?,
        HolderKind::Team => crate::teams_api::with_teams(&state, |store| {
            let team = store.team(&holder.id).ok_or(TeamError::Unknown)?;
            Ok(team.retired.is_none() && team.created.owner == person)
        })?,
    };
    if !admitted {
        return Err(GoalError::Unknown.into());
    }
    let occurrence = goals.with(|store| {
        let item = store.item(&goal).ok_or(GoalError::Unknown)?;
        if item.goal.responsible != person {
            return Err(GoalError::Unknown.into());
        }
        store
            .resend_lookup(&goal, &prior)?
            .map(|fired| {
                let sent = fired.sent.first().ok_or_else(|| ServerError::Runner {
                    refusal: "goal_resend_invalid".to_owned(),
                    words: "the kept occurrence has no intended delivery".to_owned(),
                })?;
                Ok(ResendOccurrence {
                    operation: fired.operation.clone(),
                    session: sent.session.clone(),
                })
            })
            .transpose()
    })?;
    Ok(axum::Json(ResendLookup {
        goal,
        prior,
        occurrence,
    }))
}

pub(crate) fn team_control_recipient(
    team: &crate::teams_state::Team,
    responsible: &str,
    agent: &str,
) -> bool {
    team.retired.is_none()
        && team.created.owner == responsible
        && team.members.iter().any(|member| {
            #[cfg(test)]
            crate::budgets_feed::membership_probe::member();
            member == agent
        })
        && !team.held.iter().any(|held| {
            #[cfg(test)]
            crate::budgets_feed::membership_probe::held();
            held.member == agent
        })
}

#[derive(Clone, Copy)]
struct ControlLifecycle {
    agent: lys_identity::LifecycleState,
    person: Option<lys_identity::LifecycleState>,
}
fn live_control_authority(authority: ControlLifecycle) -> bool {
    authority.agent == lys_identity::LifecycleState::Active
        && authority.person == Some(lys_identity::LifecycleState::Active)
}

#[cfg(test)]
mod authority_tests {
    use super::{ControlLifecycle, live_control_authority};
    use lys_identity::LifecycleState;
    #[test]
    fn pending_words_are_refused_when_the_responsible_person_is_no_longer_active() {
        assert!(live_control_authority(ControlLifecycle {
            agent: LifecycleState::Active,
            person: Some(LifecycleState::Active)
        }));
        for person in [
            Some(LifecycleState::Suspended),
            Some(LifecycleState::Retired),
            Some(LifecycleState::Registered),
            None,
        ] {
            assert!(!live_control_authority(ControlLifecycle {
                agent: LifecycleState::Active,
                person
            }));
        }
        assert!(!live_control_authority(ControlLifecycle {
            agent: LifecycleState::Suspended,
            person: Some(LifecycleState::Active)
        }));
    }
}
