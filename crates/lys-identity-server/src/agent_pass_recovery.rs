//! Reconcile durable endings before any route can admit a restored run pass.
use crate::error::ServerError;
use crate::routes::{AppState, start::StartService, with_directory};
use crate::runtime_api::with_runtime;
use lys_identity::{AgentId, IdentityId, LifecycleState};
use std::collections::HashSet;

pub(crate) fn withdraw(
    state: Option<&AppState>,
    launch: &str,
) -> Result<(), lys_identity::start::StartError> {
    let state = state.ok_or_else(|| lys_identity::start::StartError::Unavailable {
        reason: "agent pass store is not attached to launch withdrawal".to_owned(),
    })?;
    crate::agent_pass::end_launch(state, launch).map_err(|error| {
        lys_identity::start::StartError::Unavailable {
            reason: error.to_string(),
        }
    })
}

pub(crate) fn at_start(state: &AppState, starts: &StartService) -> Result<(), ServerError> {
    with_directory(state, |directory| {
        let projection = directory.projection()?;
        let mut launches = starts
            .launches()
            .map_err(|error| ServerError::AgentPassRefused {
                reason: format!("launch records unavailable during pass reconciliation: {error}"),
            })?;
        launches
            .settle()
            .map_err(|error| ServerError::AgentPassRefused {
                reason: format!("launch records unavailable during pass reconciliation: {error}"),
            })?;
        let reconcile = |runtime: Option<&crate::runtime_store::RuntimeStore>| {
            let mut passes = crate::agent_pass::store(state)?;
            let candidates = passes.launches();
            let withdrawn: HashSet<&str> = launches
                .withdrawals()
                .iter()
                .filter(|withdrawal| candidates.contains(withdrawal.launch_record.as_str()))
                .map(|withdrawal| withdrawal.launch_record.as_str())
                .collect();
            drop(candidates);
            passes.reconcile(|agent, launch, session| {
                let active = agent
                    .parse::<AgentId>()
                    .ok()
                    .and_then(|id| projection.record(IdentityId::Agent(id)))
                    .is_some_and(|record| record.state() == LifecycleState::Active);
                active
                    && !withdrawn.contains(launch)
                    && runtime
                        .and_then(|store| store.session(session))
                        .is_some_and(|tracked| {
                            tracked.agent.as_deref() == Some(agent)
                                && !tracked.stopped()
                                && tracked.stop_asked_at().is_none()
                        })
            })
        };
        if state.runtime.is_none() {
            reconcile(None)
        } else {
            with_runtime(state, |runtime| reconcile(Some(runtime)))
        }
    })
}
