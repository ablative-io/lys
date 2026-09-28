//! The machine is allowed for the role: read from the role's machines in the
//! roles card `Ink1H1Os`'s record.
//!
//! The machine is identified by the identifier that record gives it. A
//! machine the agent's roles may not run on fails as
//! `machine_not_allowed_for_role`, naming the machine and the roles. While
//! `Ink1H1Os`'s record does not exist the check answers `check_record_missing`
//! and no machine list is taken from any other source.

use crate::start::checks::Check;
use crate::start::error::Refusal;

/// A role an agent holds, with the machines the role may run on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeldRole {
    /// The role, by the identifier `Ink1H1Os`'s record gives it.
    pub role: String,
    /// The machines the role may run on, by the identifiers that record gives them.
    pub machines: Vec<String>,
}

/// The roles card's record of each role's machines.
pub trait RoleMachines {
    /// The roles `agent` holds with their machines, or `None` when the
    /// record does not exist.
    fn roles(&self, agent: &str) -> Option<Vec<HeldRole>>;
}

/// The machine is allowed for the role: passes when a role `agent` holds
/// may run on `machine`.
pub fn check(records: &dyn RoleMachines, agent: &str, machine: &str) -> Result<(), Refusal> {
    let Some(roles) = records.roles(agent) else {
        return Err(Check::MachineIsAllowedForRole.record_missing());
    };
    if roles
        .iter()
        .any(|held| held.machines.iter().any(|allowed| allowed == machine))
    {
        return Ok(());
    }
    Err(Refusal::MachineNotAllowedForRole {
        machine: machine.to_owned(),
        roles: roles.into_iter().map(|held| held.role).collect(),
    })
}
