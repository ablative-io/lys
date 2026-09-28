//! The five named checks a start runs before any command is given.
//!
//! Each check reads its owner's record through its own seam and keeps no
//! copy of it: the agent is active reads the lifecycle state beside the
//! identity; its profile version is reviewed and the machine is allowed for
//! the role read the roles card `Ink1H1Os`'s record; its virtual credentials
//! are valid reads the handle record SECRETS-002 keeps; the machine may
//! reach what the profile needs reads the network record of CONFORMANCE row
//! 8.5. Every check runs and every result is returned by its name. A record
//! that does not exist is answered `check_record_missing`, naming the check
//! and the card that makes the record, and never a default, a sample record
//! or a pass.

use crate::start::error::{Refusal, StartError, json_string};
use crate::start::give::Owners;
use crate::start::request::StartRequest;
use crate::start::{active, credentials, egress, machine_role, profile_review};

/// One of the five checks, in the order a start runs them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Check {
    /// The agent's lifecycle state is active.
    AgentIsActive,
    /// The requested profile version has a review on record.
    ProfileVersionIsReviewed,
    /// The machine is one the agent's role may run on.
    MachineIsAllowedForRole,
    /// The agent holds valid virtual credentials.
    VirtualCredentialsAreValid,
    /// The machine's egress list holds every destination the profile needs.
    MachineMayReach,
}

impl Check {
    /// The five checks, in the order a start runs them.
    pub const ALL: [Self; 5] = [
        Self::AgentIsActive,
        Self::ProfileVersionIsReviewed,
        Self::MachineIsAllowedForRole,
        Self::VirtualCredentialsAreValid,
        Self::MachineMayReach,
    ];

    /// The check's name, in words.
    pub fn name(self) -> &'static str {
        match self {
            Self::AgentIsActive => "the agent is active",
            Self::ProfileVersionIsReviewed => "its profile version is reviewed",
            Self::MachineIsAllowedForRole => "the machine is allowed for the role",
            Self::VirtualCredentialsAreValid => "its virtual credentials are valid",
            Self::MachineMayReach => "the machine may reach what the profile needs",
        }
    }

    /// The card that makes the record the check reads.
    pub fn card(self) -> &'static str {
        match self {
            Self::AgentIsActive => "DIRECTORY-003",
            Self::ProfileVersionIsReviewed | Self::MachineIsAllowedForRole => "Ink1H1Os",
            Self::VirtualCredentialsAreValid => "SECRETS-002",
            Self::MachineMayReach => "network row 8.5",
        }
    }

    /// The refusal for this check's record not existing.
    pub fn record_missing(self) -> Refusal {
        Refusal::CheckRecordMissing {
            check: self,
            card: self.card(),
        }
    }
}

/// One check and its result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckReport {
    /// The check.
    pub check: Check,
    /// Why it did not pass; `None` when it passed.
    pub refusal: Option<Refusal>,
}

impl CheckReport {
    /// Whether the check passed.
    pub fn passed(&self) -> bool {
        self.refusal.is_none()
    }

    /// `passed`, or the refusal's name.
    pub fn result(&self) -> &'static str {
        self.refusal.as_ref().map_or("passed", Refusal::name)
    }

    /// The result in words, naming the check.
    pub fn words(&self) -> String {
        match &self.refusal {
            None => format!("{}: passed", self.check.name()),
            Some(refusal) => refusal.to_string(),
        }
    }
}

/// What the five checks answered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Checked {
    /// Every check, in order, with its result.
    pub reports: Vec<CheckReport>,
    /// The credential ids the credentials check handed on; empty unless it
    /// passed.
    pub credential_ids: Vec<String>,
}

impl Checked {
    /// Whether every check passed.
    pub fn passed(&self) -> bool {
        self.reports.iter().all(CheckReport::passed)
    }

    /// The refusal of every check that did not pass, in order.
    pub fn refusals(&self) -> Vec<Refusal> {
        self.reports
            .iter()
            .filter_map(|report| report.refusal.clone())
            .collect()
    }
}

/// Run all five checks for `request` of `agent`, each against its owner's
/// record. Fails only when a record could not be read at all.
pub fn run(
    owners: &Owners<'_>,
    agent: &str,
    request: &StartRequest,
) -> Result<Checked, StartError> {
    let active = active::check(owners.lifecycles, agent).err();
    let reviewed = profile_review::check(owners.reviews, request.profile_version()).err();
    let allowed = machine_role::check(owners.role_machines, agent, request.machine()).err();
    let (credential_ids, credentials) = match credentials::check(owners.handles, agent)? {
        Ok(ids) => (ids, None),
        Err(refusal) => (Vec::new(), Some(refusal)),
    };
    let reachable = egress::check(
        owners.needs,
        owners.egress,
        request.profile_version(),
        request.machine(),
    )
    .err();
    let reports = Check::ALL
        .into_iter()
        .zip([active, reviewed, allowed, credentials, reachable])
        .map(|(check, refusal)| CheckReport { check, refusal })
        .collect();
    Ok(Checked {
        reports,
        credential_ids,
    })
}

/// Append `reports` to `out` as a JSON array.
pub(crate) fn checks_json(out: &mut String, reports: &[CheckReport]) {
    out.push('[');
    for (at, report) in reports.iter().enumerate() {
        if at > 0 {
            out.push(',');
        }
        out.push_str("{\"check\":");
        json_string(out, report.check.name());
        out.push_str(",\"result\":");
        json_string(out, report.result());
        out.push_str(",\"words\":");
        json_string(out, &report.words());
        out.push('}');
    }
    out.push(']');
}
