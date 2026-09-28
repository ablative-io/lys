//! The agent is active: the one check that needs only the lifecycle record.
//!
//! It reads, live, the lifecycle state DIRECTORY-003 records beside each
//! identity, and passes only on active. Registered, suspended and retired
//! each fail as `agent_not_active`, naming the state. The state is read at
//! every start and never copied, so a suspension takes effect at the next
//! start.

use crate::lifecycle::LifecycleState;
use crate::start::checks::Check;
use crate::start::error::Refusal;

/// The lifecycle record: each identity's recorded state.
pub trait Lifecycles {
    /// The state recorded for `agent`, or `None` when no lifecycle record
    /// exists for it.
    fn state(&self, agent: &str) -> Option<LifecycleState>;
}

/// The agent is active: passes only when `agent`'s recorded state is active.
pub fn check(lifecycles: &dyn Lifecycles, agent: &str) -> Result<(), Refusal> {
    match lifecycles.state(agent) {
        None => Err(Check::AgentIsActive.record_missing()),
        Some(LifecycleState::Active) => Ok(()),
        Some(state) => Err(Refusal::AgentNotActive {
            state: state.to_string(),
        }),
    }
}
