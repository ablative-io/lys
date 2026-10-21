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
//! answers for it, never in their place (P8). An act a product asked for
//! names the app it came through the same way: beside the person.

use crate::binding::LoginBinding;
use crate::error::IdentityError;
use crate::id::{AgentId, ServiceAccountId};

/// How the service authenticated the actor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AuthMethod {
    /// An OIDC sign-in whose token the service validated against the configured issuer.
    Oidc,
    /// A request signed by this agent, which the actor is responsible for,
    /// whose signature the service verified against the agent's certificate.
    AgentSignature(AgentId),
    /// A request made on the machine the service runs on with the install's
    /// operator token, which only the account that owns the install can read.
    Operator,
    /// This person's service account presented its independently verified
    /// bearer credential. This never asserts the person signed in.
    ServiceAccountBearer(ServiceAccountId),
    /// A live pass bound to this agent's run authenticated the request.
    AgentPass(AgentId),
    /// A founding request from a person who has no account yet
    /// (DIRECTORY-111 R1): no credential, the address unproven at this
    /// leaf. The binding names the founding, never a login.
    Founding,
    /// The person proved their address by following the letter's link
    /// (DIRECTORY-111 R2): the session this begins is the person signed in,
    /// as a sign-in through the issuer is.
    AddressProof,
    /// The person followed a link Lys handed them on the screen, because the
    /// install has no mail settings (DIRECTORY-112 R2): the session this
    /// begins is the person signed in, and the record says the link was
    /// handed, never sent.
    HandedLink,
}

impl AuthMethod {
    /// Whether this is the person themselves signed in: through the issuer
    /// or by the proof of their address, never a pass, a signature, the
    /// operator or a service account.
    #[must_use]
    pub fn is_person_signed_in(self) -> bool {
        matches!(self, Self::Oidc | Self::AddressProof | Self::HandedLink)
    }
}

/// The authentication evidence the service has for the actor.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Provenance {
    /// An authentication whose time the service knows.
    Authenticated {
        /// How the actor authenticated.
        method: AuthMethod,
        /// The authentication time in seconds since the Unix epoch.
        authenticated_at: u64,
    },
    /// A verified run pass, with no assertion of an authentication instant.
    AgentPass {
        /// The agent the run belongs to.
        agent: AgentId,
        /// The launch the pass belongs to.
        launch: String,
        /// The session the pass belongs to.
        session: String,
    },
}

impl Provenance {
    /// Authentication by `method` at `authenticated_at`, in seconds since the Unix epoch.
    #[must_use]
    pub fn new(method: AuthMethod, authenticated_at: u64) -> Self {
        Self::Authenticated {
            method,
            authenticated_at,
        }
    }

    /// Authentication by a request `agent` signed, verified at
    /// `authenticated_at`, in seconds since the Unix epoch.
    #[must_use]
    pub fn by_agent(agent: AgentId, authenticated_at: u64) -> Self {
        Self::new(AuthMethod::AgentSignature(agent), authenticated_at)
    }

    /// Authentication by a pass bound to this agent's launch and session.
    ///
    /// # Errors
    /// Refuses an empty run identifier.
    pub fn by_pass(agent: AgentId, launch: &str, session: &str) -> Result<Self, IdentityError> {
        if launch.is_empty() || session.is_empty() {
            return Err(IdentityError::EventMalformed {
                reason: "pass provenance requires nonempty launch and session identifiers",
            });
        }
        Ok(Self::AgentPass {
            agent,
            launch: launch.to_owned(),
            session: session.to_owned(),
        })
    }

    /// How the actor was authenticated.
    #[must_use]
    pub fn method(&self) -> AuthMethod {
        match self {
            Self::Authenticated { method, .. } => *method,
            Self::AgentPass { agent, .. } => AuthMethod::AgentPass(*agent),
        }
    }

    /// The agent whose signed request authenticated the actor; none when the
    /// actor signed in themselves.
    #[must_use]
    pub fn agent(&self) -> Option<AgentId> {
        match self.method() {
            AuthMethod::Oidc
            | AuthMethod::Operator
            | AuthMethod::ServiceAccountBearer(_)
            | AuthMethod::Founding
            | AuthMethod::AddressProof
            | AuthMethod::HandedLink => None,
            AuthMethod::AgentSignature(agent) | AuthMethod::AgentPass(agent) => Some(agent),
        }
    }

    /// The independently authenticated service account, if any.
    #[must_use]
    pub fn service_account(&self) -> Option<ServiceAccountId> {
        match self.method() {
            AuthMethod::ServiceAccountBearer(account) => Some(account),
            AuthMethod::Oidc
            | AuthMethod::Operator
            | AuthMethod::AgentSignature(_)
            | AuthMethod::AgentPass(_)
            | AuthMethod::Founding
            | AuthMethod::AddressProof
            | AuthMethod::HandedLink => None,
        }
    }

    /// When the actor was authenticated, in seconds since the Unix epoch.
    #[must_use]
    pub fn authenticated_at(&self) -> Option<u64> {
        match self {
            Self::Authenticated {
                authenticated_at, ..
            } => Some(*authenticated_at),
            Self::AgentPass { .. } => None,
        }
    }

    /// The verified launch and session, when a run pass authenticated the actor.
    #[must_use]
    pub fn run(&self) -> Option<(&str, &str)> {
        match self {
            Self::AgentPass {
                launch, session, ..
            } => Some((launch, session)),
            Self::Authenticated { .. } => None,
        }
    }
}

/// The authenticated human who made a change, as the service attests them.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Actor {
    binding: LoginBinding,
    provenance: Provenance,
    draft: Option<Box<DraftProvenance>>,
    through_app: Option<String>,
}

/// The prepared act's origin, separate from the approver's authentication.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DraftProvenance {
    /// The immutable draft id.
    pub draft: crate::OperationId,
    /// The reserved application operation.
    pub application: crate::OperationId,
    /// The agent that prepared the act, absent for a person's correction.
    pub drafted_by: Option<AgentId>,
}

impl Actor {
    /// Attach the approved act's origin without changing authentication.
    #[must_use]
    pub fn with_draft(mut self, draft: DraftProvenance) -> Self {
        self.draft = Some(Box::new(draft));
        self
    }
    /// The prepared act carried by this authenticated person.
    #[must_use]
    pub fn draft(&self) -> Option<&DraftProvenance> {
        self.draft.as_deref()
    }

    /// Attach the approved app the person acted through (DIRECTORY-113
    /// Amendment 2): named beside them, never in their place.
    #[must_use]
    pub fn with_through_app(mut self, app: &str) -> Self {
        self.through_app = Some(app.to_owned());
        self
    }

    /// The approved app this person acted through, when a product asked.
    #[must_use]
    pub fn through_app(&self) -> Option<&str> {
        self.through_app.as_deref()
    }

    /// The human named by `binding`, one of their logins, authenticated as
    /// `provenance` says.
    #[must_use]
    pub fn new(binding: LoginBinding, provenance: Provenance) -> Self {
        Self {
            binding,
            provenance,
            draft: None,
            through_app: None,
        }
    }

    /// The login that names the actor: the one they signed in through, or the
    /// one of theirs the service admitted their agent's signed request under.
    #[must_use]
    pub fn binding(&self) -> &LoginBinding {
        &self.binding
    }

    /// How and when the actor was authenticated.
    #[must_use]
    pub fn provenance(&self) -> &Provenance {
        &self.provenance
    }
}

#[cfg(test)]
#[path = "provenance_tests.rs"]
mod tests;
