//! Immutable prepared changes and the decisions bound to their payload hashes.

use crate::{Actor, AgentId, IdentityError, IdentityId, OperationId};
use std::sync::Arc;

#[path = "draft_event_encoding.rs"]
mod encoding;
pub use encoding::{decode, encode};

/// The authority resource validated by the registered mutation route.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    /// The grant resource kind.
    pub kind: String,
    /// The grant resource identifier.
    pub id: String,
    /// The required grant action.
    pub action: String,
}

/// Original verified agent request material, separate from a derived action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestEvidence {
    /// The signing agent.
    pub agent: AgentId,
    /// The method the agent signed.
    pub method: String,
    /// The path the agent signed.
    pub path: String,
    /// The original body, before any parsing or reserialization.
    pub body: Vec<u8>,
    /// The signed timestamp in milliseconds.
    pub signed_at_ms: u64,
    /// The nonce text exactly as signed.
    pub nonce: String,
    /// The original complete COSE attestation.
    pub cose_sign1: Vec<u8>,
}

/// The request bytes retained only when the original bytes are available.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestSignature {
    /// The original signature header value, including its whitespace.
    pub header: Vec<u8>,
    /// The exact payload the agent signed.
    pub payload: Vec<u8>,
}

/// One immutable draft creation payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Created {
    /// The creation operation and draft identifier.
    pub operation: OperationId,
    /// The authenticated original author.
    pub actor: Actor,
    /// Creation time in seconds.
    pub recorded_at: u64,
    /// The action's authority resource.
    pub target: Target,
    /// The registered mutation method.
    pub method: String,
    /// The registered local mutation path.
    pub path: String,
    /// The exact prepared body to apply after approval.
    pub body: Vec<u8>,
    /// The author's review note.
    pub note: String,
    /// The authority holder selected on the reporting line.
    pub reviewer: IdentityId,
    /// The original draft when this payload corrects it.
    pub corrects: Option<OperationId>,
    /// The verified original request when an agent authored the draft.
    pub evidence: Option<RequestEvidence>,
    /// Original header and payload; absent only for historical evidence or a human caller.
    pub request_signature: Option<RequestSignature>,
}

/// An approval of one immutable draft, never of replacement words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Approved {
    /// The decision operation.
    pub operation: OperationId,
    /// The authenticated reviewer.
    pub actor: Actor,
    /// Decision time in seconds.
    pub recorded_at: u64,
    /// The creation operation being approved.
    pub draft: OperationId,
    /// SHA-256 of the exact canonical creation payload.
    pub draft_hash: [u8; 32],
    /// The operation reserved for application of the stored action.
    pub application: OperationId,
    /// The reviewer's verified request when agent-authenticated.
    pub evidence: Option<RequestEvidence>,
}

/// A refusal of one immutable creation payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refused {
    /// The refusal operation.
    pub operation: OperationId,
    /// The authenticated reviewer.
    pub actor: Actor,
    /// Refusal time in seconds.
    pub recorded_at: u64,
    /// The creation operation.
    pub draft: OperationId,
    /// SHA-256 of the creation payload.
    pub draft_hash: [u8; 32],
    /// The reason the original is refused.
    pub reason: String,
    /// The reviewer's verified agent evidence, when present.
    pub evidence: Option<RequestEvidence>,
}

/// One durable act refusing the original and saving the corrector's own change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Correction {
    /// The correction operation.
    pub operation: OperationId,
    /// The authenticated corrector.
    pub actor: Actor,
    /// Correction time in seconds.
    pub recorded_at: u64,
    /// The original creation operation.
    pub draft: OperationId,
    /// SHA-256 of the original creation payload.
    pub draft_hash: [u8; 32],
    /// The reason the original is refused.
    pub reason: String,
    /// The replacement owned by the corrector, linked back to the original.
    pub corrected: Arc<Created>,
    /// SHA-256 of the replacement's canonical payload.
    pub corrected_hash: [u8; 32],
}

/// The durable outcome of carrying out an approved draft.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Executed {
    /// The closing event's operation.
    pub operation: OperationId,
    /// The approving person, authenticated independently of the author.
    pub actor: Actor,
    /// The closing instant in seconds.
    pub recorded_at: u64,
    /// The immutable draft being closed.
    pub draft: OperationId,
    /// The creation payload commitment.
    pub draft_hash: [u8; 32],
    /// The operation reserved at approval.
    pub application: OperationId,
    /// The successful route status.
    pub status: u16,
    /// The applied route's durable receipt index.
    pub receipt_index: u64,
}

