//! Refusals of the secrets list and of a lease's read, revoke and
//! relinquish, each with the HTTP status and the JSON body a route answers
//! it with, so every server that mounts the routes answers them alike.
//!
//! A lease the caller may not discover answers exactly as a lease that does
//! not exist: one status and one body, naming nothing.

use serde_json::{Value, json};

use super::SecretsError;
use crate::broker::LeaseEnd;

/// Why the secrets list was not answered.
#[derive(Debug, thiserror::Error)]
pub enum ListRefusal {
    /// The list was asked for with a scope outside the closed set.
    #[error(
        "unknown_scope: {given:?} is not a scope of the secrets list (act: ask with scope organisation, team or mine, or with no scope for every secret you may see)"
    )]
    UnknownScope {
        /// The scope given.
        given: String,
    },
    /// An agent asked for the secrets list, which is a person's view.
    #[error(
        "agent_uses_virtual_credentials: {agent} is an agent, and agents reach secrets only through their virtual credentials (act: use the handle issued to the agent)"
    )]
    AgentUsesVirtualCredentials {
        /// The agent that asked.
        agent: String,
    },
}

impl ListRefusal {
    /// The refusal's name, the `error` of its body.
    pub fn name(&self) -> &'static str {
        match self {
            Self::UnknownScope { .. } => "unknown_scope",
            Self::AgentUsesVirtualCredentials { .. } => "agent_uses_virtual_credentials",
        }
    }

    /// The HTTP status a route answers it with.
    pub fn status(&self) -> u16 {
        match self {
            Self::UnknownScope { .. } => 400,
            Self::AgentUsesVirtualCredentials { .. } => 403,
        }
    }

    /// The JSON body a route answers it with. It carries no list.
    pub fn body(&self) -> Value {
        match self {
            Self::UnknownScope { given } => json!({
                "error": self.name(),
                "scope": given,
                "reason": self.to_string(),
            }),
            Self::AgentUsesVirtualCredentials { agent } => json!({
                "error": self.name(),
                "agent": agent,
                "reason": self.to_string(),
            }),
        }
    }
}

/// Why a lease was not read, revoked or relinquished.
#[derive(Debug, thiserror::Error)]
pub enum LeaseRefusal {
    /// No such lease, or one the caller may not discover; the two are not
    /// told apart and nothing is named.
    #[error(
        "lease_not_found: no lease by that id stands for you (act: ask about a lease you hold or are acted for under)"
    )]
    NotFound,
    /// The lease has already ended; the first end record stays the only
    /// one.
    #[error(
        "lease_already_ended: lease {lease} already ended by {} (act: nothing; ask for a new lease if access is still needed)",
        .end.way.label()
    )]
    AlreadyEnded {
        /// The lease.
        lease: String,
        /// How, when and by whom it first ended.
        end: LeaseEnd,
    },
    /// The lease's holder, not its person acted for, asked to revoke it.
    #[error(
        "holder_relinquishes: the holder of lease {lease} ends it by relinquish, not revoke (act: relinquish the lease)"
    )]
    HolderRelinquishes {
        /// The lease.
        lease: String,
    },
    /// A caller who may discover the lease may not revoke it.
    #[error(
        "revoke_not_permitted: you may not revoke lease {lease} (act: ask the person the lease is acted for to revoke it)"
    )]
    RevokeNotPermitted {
        /// The lease.
        lease: String,
    },
    /// A caller who may discover the lease is not its holder, so may not
    /// relinquish it.
    #[error(
        "relinquish_not_permitted: only the holder of lease {lease} relinquishes it (act: revoke it if it is acted for you)"
    )]
    RelinquishNotPermitted {
        /// The lease.
        lease: String,
    },
    /// The broker could not record the act.
    #[error(transparent)]
    Failed(#[from] SecretsError),
}

impl LeaseRefusal {
    /// The refusal's name, the `error` of its body.
    pub fn name(&self) -> &'static str {
        match self {
            Self::NotFound => "lease_not_found",
            Self::AlreadyEnded { .. } => "lease_already_ended",
            Self::HolderRelinquishes { .. } => "holder_relinquishes",
            Self::RevokeNotPermitted { .. } => "revoke_not_permitted",
            Self::RelinquishNotPermitted { .. } => "relinquish_not_permitted",
            Self::Failed(error) => error.name(),
        }
    }

    /// The HTTP status a route answers it with.
    pub fn status(&self) -> u16 {
        match self {
            Self::NotFound => 404,
            Self::AlreadyEnded { .. } => 409,
            Self::HolderRelinquishes { .. }
            | Self::RevokeNotPermitted { .. }
            | Self::RelinquishNotPermitted { .. } => 403,
            Self::Failed(_) => 500,
        }
    }

    /// The JSON body a route answers it with.
    pub fn body(&self) -> Value {
        match self {
            Self::NotFound | Self::Failed(_) => json!({
                "error": self.name(),
                "reason": self.to_string(),
            }),
            Self::AlreadyEnded { lease, end } => json!({
                "error": self.name(),
                "lease": lease,
                "ended_by": end.way.label(),
                "ended_at_ms": end.at_ms,
                "ended_by_identity": end.by,
                "reason": self.to_string(),
            }),
            Self::HolderRelinquishes { lease } => json!({
                "error": self.name(),
                "lease": lease,
                "act": "relinquish",
                "reason": self.to_string(),
            }),
            Self::RevokeNotPermitted { lease } | Self::RelinquishNotPermitted { lease } => json!({
                "error": self.name(),
                "lease": lease,
                "reason": self.to_string(),
            }),
        }
    }
}
