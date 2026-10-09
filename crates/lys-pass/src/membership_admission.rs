//! Scoped guest admission (ACCESS-006 R4): a subject is admitted to a
//! workspace when it holds at least one current grant within it, and
//! refused [`NO_CURRENT_GRANT`] when it holds none.
//!
//! Admission is an existence decision, not a right: it opens no channel,
//! and every act after it still needs its own read or post decision. It is
//! never inferred from a Lys account or a sign-in, and an agent is asked
//! about as its own identity, never as its responsible person.

use serde::{Deserialize, Serialize};

use crate::membership::{CONTRACT_VERSION, GrantLog, Refused, Subject, Verdict, check};
use crate::rights::Resource;

/// The subject holds no current grant within the workspace.
pub const NO_CURRENT_GRANT: &str = "membership_no_current_grant";

/// Whether `subject` may be admitted to `workspace`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema), schema(as = MembershipAdmissionRequest))]
pub struct AdmissionRequest {
    /// The contract version the asker speaks.
    pub contract: u32,
    /// The grant log the asker's revisions belong to.
    pub log: GrantLog,
    /// The workspace.
    pub workspace: Resource,
    /// Who is asked about.
    pub subject: Subject,
    /// The least receipt revision the decision must reflect.
    pub at_least: u64,
}

/// The answer to an admission question: allowed by a current grant within
/// the workspace, with the scope it is held on, or refused by name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema), schema(as = MembershipAdmissionDecision))]
pub struct AdmissionDecision {
    /// The contract version answered under.
    pub contract: u32,
    /// The grant log decided from.
    pub log: GrantLog,
    /// The exact request answered.
    pub request: AdmissionRequest,
    /// The revision decided at; absent when refused before the grants were
    /// asked.
    pub revision: Option<u64>,
    /// The decision.
    pub verdict: Verdict,
}

/// Refuse an admission request this contract cannot ask.
pub fn validate(request: &AdmissionRequest, served: &GrantLog) -> Result<(), Refused> {
    check(
        request.contract,
        &request.log,
        served,
        &[
            ("workspace kind", request.workspace.kind.as_str()),
            ("workspace id", request.workspace.id.as_str()),
            ("subject id", request.subject.id.as_str()),
            ("subject kind", request.subject.kind.as_str()),
        ],
    )
}

/// The admission refused by `name` before the grants were asked.
#[must_use]
pub fn refused_before_lookup(
    request: &AdmissionRequest,
    served: &GrantLog,
    name: &str,
    reason: &str,
) -> AdmissionDecision {
    AdmissionDecision {
        contract: CONTRACT_VERSION,
        log: served.clone(),
        request: request.clone(),
        revision: None,
        verdict: Verdict::Refused {
            refusal: name.to_owned(),
            reason: reason.to_owned(),
        },
    }
}
