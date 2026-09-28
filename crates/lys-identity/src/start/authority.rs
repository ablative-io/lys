//! Who may start an agent, give its start again or withdraw it: the agent's
//! responsible person (ADR-003, ADR-011) or a directory administrator.
//!
//! In step 1 the administrator is the one the configured issuer and subject
//! admit (P9). The responsible person is admitted when step 1's admission
//! admits them; until then the directory's admission refuses them as it
//! refuses every other caller. Anyone else is refused as
//! `start_right_missing`, naming the agent and the right they lack. No grant
//! held by anyone else admits a start: that grant is a further unit and is
//! not read here.

use crate::start::error::Refusal;
use crate::start::request::AgentRecord;

/// The directory's admission of callers.
pub trait Admission {
    /// Whether `caller` is a directory administrator.
    fn is_administrator(&self, caller: &str) -> bool;

    /// Whether the directory's admission admits `caller` at all.
    fn admits(&self, caller: &str) -> bool;
}

/// A caller admitted to start, give again or withdraw a start of one agent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Admitted {
    /// The caller, as the launch record and the withdrawal name them.
    pub caller: String,
}

/// Admit `caller` for `agent`, or refuse them as `start_right_missing`.
pub fn admit(
    admission: &dyn Admission,
    caller: &str,
    agent: &AgentRecord,
) -> Result<Admitted, Refusal> {
    let responsible = agent.responsible.as_deref() == Some(caller) && admission.admits(caller);
    if admission.is_administrator(caller) || responsible {
        return Ok(Admitted {
            caller: caller.to_owned(),
        });
    }
    Err(Refusal::StartRightMissing {
        agent: agent.id.clone(),
    })
}
