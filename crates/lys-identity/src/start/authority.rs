//! Who may start an agent, give its start again or withdraw it: the agent's
//! responsible person (ADR-003, ADR-011), a directory administrator, or a
//! caller holding a live `agent.start` grant on that agent (AGENTS-015 R1).
//!
//! In step 1 the administrator is the one the configured issuer and subject
//! admit (P9). The responsible person is admitted when step 1's admission
//! admits them; until then the directory's admission refuses them as it
//! refuses every other caller. A person or an agent holding a live grant of
//! `agent.start` on the agent is admitted by that grant: the person who
//! answers for the agent gave it, and every start under it is on the record
//! as the caller's. Anyone else is refused as `start_right_missing`, naming
//! the agent and the right they lack.

use crate::start::error::Refusal;
use crate::start::request::AgentRecord;

/// The directory's admission of callers.
pub trait Admission {
    /// Whether `caller` is a directory administrator.
    fn is_administrator(&self, caller: &str) -> bool;

    /// Whether the directory's admission admits `caller` at all.
    fn admits(&self, caller: &str) -> bool;

    /// Whether `caller` holds a live grant of `agent.start` on `agent`. An
    /// admission that reads no grants answers that it holds none.
    fn holds_start(&self, _caller: &str, _agent: &str) -> bool {
        false
    }
}

/// A caller admitted to start, give again or withdraw a start of one agent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Admitted {
    /// The caller, as the launch record and the withdrawal name them.
    pub caller: String,
}

/// Admit `caller` for `agent`, or refuse them as `start_right_missing`.
///
/// # Errors
/// - `Refusal::StartRightMissing` if the caller is neither an administrator, an admitted responsible person nor a holder of a live `agent.start` grant on the agent.
pub fn admit(
    admission: &dyn Admission,
    caller: &str,
    agent: &AgentRecord,
) -> Result<Admitted, Refusal> {
    let responsible = agent.responsible.as_deref() == Some(caller) && admission.admits(caller);
    if admission.is_administrator(caller) || responsible || admission.holds_start(caller, &agent.id)
    {
        return Ok(Admitted {
            caller: caller.to_owned(),
        });
    }
    Err(Refusal::StartRightMissing {
        agent: agent.id.clone(),
    })
}
