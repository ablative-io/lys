//! A change an agent made through the Lys MCP endpoint, kept as a leaf of the
//! directory log. The leaf names the route, the digest of the body the route
//! was given, the status it answered and, when the agent signed the message
//! that asked for the change, that signature as it was sent, so the change
//! carries a receipt and its evidence survives replay with it.

use lys_log_store::LeafStore;

use crate::directory::Directory;
use crate::error::IdentityError;
use crate::event::Change;
use crate::id::{AgentId, IdentityId};
use crate::operation::OperationId;
use crate::provenance::Actor;
use crate::receipt::Receipt;

/// The most bytes an agent call's path or signature carries.
const TEXT_MAX: usize = 4096;

/// One change an agent made through MCP.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AgentCall {
    method: String,
    path: String,
    body_sha256: String,
    status: u16,
    signature: String,
}

impl AgentCall {
    /// A call record, refused unless it names a changing method, a local
    /// route, a SHA-256 digest in lowercase hex and a status the route
    /// answered for a change it made.
    pub fn new(
        method: &str,
        path: &str,
        body_sha256: &str,
        status: u16,
        signature: &str,
    ) -> Result<Self, IdentityError> {
        if !matches!(method, "POST" | "PUT" | "PATCH" | "DELETE") {
            return Err(IdentityError::ChangeMismatch {
                reason: "an agent call records a changing method",
            });
        }
        if !path.starts_with('/') || path.starts_with("//") || path.len() > TEXT_MAX {
            return Err(IdentityError::ChangeMismatch {
                reason: "an agent call names a local route of at most 4096 bytes",
            });
        }
        if body_sha256.len() != 64
            || !body_sha256
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(IdentityError::ChangeMismatch {
                reason: "an agent call's body digest is SHA-256 in lowercase hex",
            });
        }
        if !(200..300).contains(&status) {
            return Err(IdentityError::ChangeMismatch {
                reason: "an agent call records a change the route made",
            });
        }
        if signature.len() > TEXT_MAX {
            return Err(IdentityError::ChangeMismatch {
                reason: "an agent call's signature is at most 4096 bytes",
            });
        }
        Ok(Self {
            method: method.to_owned(),
            path: path.to_owned(),
            body_sha256: body_sha256.to_owned(),
            status,
            signature: signature.to_owned(),
        })
    }

    /// The route's method.
    pub fn method(&self) -> &str {
        &self.method
    }

    /// The route's path, with its query.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// The SHA-256 of the body the route was given, in lowercase hex.
    pub fn body_sha256(&self) -> &str {
        &self.body_sha256
    }

    /// The status the route answered.
    pub fn status(&self) -> u16 {
        self.status
    }

    /// The agent's signature header over the MCP message, as it was sent;
    /// empty when the call was admitted by a run pass or a grant token.
    pub fn signature(&self) -> &str {
        &self.signature
    }
}

impl<S: LeafStore> Directory<S> {
    /// Record a change `agent` made through MCP, answering its receipt. The
    /// actor must be the agent acting for its responsible person.
    pub fn record_agent_call(
        &mut self,
        actor: Actor,
        agent: AgentId,
        call: AgentCall,
        recorded_at: u64,
    ) -> Result<Receipt, IdentityError> {
        if actor.provenance().agent() != Some(agent) {
            return Err(IdentityError::ChangeMismatch {
                reason: "an agent call is recorded by the agent that made it",
            });
        }
        self.commit_change(
            actor,
            OperationId::generate()?,
            IdentityId::Agent(agent),
            Change::AgentCall(call),
            recorded_at,
        )
    }
}

#[cfg(test)]
#[path = "agent_call_tests.rs"]
mod tests;
