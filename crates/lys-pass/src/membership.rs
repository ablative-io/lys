//! The one shared channel membership contract (ACCESS-006 R1).
//!
//! A membership is effective scoped authority, a read or a post on one
//! placed resource, judged by Lys's one grant authority at a named revision;
//! it is never a product-local roster, a role flag or a consequence of
//! signing in. Every product asks with a [`MembershipRequest`] and receives
//! a [`MembershipDecision`] that echoes the exact request it answers, the
//! grant log and revision it was decided at, and either the granting
//! receipt and effective scope or a refusal by its stable name.
//!
//! Read and post are separate actions: an answer about one says nothing of
//! the other. No display label, follow, mute, tag, readable ancestor or
//! cursor is a member of the request, and unknown members are refused, so
//! none can be sent as a claim of membership. The JSON member names below
//! are the contract; a change to them is a new [`CONTRACT_VERSION`].

use serde::{Deserialize, Serialize};

use crate::rights::{Mode, Resource};

/// The contract version this crate speaks. A request naming another is
/// refused [`CONTRACT_UNSUPPORTED`] before any lookup.
pub const CONTRACT_VERSION: u32 = 1;

/// The request names a contract version the provider does not serve.
pub const CONTRACT_UNSUPPORTED: &str = "membership_contract_unsupported";
/// The request names another grant log, or another reset epoch of it; its
/// revisions are not comparable with the provider's.
pub const LOG_MISMATCH: &str = "membership_log_mismatch";
/// A member of the request is empty or contains control characters.
pub const REQUEST_MALFORMED: &str = "membership_request_malformed";
/// The subject's stated kind is not the kind its identity names.
pub const SUBJECT_KIND_MISMATCH: &str = "membership_subject_kind_mismatch";
/// The resource is not placed under the named workspace.
pub const OUTSIDE_WORKSPACE: &str = "membership_outside_workspace";

/// The grant log a revision belongs to. Revisions from different
/// identities, or from different epochs of one identity, never compare.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GrantLog {
    /// The log's stable identity, fixed when the log was created.
    pub identity: String,
    /// The reset epoch: a deliberate replacement of the log moves it.
    pub epoch: u64,
}

/// The verified identity a membership is asked about.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Subject {
    /// The directory identity id, as verified by the product.
    pub id: String,
    /// The identity's kind: `person`, `agent`, `service_account` or
    /// `connector`. An AI is asked about as its own agent identity, never
    /// as its responsible person.
    pub kind: String,
}

/// One membership question, every member explicit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MembershipRequest {
    /// The contract version the asker speaks.
    pub contract: u32,
    /// The grant log the asker's revisions belong to.
    pub log: GrantLog,
    /// The workspace the resource must be placed under.
    pub workspace: Resource,
    /// Who is asked about.
    pub subject: Subject,
    /// The channel or other placed resource asked about.
    pub resource: Resource,
    /// The action, `read` or `post` as the product's schema declares them.
    pub action: String,
    /// The least receipt revision the decision must reflect: the revision
    /// of the last grant change the asker observed, or 0 for none.
    pub at_least: u64,
}

/// The answer to one membership question.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MembershipDecision {
    /// The contract version the provider answered under.
    pub contract: u32,
    /// The grant log the provider decided from.
    pub log: GrantLog,
    /// The exact request answered.
    pub request: MembershipRequest,
    /// The grant revision the authority decided at; absent only when the
    /// request was refused before the authority was asked.
    pub revision: Option<u64>,
    /// The decision.
    pub verdict: Verdict,
}

/// Allowed, held for approval, or refused by name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case", deny_unknown_fields)]
pub enum Verdict {
    /// The subject may take the action now.
    Allowed {
        /// The grant whose authority allows it.
        grant: String,
        /// That grant and every ancestor, to its root.
        path: Vec<String>,
        /// The resource whose grant allows it: the one asked about, or a
        /// workspace or parent it is placed in whose schema lets it flow.
        scope: Resource,
    },
    /// A grant reaches the action, but it is exercised only by approval.
    Held {
        /// The grant that reaches it.
        grant: String,
        /// The resource whose grant reaches it.
        scope: Resource,
        /// The approval the grant requires.
        mode: Mode,
    },
    /// The action is refused.
    Refused {
        /// The stable refusal name.
        refusal: String,
        /// The refusal's words, naming no resource outside the asker's scope.
        reason: String,
    },
}

/// A request refused before any lookup, by its stable name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refused {
    /// The stable refusal name.
    pub name: &'static str,
    /// The refusal's words.
    pub reason: String,
}

fn malformed(value: &str) -> bool {
    value.is_empty() || value.chars().any(char::is_control)
}

/// Refuse a request this contract cannot ask, in the order a provider
/// checks: the contract version, the grant log `served`, then every member's
/// shape. Identity, kind and placement are the provider's to judge after.
pub fn validate(request: &MembershipRequest, served: &GrantLog) -> Result<(), Refused> {
    if request.contract != CONTRACT_VERSION {
        return Err(Refused {
            name: CONTRACT_UNSUPPORTED,
            reason: format!(
                "contract {} is not served; this provider speaks {CONTRACT_VERSION}",
                request.contract
            ),
        });
    }
    if &request.log != served {
        return Err(Refused {
            name: LOG_MISMATCH,
            reason: "the request names another grant log or reset epoch".to_owned(),
        });
    }
    let members = [
        ("log identity", request.log.identity.as_str()),
        ("workspace kind", request.workspace.kind.as_str()),
        ("workspace id", request.workspace.id.as_str()),
        ("subject id", request.subject.id.as_str()),
        ("subject kind", request.subject.kind.as_str()),
        ("resource kind", request.resource.kind.as_str()),
        ("resource id", request.resource.id.as_str()),
        ("action", request.action.as_str()),
    ];
    if let Some((member, _)) = members.iter().find(|(_, value)| malformed(value)) {
        return Err(Refused {
            name: REQUEST_MALFORMED,
            reason: format!("the {member} is empty or contains control characters"),
        });
    }
    Ok(())
}

/// The decision refusing `request` by `name` before the authority was
/// asked: it names no revision, and echoes the request it answers.
#[must_use]
pub fn refused_before_lookup(
    request: &MembershipRequest,
    served: &GrantLog,
    name: &str,
    reason: &str,
) -> MembershipDecision {
    MembershipDecision {
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
