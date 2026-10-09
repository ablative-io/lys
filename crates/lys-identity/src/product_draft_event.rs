//! A product's held act and its decisions (ACCESS-001 R3): the product
//! records the exact words of an act its grant holds by draft or by two,
//! people approve or refuse it, and the product's connector closes it as
//! executed or refused on execution. Lys never executes the act.
//!
//! These events are a kind of their own beside the operator drafts of
//! [`crate::draft_event`], in their own envelope, so neither is read as the
//! other. Every id here is derived, never chosen: a draft's id from its
//! holder and the product's own operation, and its close from the draft, so
//! a product retrying the same act or the same close finds what it recorded.

use crate::draft_event::Target;
use crate::encoding::payload_commitment;
use crate::grants::{GrantId, Mode};
use crate::id::ID_LEN;
use crate::{Actor, IdentityError, IdentityId, OperationId, PersonId};
use std::sync::Arc;

#[path = "product_draft_encoding.rs"]
mod encoding;
pub use encoding::{decode, encode};

/// The wire version of every product draft event; each kind is numbered
/// beside it, 0 creation, 1 approval, 2 refusal, 3 executed, 4 refused on
/// execution.
pub const VERSION: u64 = 1;

/// The domain separating a draft id from every other derived id.
const DRAFT_DOMAIN: &str = "lys/product-draft/v1";
/// The domain separating a draft's one close from every other derived id.
const CLOSE_DOMAIN: &str = "lys/product-draft/close/v1";

/// A product's held act, recorded with its exact words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Created {
    /// The draft id: [`draft_operation`] of the holder and `client_operation`.
    pub operation: OperationId,
    /// The product's own operation id, kept across its retries.
    pub client_operation: String,
    /// The grant's holder, who asked.
    pub holder: IdentityId,
    /// The person responsible for the holder, as the grant names them.
    pub responsible: PersonId,
    /// Creation time in seconds.
    pub recorded_at: u64,
    /// The app whose kind the target is.
    pub app: String,
    /// The held grant the act rests on.
    pub grant: GrantId,
    /// The grant's mode when the draft was recorded: by draft or by two.
    pub mode: Mode,
    /// The resource and action the act takes.
    pub target: Target,
    /// SHA-256 of the exact UTF-8 words.
    pub request_digest: [u8; 32],
    /// The exact words of the prepared act.
    pub words: String,
}

/// One person's approval of one draft.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Approved {
    /// The approval's operation.
    pub operation: OperationId,
    /// The signed-in person's session.
    pub actor: Actor,
    /// The person approving.
    pub approver: PersonId,
    /// Decision time in seconds.
    pub recorded_at: u64,
    /// The draft approved.
    pub draft: OperationId,
    /// SHA-256 of the draft's canonical creation payload.
    pub draft_hash: [u8; 32],
}

/// One person's refusal of one draft, with the reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refused {
    /// The refusal's operation.
    pub operation: OperationId,
    /// The signed-in person's session.
    pub actor: Actor,
    /// The person refusing.
    pub approver: PersonId,
    /// Decision time in seconds.
    pub recorded_at: u64,
    /// The draft refused.
    pub draft: OperationId,
    /// SHA-256 of the draft's canonical creation payload.
    pub draft_hash: [u8; 32],
    /// Why it is refused.
    pub reason: String,
}

/// The product's receipt that it executed an approved draft.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Executed {
    /// [`close_operation`] of the draft.
    pub operation: OperationId,
    /// Close time in seconds.
    pub recorded_at: u64,
    /// The draft closed.
    pub draft: OperationId,
    /// SHA-256 of the draft's canonical creation payload.
    pub draft_hash: [u8; 32],
    /// The app whose connector closed it.
    pub app: String,
    /// The product's committed receipt digest.
    pub receipt_digest: [u8; 32],
}

/// The product's refusal of an approved draft when it came to execute it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefusedOnExecution {
    /// [`close_operation`] of the draft.
    pub operation: OperationId,
    /// Close time in seconds.
    pub recorded_at: u64,
    /// The draft closed.
    pub draft: OperationId,
    /// SHA-256 of the draft's canonical creation payload.
    pub draft_hash: [u8; 32],
    /// The app whose connector closed it.
    pub app: String,
    /// The product's own name for the refusal.
    pub refusal: String,
    /// The product's words for why.
    pub reason: String,
}

/// A product draft's creation, a decision, or its close, in the directory log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProductDraftEvent {
    /// Record the held act.
    Created(Arc<Created>),
    /// Record one approval.
    Approved(Arc<Approved>),
    /// Refuse the draft.
    Refused(Arc<Refused>),
    /// Close it as executed.
    Executed(Arc<Executed>),
    /// Close it as refused on execution.
    RefusedOnExecution(Arc<RefusedOnExecution>),
}

fn invalid(reason: &'static str) -> IdentityError {
    IdentityError::DraftChangeInvalid { reason }
}

fn word(value: &str) -> bool {
    !value.is_empty() && !value.chars().any(char::is_control)
}

fn derived(domain: &str, parts: &[&[u8]]) -> OperationId {
    let mut material = domain.as_bytes().to_vec();
    for part in parts {
        material.push(0);
        material.extend_from_slice(part);
    }
    let digest = payload_commitment(&material);
    let mut bytes = [0u8; ID_LEN];
    bytes.copy_from_slice(&digest[..ID_LEN]);
    OperationId::from_bytes(bytes)
}

