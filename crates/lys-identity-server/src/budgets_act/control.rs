//! Service authority is read at a current owned boundary, without holding a store across IO.

use std::collections::BTreeSet;
use std::str::FromStr;
use std::sync::Arc;

use lys_identity::{AgentId, IdentityId, LifecycleState};
use lys_runner::harness_control::{BoundaryReply, ContextDecision, ControlStatus};
use lys_runner::operations::{Operation, OperationRequest, OperationState, TextDigest};
use lys_runner::{Act, Answer};

use crate::budgets_api::with_budgets;
use crate::budgets_state::Standing;
use crate::error::ServerError;
use crate::routes::{AppState, with_directory};
use crate::runtime_api::with_runtime;

fn refused(reason: impl Into<String>) -> ServerError {
    ServerError::Runner {
        refusal: "control_authority_unavailable".to_owned(),
        words: reason.into(),
    }
}

fn standing(state: &AppState, agent: &str) -> Result<Standing, ServerError> {
    let agent_id = AgentId::from_str(agent)
        .map_err(|error| refused(format!("tracked control agent: {error}")))?;
    with_directory(state, |directory| {
        let record = directory
            .projection()?
            .record(IdentityId::Agent(agent_id))
            .ok_or(ServerError::AgentNotVisible)?;
        if record.state() != LifecycleState::Active {
            return Err(ServerError::AgentNotVisible);
        }
        // Context limits cannot be set on teams; the holder's responsible person still applies.
        Ok(Standing {
            agent: agent.to_owned(),
            person: record.responsible().map(|person| person.to_string()),
            teams: BTreeSet::new(),
        })
    })
}

fn decision(
    state: &AppState,
    agent: &str,
    session: &str,
    status: &ControlStatus,
) -> Result<BoundaryReply, ServerError> {
    let at_ms = jiff::Timestamp::now().as_millisecond();
    let standing = standing(state, agent)?;
    let context = with_budgets(state, |store| {
        store
            .held()
            .control_context(&standing, status, session, at_ms)
            .map_err(refused)
    })?;
    let at = u64::try_from(at_ms.div_euclid(1000))
        .map_err(|error| refused(format!("control dispatch instant: {error}")))?;
    let reminders = crate::goals_api::boundary_reminders(state, session, &status.queued, at)?;
    Ok(BoundaryReply {
        generation: status.generation,
        boundary: status.boundary.clone(),
        context,
        reminders,
    })
}

/// Read current identifiers and ask the existing owners to judge one boundary.
pub(crate) async fn review(
    state: &Arc<AppState>,
    session: &str,
    cause: &str,
) -> Result<(), ServerError> {
    review_with(state, session, cause, None).await
}

async fn review_with(
    state: &Arc<AppState>,
    session: &str,
    cause: &str,
    unavailable: Option<&str>,
) -> Result<(), ServerError> {
    let (agent, machine) = with_runtime(state, |store| {
        let tracked = store
            .session(session)
            .ok_or(ServerError::RuntimeSessionUnknown)?;
        let agent = tracked
            .agent
            .as_ref()
            .ok_or(ServerError::RuntimeSessionUnknown)?;
        Ok((agent.clone(), tracked.machine.clone()))
    })?;
    let runner = crate::runner_sessions::machine_runner(state, &machine)?.ok_or_else(|| {
        ServerError::RunnerAbsent {
            machine: machine.clone(),
        }
    })?;
    let answer = crate::runner_client::ask(
        state,
        &machine,
        runner.clone(),
        Act::ControlStatus {
            session: session.to_owned(),
        },
    )
    .await?;
    let control = match answer {
        Answer::ControlStatus {
            session: answered,
            control,
        } if answered == session => control,
        other => {
            return Err(refused(format!(
                "runner answered {} to a control status read",
                crate::runner_sessions::kind(&other)
            )));
        }
    };
    let Some(control) = control else {
        return Ok(());
    };
    if unavailable.is_none() && control.boundary.is_none() {
        return Ok(());
    }
    let reply = match unavailable {
        Some(reason) => BoundaryReply {
            generation: control.generation,
            boundary: control.boundary.clone(),
            context: ContextDecision::Unavailable {
                reason: reason.to_owned(),
            },
            reminders: Vec::new(),
        },
        None => match decision(state, &agent, session, &control) {
            Ok(reply) => reply,
            Err(error) => BoundaryReply {
                generation: control.generation,
                boundary: control.boundary.clone(),
                context: ContextDecision::Unavailable {
                    reason: error.to_string(),
                },
                reminders: Vec::new(),
            },
        },
    };
    let encoded = serde_json::to_string(&reply)
        .map_err(|error| refused(format!("control reply encoding: {error}")))?;
    let digest = TextDigest::of(&encoded);
    let operation = Operation {
        operation: format!("control-{session}-{cause}-{}", digest.sha256),
        session: session.to_owned(),
        request: OperationRequest::BoundaryReply { reply },
    };
    match crate::runner_client::ask(state, &machine, runner, Act::Operate { operation }).await? {
        Answer::Operation { outcome } if outcome.state == OperationState::Confirmed => Ok(()),
        Answer::Operation { outcome } => Err(refused(format!(
            "boundary reply {}: {}",
            outcome.operation, outcome.words
        ))),
        other => Err(refused(format!(
            "runner answered {} to a boundary reply",
            crate::runner_sessions::kind(&other)
        ))),
    }
}

fn live(state: &AppState, machine: Option<&str>) -> Result<Vec<String>, ServerError> {
    if state.runtime.is_none() {
        return Ok(Vec::new());
    }
    with_runtime(state, |store| {
        Ok(store
            .live_ordered(std::ops::Bound::Unbounded)
            .filter(|tracked| machine.is_none_or(|machine| tracked.machine == machine))
            .map(|tracked| tracked.session.clone())
            .collect())
    })
}

/// Current live sessions are recovered once; no dead pipe is resumed.
pub(crate) async fn start(state: &Arc<AppState>) -> Result<(), ServerError> {
    for session in live(state, None)? {
        if let Err(error) = review(state, &session, "service-start").await {
            (state.say)(&format!(
                "controls: boundary recovery for session {session} at start failed: {error}"
            ));
        }
    }
    Ok(())
}

/// A feed end names the refusal on held words without releasing a turn.
pub(crate) async fn feed_end(
    state: &Arc<AppState>,
    machine: &str,
    error: &ServerError,
) -> Result<(), ServerError> {
    let reason = format!("control_feed_ended: {error}");
    let mut failures = Vec::new();
    for session in live(state, Some(machine))? {
        if let Err(error) = review_with(state, &session, "feed-ended", Some(&reason)).await {
            failures.push(format!("session {session}: {error}"));
        }
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(refused(failures.join("; ")))
    }
}

pub(crate) fn resend_allowed(
    state: &AppState,
    agent: &str,
    session: &str,
    status: &ControlStatus,
) -> Result<(), ServerError> {
    let standing = standing(state, agent)?;
    let context = with_budgets(state, |store| {
        store
            .held()
            .control_context(
                &standing,
                status,
                session,
                jiff::Timestamp::now().as_millisecond(),
            )
            .map_err(refused)
    })?;
    match context {
        ContextDecision::Released => Ok(()),
        ContextDecision::Compact { .. } => Err(refused(
            "context_compaction_pending: the intended session must compact before another occurrence",
        )),
        ContextDecision::Held { reason, .. } | ContextDecision::Unavailable { reason } => {
            Err(refused(reason))
        }
    }
}
