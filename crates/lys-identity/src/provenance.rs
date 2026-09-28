//! The actor of a change, as the service attests it.
//!
//! The directory service signs every event with its own key. What it attests
//! is that it authenticated this human, through this login, by this method, at
//! this time. The actor is always a person. The method says how the service
//! authenticated them: their own OIDC sign-in, or a request signed by an agent
//! they are responsible for, whose signature the service verified against the
//! agent's certificate. In the second case the provenance keeps the agent's
//! id, and in the first it keeps none: the agent's id is part of the method,
//! so neither can be made without the other.
//!
//! The service never claims the person signed anything: a person holds no key
//! the directory can speak for. An agent registered by a person records that
//! person, and an agent that signed a request is named beside the person who
//! answers for it, never in their place (P8).

use crate::binding::LoginBinding;
use crate::id::AgentId;

/// How the service authenticated the actor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AuthMethod {
    /// An OIDC sign-in whose token the service validated against the configured issuer.
    Oidc,
    /// A request signed by this agent, which the actor is responsible for,
    /// whose signature the service verified against the agent's certificate.
    AgentSignature(AgentId),
}

/// How and when the service authenticated the actor.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Provenance {
    method: AuthMethod,
    authenticated_at: u64,
}

impl Provenance {
    /// Authentication by `method` at `authenticated_at`, in seconds since the Unix epoch.
    pub fn new(method: AuthMethod, authenticated_at: u64) -> Self {
        Self {
            method,
            authenticated_at,
        }
    }

    /// Authentication by a request `agent` signed, verified at
    /// `authenticated_at`, in seconds since the Unix epoch.
    pub fn by_agent(agent: AgentId, authenticated_at: u64) -> Self {
        Self::new(AuthMethod::AgentSignature(agent), authenticated_at)
    }

    /// How the actor was authenticated.
    pub fn method(&self) -> AuthMethod {
        self.method
    }

    /// The agent whose signed request authenticated the actor; none when the
    /// actor signed in themselves.
    pub fn agent(&self) -> Option<AgentId> {
        match self.method {
            AuthMethod::Oidc => None,
            AuthMethod::AgentSignature(agent) => Some(agent),
        }
    }

    /// When the actor was authenticated, in seconds since the Unix epoch.
    pub fn authenticated_at(&self) -> u64 {
        self.authenticated_at
    }
}

/// The authenticated human who made a change, as the service attests them.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Actor {
    binding: LoginBinding,
    provenance: Provenance,
}

impl Actor {
    /// The human named by `binding`, one of their logins, authenticated as
    /// `provenance` says.
    pub fn new(binding: LoginBinding, provenance: Provenance) -> Self {
        Self {
            binding,
            provenance,
        }
    }

    /// The login that names the actor: the one they signed in through, or the
    /// one of theirs the service admitted their agent's signed request under.
    pub fn binding(&self) -> &LoginBinding {
        &self.binding
    }

    /// How and when the actor was authenticated.
    pub fn provenance(&self) -> &Provenance {
        &self.provenance
    }
}
