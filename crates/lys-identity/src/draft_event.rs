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

/// A draft creation or a hash-bound approval in the directory log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DraftEvent {
    /// Record the immutable prepared change.
    Created(Arc<Created>),
    /// Record a decision bound to that creation's payload.
    Approved(Arc<Approved>),
}

fn invalid(reason: &'static str) -> IdentityError {
    IdentityError::DraftChangeInvalid { reason }
}

fn token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'_' | b'-' | b'.')
        })
}

fn request(method: &str, path: &str, body: &[u8]) -> Result<(), IdentityError> {
    if !matches!(method, "POST" | "PUT" | "PATCH" | "DELETE")
        || !path.starts_with('/')
        || path.starts_with("//")
        || path.len() > 2048
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
    if body.len() > crate::signer::MAX_EVENT_BYTES {
        return Err(invalid("the request body exceeds the leaf bound"));
    }
    Ok(())
}

fn evidence(actor: &Actor, proof: Option<&RequestEvidence>) -> Result<(), IdentityError> {
    match (actor.provenance().agent(), proof) {
        (None, None) => Ok(()),
        (Some(agent), Some(proof)) if agent == proof.agent => {
            request(&proof.method, &proof.path, &proof.body)?;
            if proof.nonce.len() < 32
                || proof.nonce.len() > 128
                || !proof.nonce.bytes().all(|byte| byte.is_ascii_hexdigit())
                || proof.cose_sign1.is_empty()
                || proof.cose_sign1.len() > crate::signer::MAX_EVENT_BYTES
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
    pub fn operation(&self) -> OperationId {
        match self {
            Self::Created(event) => event.operation,
            Self::Approved(event) => event.operation,
        }
    }

    /// Validate the stored shape; caller admission verifies authority and cryptographic evidence.
    pub fn validate(&self) -> Result<(), IdentityError> {
        match self {
            Self::Created(event) => {
                if !token(&event.target.kind)
                    || !token(&event.target.id)
                    || !token(&event.target.action)
                    || event.note.chars().count() > 500
                    || event.corrects == Some(event.operation)
                {
                    return Err(invalid(
                        "the draft needs a valid resource, bounded note and distinct correction link",
                    ));
                }
                request(&event.method, &event.path, &event.body)?;
                evidence(&event.actor, event.evidence.as_ref())
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
        }
    }
}
