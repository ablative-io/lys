//! The machine may reach what the profile needs: every destination the
//! profile version needs is on the machine's egress list.
//!
//! What the profile needs is read from the profile version's record, and the
//! egress list from the network record of CONFORMANCE row 8.5. A destination
//! missing from the list fails as `egress_not_reachable`, naming each one.
//! While row 8.5's record does not exist the check answers
//! `check_record_missing` naming network row 8.5, and open egress is never
//! assumed; while the profile version's record of what it needs does not
//! exist it answers the same, naming `Ink1H1Os`.

use crate::start::checks::Check;
use crate::start::error::Refusal;

/// The profile version's record of the destinations it needs.
pub trait ProfileNeeds {
    /// The destinations `profile_version` needs, or `None` when its record
    /// does not exist.
    fn needs(&self, profile_version: &str) -> Option<Vec<String>>;
}

/// The network record of each machine's egress list.
pub trait EgressLists {
    /// The destinations `machine` may reach, or `None` when the record does
    /// not exist.
    fn egress(&self, machine: &str) -> Option<Vec<String>>;
}

/// The machine may reach what the profile needs.
pub fn check(
    needs: &dyn ProfileNeeds,
    lists: &dyn EgressLists,
    profile_version: &str,
    machine: &str,
) -> Result<(), Refusal> {
    let Some(reachable) = lists.egress(machine) else {
        return Err(Check::MachineMayReach.record_missing());
    };
    let Some(needed) = needs.needs(profile_version) else {
        return Err(Refusal::CheckRecordMissing {
            check: Check::MachineMayReach,
            card: "Ink1H1Os",
        });
    };
    let destinations: Vec<String> = needed
        .into_iter()
        .filter(|destination| !reachable.contains(destination))
        .collect();
    if destinations.is_empty() {
        return Ok(());
    }
    Err(Refusal::EgressNotReachable {
        machine: machine.to_owned(),
        destinations,
    })
}