/// The durable outcome of carrying out an approved draft.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefusedOnExecution {
    /// The closing event's operation.
    pub operation: OperationId,
    /// The approving person, authenticated independently of the author.
    pub actor: Actor,
    /// The closing instant in seconds.
    pub recorded_at: u64,
    /// The immutable draft being closed.
    pub draft: OperationId,
    /// The creation payload commitment.
    pub draft_hash: [u8; 32],
    /// The operation reserved at approval.
    pub application: OperationId,
    /// The route's own refusal name.
    pub refusal: String,
    /// The route's own refusal words.
    pub reason: String,
}

/// A draft creation or a hash-bound approval in the directory log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DraftEvent {
    /// Record the immutable prepared change.
    Created(Arc<Created>),
    /// The route carried out the approved request.
    Executed(Arc<Executed>),
    /// The route refused the approved request.
    RefusedOnExecution(Arc<RefusedOnExecution>),
    /// Record a decision bound to that creation's payload.
    Approved(Arc<Approved>),
    /// Refuse the original change without applying it.
    Refused(Arc<Refused>),
    /// Refuse and link the original to the corrector's own replacement.
    Correction(Arc<Correction>),
}

fn invalid(reason: &'static str) -> IdentityError {
    IdentityError::DraftChangeInvalid { reason }
}

fn token(value: &str) -> bool {
    !value.is_empty()
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'_' | b'-' | b'.')
        })
}

/// A resource id, as the grant resource takes it: case kept, never folded.
fn resource_id(value: &str) -> bool {
    !value.is_empty() && value.bytes().all(crate::grants::types::resource_id_byte)
}

fn request(method: &str, path: &str) -> Result<(), IdentityError> {
    if !matches!(method, "POST" | "PUT" | "PATCH" | "DELETE")
        || !path.starts_with('/')
        || path.starts_with("//")
        || !path.is_ascii()
        || path
            .bytes()
            .any(|byte| byte.is_ascii_control() || matches!(byte, b'?' | b'#'))
        || path.split('/').any(|part| matches!(part, "." | ".."))
    {
        return Err(invalid(
            "the request must name a local mutation method and path",
        ));
    }
    Ok(())
}

fn evidence(actor: &Actor, proof: Option<&RequestEvidence>) -> Result<(), IdentityError> {
    match (actor.provenance().agent(), proof) {
        (None, None) => Ok(()),
        (Some(agent), Some(proof)) if agent == proof.agent => {
            request(&proof.method, &proof.path)?;
            if proof.nonce.len() < 32
                || !proof.nonce.bytes().all(|byte| byte.is_ascii_hexdigit())
                || proof.cose_sign1.is_empty()
            {
                return Err(invalid(
                    "agent evidence needs its original nonce and COSE attestation",
                ));
            }
            Ok(())
        }
        _ => Err(invalid("agent provenance and request evidence must agree")),
    }
}

impl DraftEvent {
    /// The operation this leaf records.
    #[must_use]
    pub fn operation(&self) -> OperationId {
        match self {
            Self::Created(event) => event.operation,
            Self::Executed(event) => event.operation,
            Self::RefusedOnExecution(event) => event.operation,
            Self::Approved(event) => event.operation,
            Self::Refused(event) => event.operation,
            Self::Correction(event) => event.operation,
        }
    }