/// The draft id of `holder`'s act under the product's `client_operation`:
/// the same holder and operation always name the same draft, and two
/// holders never share one.
#[must_use]
pub fn draft_operation(holder: IdentityId, client_operation: &str) -> OperationId {
    derived(
        DRAFT_DOMAIN,
        &[holder.to_string().as_bytes(), client_operation.as_bytes()],
    )
}

/// The one operation that closes `draft`, as executed or refused on
/// execution: a draft is closed once, and a second close in other words is
/// refused as the reuse of this operation.
#[must_use]
pub fn close_operation(draft: OperationId) -> OperationId {
    derived(CLOSE_DOMAIN, &[draft.as_bytes()])
}

/// SHA-256 of `words`' exact UTF-8 bytes.
#[must_use]
pub fn words_digest(words: &str) -> [u8; 32] {
    payload_commitment(words.as_bytes())
}

impl ProductDraftEvent {
    /// The operation this leaf records.
    pub fn operation(&self) -> OperationId {
        match self {
            Self::Created(event) => event.operation,
            Self::Approved(event) => event.operation,
            Self::Refused(event) => event.operation,
            Self::Executed(event) => event.operation,
            Self::RefusedOnExecution(event) => event.operation,
        }
    }

    /// The draft a decision or close names, with the creation hash it binds.
    pub fn decision(&self) -> Option<(OperationId, [u8; 32])> {
        match self {
            Self::Created(_) => None,
            Self::Approved(event) => Some((event.draft, event.draft_hash)),
            Self::Refused(event) => Some((event.draft, event.draft_hash)),
            Self::Executed(event) => Some((event.draft, event.draft_hash)),
            Self::RefusedOnExecution(event) => Some((event.draft, event.draft_hash)),
        }
    }

    /// Whether `other` is the same act asked again: every member the caller
    /// chose is equal, and only the times and the session it arrived in may
    /// differ. A retry is answered as first recorded, never recorded twice.
    pub fn same_act(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Created(held), Self::Created(asked)) => {
                held.operation == asked.operation
                    && held.client_operation == asked.client_operation
                    && held.holder == asked.holder
                    && held.app == asked.app
                    && held.grant == asked.grant
                    && held.target == asked.target
                    && held.request_digest == asked.request_digest
                    && held.words == asked.words
            }
            (Self::Approved(held), Self::Approved(asked)) => {
                held.operation == asked.operation
                    && held.approver == asked.approver
                    && held.draft == asked.draft
                    && held.draft_hash == asked.draft_hash
            }
            (Self::Refused(held), Self::Refused(asked)) => {
                held.operation == asked.operation
                    && held.approver == asked.approver
                    && held.draft == asked.draft
                    && held.draft_hash == asked.draft_hash
                    && held.reason == asked.reason
            }
            (Self::Executed(held), Self::Executed(asked)) => {
                held.operation == asked.operation
                    && held.draft == asked.draft
                    && held.app == asked.app
                    && held.receipt_digest == asked.receipt_digest
            }
            (Self::RefusedOnExecution(held), Self::RefusedOnExecution(asked)) => {
                held.operation == asked.operation
                    && held.draft == asked.draft
                    && held.app == asked.app
                    && held.refusal == asked.refusal
                    && held.reason == asked.reason
            }
            _ => false,
        }
    }

    /// Validate the stored shape; the server judges the grant, the caller
    /// and the approvers before an event is made.
    pub fn validate(&self) -> Result<(), IdentityError> {
        match self {
            Self::Created(event) => {
                if !word(&event.client_operation)
                    || event.operation != draft_operation(event.holder, &event.client_operation)
                {
                    return Err(invalid(
                        "a product draft's id is derived from its holder and operation",
                    ));
                }
                if !word(&event.target.kind)
                    || !word(&event.target.id)
                    || !word(&event.target.action)
                    || !word(&event.app)
                    || event
                        .target
                        .kind
                        .strip_prefix(event.app.as_str())
                        .is_none_or(|rest| !rest.starts_with('.'))
                {
                    return Err(invalid(
                        "a product draft names a target of its own app's kinds",
                    ));
                }
                if !event.mode.is_held() {
                    return Err(invalid(
                        "only a grant held by draft or by two makes a draft",
                    ));
                }
                if words_digest(&event.words) != event.request_digest {
                    return Err(invalid(
                        "the request digest is not SHA-256 of the draft's words",
                    ));
                }
                Ok(())
            }
            Self::Approved(event) => {
                if event.operation == event.draft {
                    return Err(invalid("an approval has an operation of its own"));
                }
                Ok(())
            }
            Self::Refused(event) => {
                if event.operation == event.draft || event.reason.trim().is_empty() {
                    return Err(invalid("a refusal needs a distinct operation and a reason"));
                }
                Ok(())
            }
            Self::Executed(event) => {
                if event.operation != close_operation(event.draft) || !word(&event.app) {
                    return Err(invalid("a close is the draft's one close, by its app"));
                }
                Ok(())
            }
            Self::RefusedOnExecution(event) => {
                if event.operation != close_operation(event.draft)
                    || !word(&event.app)
                    || !word(&event.refusal)
                    || event.reason.trim().is_empty()
                {
                    return Err(invalid(
                        "a refusal on execution is the draft's one close, with a name and a reason",
                    ));
                }
                Ok(())
            }
        }
    }
}

#[cfg(test)]
#[path = "product_draft_event_tests.rs"]
mod tests;