    /// Validate the stored shape; caller admission verifies authority and cryptographic evidence.
    ///
    /// # Errors
    /// - `IdentityError::DraftChangeInvalid` if the target, request, evidence, signature shape or decision operations violate the draft contract.
    /// - `IdentityError::DraftHashMismatch` if a correction does not commit to its corrected creation payload.
    pub fn validate(&self) -> Result<(), IdentityError> {
        match self {
            Self::Executed(event) => {
                if !(200..300).contains(&event.status) {
                    return Err(invalid("an execution needs a successful status"));
                }
                closing(event.operation, event.draft, event.application)
            }
            Self::RefusedOnExecution(event) => {
                if event.refusal.trim().is_empty() || event.reason.trim().is_empty() {
                    return Err(invalid("an execution refusal needs its name and words"));
                }
                closing(event.operation, event.draft, event.application)
            }
            Self::Created(event) => {
                if !token(&event.target.kind)
                    || !resource_id(&event.target.id)
                    || !token(&event.target.action)
                    || event.corrects == Some(event.operation)
                {
                    return Err(invalid(
                        "the draft needs a valid resource and a distinct correction link",
                    ));
                }
                request(&event.method, &event.path)?;
                evidence(&event.actor, event.evidence.as_ref())?;
                if let Some(signature) = &event.request_signature {
                    let proof = event
                        .evidence
                        .as_ref()
                        .ok_or_else(|| invalid("a request signature needs agent evidence"))?;
                    signature.check(proof)?;
                }
                Ok(())
            }
            Self::Approved(event) => {
                if event.operation == event.draft
                    || event.application == event.operation
                    || event.application == event.draft
                {
                    return Err(invalid(
                        "creation, approval and application operations must be distinct",
                    ));
                }
                evidence(&event.actor, event.evidence.as_ref())
            }
            Self::Refused(event) => {
                if event.operation == event.draft || event.reason.trim().is_empty() {
                    return Err(invalid("a refusal needs a distinct operation and a reason"));
                }
                evidence(&event.actor, event.evidence.as_ref())
            }
            Self::Correction(event) => {
                if event.operation == event.draft
                    || event.operation == event.corrected.operation
                    || event.corrected.operation == event.draft
                    || event.reason.trim().is_empty()
                    || event.corrected.actor != event.actor
                    || event.corrected.recorded_at != event.recorded_at
                    || event.corrected.corrects != Some(event.draft)
                {
                    return Err(invalid(
                        "a correction must refuse the original and save the corrector's own linked change",
                    ));
                }
                Self::Created(Arc::clone(&event.corrected)).validate()?;
                if crate::encoding::payload_commitment(&encode(&Self::Created(Arc::clone(
                    &event.corrected,
                )))) != event.corrected_hash
                {
                    return Err(IdentityError::DraftHashMismatch);
                }
                Ok(())
            }
        }
    }

    /// The immutable wire version, preserving historical creation hashes.
    #[must_use]
    pub fn version(&self) -> u64 {
        match self {
            Self::Created(event) if event.request_signature.is_some() => 2,
            Self::Refused(_)
            | Self::Correction(_)
            | Self::Executed(_)
            | Self::RefusedOnExecution(_) => 2,
            Self::Created(_) | Self::Approved(_) => 1,
        }
    }

    pub(crate) fn decision(&self) -> Option<(OperationId, [u8; 32])> {
        match self {
            Self::Created(_) => None,
            Self::Approved(event) => Some((event.draft, event.draft_hash)),
            Self::Executed(event) => Some((event.draft, event.draft_hash)),
            Self::RefusedOnExecution(event) => Some((event.draft, event.draft_hash)),
            Self::Refused(event) => Some((event.draft, event.draft_hash)),
            Self::Correction(event) => Some((event.draft, event.draft_hash)),
        }
    }

    pub(crate) fn validate_new(&self) -> Result<(), IdentityError> {
        self.validate()?;
        let creation = match self {
            Self::Created(event) => Some(event),
            Self::Correction(event) => Some(&event.corrected),
            Self::Approved(_)
            | Self::Refused(_)
            | Self::Executed(_)
            | Self::RefusedOnExecution(_) => None,
        };
        if creation
            .is_some_and(|event| event.evidence.is_some() && event.request_signature.is_none())
        {
            return Err(invalid(
                "new agent-authored changes require the original header and signed payload",
            ));
        }
        Ok(())
    }
}

#[path = "draft_request_signature.rs"]
mod signature;

#[cfg(test)]
#[path = "draft_event_close_tests.rs"]
mod close_tests;

/// The approver closes the draft: a person, or an agent admitted on its own
/// draft.decide grant, whose signed decision the approval carries (24 October
/// 2026). Who may approve is the server's judgement at the approval; the
/// close only needs the three operations kept apart.
fn closing(
    operation: OperationId,
    draft: OperationId,
    application: OperationId,
) -> Result<(), IdentityError> {
    if operation == draft || operation == application || draft == application {
        return Err(invalid("a close needs distinct operations"));
    }
    Ok(())
}
