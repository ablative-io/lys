//! The identity event: one change to the directory and its audit record at once.
//!
//! An event names the caller's operation id, the actor the service
//! authenticated, the identity it changes, when the service recorded it, and
//! the change itself. It is signed and appended to the log as one leaf, and the
//! directory is rebuilt from those leaves when it opens (P4). The log
//! coordinate is not part of the event: it is returned in the receipt, outside
//! the leaf, because the leaf's bytes are fixed before the log gives it a place.
//!
//! [`IdentityEvent::new`] is the only way to make an event, and it refuses a
//! change that does not fit the identity it names. A decoded event is built
//! through it as well, so an event read back from the log obeys the same rules
//! as one written today.

use crate::binding::LoginBinding;
use crate::error::IdentityError;
use crate::id::{AgentId, IdentityId, PersonId};
use crate::lifecycle::{LifecycleState, Transition};
use crate::operation::OperationId;
use crate::profile::Profile;
use crate::provenance::{Actor, AuthMethod};

/// The envelope version this crate writes and reads.
pub const EVENT_VERSION: u64 = 1;

/// Whether an issuer observed a login being linked to or unlinked from a person.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LinkChange {
    /// The login was linked.
    Linked,
    /// The login was unlinked.
    Unlinked,
}

/// A link or unlink the issuer observed and reported, as the receiver accepted it.
///
/// This is the issuer's observation, carried apart from any claim a person
/// made: the event records that the issuer reported it, under the source's own
/// operation id, and never that the person signed it.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LinkObservation {
    source_operation_id: String,
    change: LinkChange,
    binding: LoginBinding,
    observer: String,
    observed_at: u64,
}

impl LinkObservation {
    /// The observation the source reported under `source_operation_id`.
    pub fn new(
        source_operation_id: &str,
        change: LinkChange,
        binding: LoginBinding,
        observer: &str,
        observed_at: u64,
    ) -> Result<Self, IdentityError> {
        if source_operation_id.is_empty() {
            return Err(IdentityError::ChangeMismatch {
                reason: "a source operation id is not empty",
            });
        }
        if observer.is_empty() {
            return Err(IdentityError::ChangeMismatch {
                reason: "an observation names the issuer that observed it",
            });
        }
        Ok(Self {
            source_operation_id: source_operation_id.to_owned(),
            change,
            binding,
            observer: observer.to_owned(),
            observed_at,
        })
    }

    /// The source's own operation id, which the receiver deduplicates on.
    pub fn source_operation_id(&self) -> &str {
        &self.source_operation_id
    }

    /// Whether the login was linked or unlinked.
    pub fn change(&self) -> LinkChange {
        self.change
    }

    /// The login that was linked or unlinked.
    pub fn binding(&self) -> &LoginBinding {
        &self.binding
    }

    /// The issuer that observed the change.
    pub fn observer(&self) -> &str {
        &self.observer
    }

    /// When the issuer observed it, in seconds since the Unix epoch.
    pub fn observed_at(&self) -> u64 {
        self.observed_at
    }
}

/// One change to the directory.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Change {
    /// First-run person creation, active and bound to the event's authenticated actor.
    /// Admission as the configured administrator belongs to the service.
    SetupPerson {
        /// The person's display name.
        profile: Profile,
    },
    /// A person is registered with a display profile.
    RegisterPerson {
        /// How the person is shown.
        profile: Profile,
    },
    /// An agent is registered with a direct edge to its accountable person.
    RegisterAgent {
        /// The signed-in person who registered the agent.
        responsible: PersonId,
        /// How the agent is shown.
        profile: Profile,
    },
    /// Register an agent with its reporting edge in the same signed change.
    ReportingRegistration {
        /// The person reached by the reporting chain.
        responsible: PersonId,
        /// How the agent is shown.
        profile: Profile,
        /// The immediate person or agent the new agent reports to.
        reports_to: IdentityId,
    },
    /// Move one reporting edge and its subtree's responsibility atomically.
    ReportsToChanged {
        /// The previous reporting target.
        from: IdentityId,
        /// The chosen reporting target.
        to: IdentityId,
        /// The previous accountable person.
        responsible_from: PersonId,
        /// The new accountable person.
        responsible_to: PersonId,
    },
    /// An identity's display profile is replaced.
    ChangeProfile {
        /// The new profile.
        profile: Profile,
    },
    /// A login is bound to a person.
    BindLogin {
        /// The login.
        binding: LoginBinding,
    },
    /// An identity moves from one lifecycle state to another.
    Transition {
        /// The transition.
        transition: Transition,
        /// The state it left.
        from: LifecycleState,
        /// The state it entered.
        to: LifecycleState,
        /// The reason given, empty when the transition needs none and none was given.
        reason: String,
    },
    /// The link-audit receiver accepted an issuer's observation.
    LinkAudit(LinkObservation),
    /// An agent made a change through MCP.
    AgentCall(crate::agent_call::AgentCall),
}

/// A signed directory change, before it is signed or after it is verified.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IdentityEvent {
    operation: OperationId,
    actor: Actor,
    identity: IdentityId,
    recorded_at: u64,
    change: Change,
}

/// Whether `target` is a kind a directory identity may report to.
fn reporting_target(target: IdentityId) -> bool {
    match target {
        IdentityId::Person(_) | IdentityId::Agent(_) => true,
        IdentityId::ServiceAccount(_) | IdentityId::Connector(_) | IdentityId::Machine(_) => false,
    }
}

/// Refuse `change` if it does not fit `identity`.
fn check_fit(identity: IdentityId, change: &Change) -> Result<(), IdentityError> {
    let invalid_target = match change {
        Change::ReportingRegistration { reports_to, .. } => !reporting_target(*reports_to),
        Change::ReportsToChanged { from, to, .. } => {
            !reporting_target(*from) || !reporting_target(*to)
        }
        _ => false,
    };
    if invalid_target {
        return Err(IdentityError::ChangeMismatch {
            reason: "a reporting target must be a person or agent",
        });
    }
    match identity {
        IdentityId::Person(_) | IdentityId::Agent(_) => {}
        IdentityId::ServiceAccount(_) => {
            return Err(IdentityError::ChangeMismatch {
                reason: "service account lifecycle belongs to the service-account log",
            });
        }
        IdentityId::Connector(_) => {
            return Err(IdentityError::ChangeMismatch {
                reason: "a connector is recorded with its app's approval in the apps log",
            });
        }
        IdentityId::Machine(_) => {
            return Err(IdentityError::ChangeMismatch {
                reason: "a machine is recorded with its join in the connection codes",
            });
        }
    }
    match (change, identity) {
        (Change::RegisterPerson { .. } | Change::SetupPerson { .. }, IdentityId::Agent(_)) => {
            Err(IdentityError::ChangeMismatch {
                reason: "a person registration names a person",
            })
        }
        (
            Change::RegisterAgent { .. }
            | Change::ReportingRegistration { .. }
            | Change::ReportsToChanged { .. },
            IdentityId::Person(_),
        ) => Err(IdentityError::ChangeMismatch {
            reason: "an agent registration names an agent",
        }),
        (Change::BindLogin { .. }, IdentityId::Agent(_)) => Err(IdentityError::ChangeMismatch {
            reason: "a login is bound to a person, never to an agent",
        }),
        (Change::LinkAudit(_), IdentityId::Agent(_)) => Err(IdentityError::ChangeMismatch {
            reason: "a link observation names the person the login belongs to",
        }),
        (Change::AgentCall(_), IdentityId::Person(_)) => Err(IdentityError::ChangeMismatch {
            reason: "an agent call names the agent that made it",
        }),
        (
            Change::Transition {
                transition,
                from,
                to,
                reason,
            },
            _,
        ) => {
            if transition.target(*from)? != *to {
                return Err(IdentityError::ChangeMismatch {
                    reason: "a transition's recorded target is not the one the table gives",
                });
            }
            if transition.requires_reason() && reason.trim().is_empty() {
                return Err(IdentityError::ReasonRequired {
                    transition: *transition,
                });
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

impl IdentityEvent {
    /// Service-account provenance uses a new envelope version. Existing
    /// OIDC and agent-signature events retain their exact v1 encoding.
    pub fn version(&self) -> u64 {
        if self.actor.provenance().service_account().is_some() {
            2
        } else {
            EVENT_VERSION
        }
    }

    /// The event recording `change` to `identity`, made by `actor` under `operation`.
    pub fn new(
        operation: OperationId,
        actor: Actor,
        identity: IdentityId,
        recorded_at: u64,
        change: Change,
    ) -> Result<Self, IdentityError> {
        check_fit(identity, &change)?;
        Ok(Self {
            operation,
            actor,
            identity,
            recorded_at,
            change,
        })
    }

    /// The operation id the caller gave the change.
    pub fn operation(&self) -> OperationId {
        self.operation
    }

    /// The authenticated human who made the change, as the service attests them.
    pub fn actor(&self) -> &Actor {
        &self.actor
    }

    /// The identity the change is about.
    pub fn identity(&self) -> IdentityId {
        self.identity
    }

    /// When the service recorded the change, in seconds since the Unix epoch.
    pub fn recorded_at(&self) -> u64 {
        self.recorded_at
    }

    /// The change.
    pub fn change(&self) -> &Change {
        &self.change
    }
}

/// The envelope's wire codes. Each closed set is read both ways here, next to
/// its type, and docs/design/identity/IDENTITY-EVENTS.md gives the same table.
pub(crate) mod wire {
    use super::{AgentId, AuthMethod, Change, LifecycleState, LinkChange, Transition};

    /// An identity that is a person.
    pub(crate) const PERSON: u64 = 1;
    /// An identity that is an agent.
    pub(crate) const AGENT: u64 = 2;
    /// A principal held in the service-account log.
    pub(crate) const SERVICE_ACCOUNT: u64 = 3;
    /// An approved app's connector, recorded in the apps log.
    pub(crate) const CONNECTOR: u64 = 4;
    /// A computer that joined with a connection code (ACCESS-005 R1),
    /// recorded with its join.
    pub(crate) const MACHINE: u64 = 5;

    /// A person is registered.
    pub(crate) const REGISTER_PERSON: u64 = 1;
    /// An agent is registered under its responsible person.
    pub(crate) const REGISTER_AGENT: u64 = 2;
    /// A display profile is replaced.
    pub(crate) const CHANGE_PROFILE: u64 = 3;
    /// A login is bound to a person.
    pub(crate) const BIND_LOGIN: u64 = 4;
    /// A lifecycle transition.
    pub(crate) const TRANSITION: u64 = 5;
    /// An accepted link-audit observation.
    pub(crate) const LINK_AUDIT: u64 = 6;
    /// An active person bound to the authenticated actor, in one leaf.
    pub(crate) const SETUP_PERSON: u64 = 7;
    /// Registration with an explicit reporting edge.
    pub(crate) const REPORTING_REGISTRATION: u64 = 8;
    /// A reporting edge and accountable subtree change.
    pub(crate) const REPORTS_TO_CHANGED: u64 = 9;
    /// A change an agent made through MCP.
    pub(crate) const AGENT_CALL: u64 = 10;

    pub(crate) fn change(value: &Change) -> u64 {
        match value {
            Change::SetupPerson { .. } => SETUP_PERSON,
            Change::RegisterPerson { .. } => REGISTER_PERSON,
            Change::RegisterAgent { .. } => REGISTER_AGENT,
            Change::ReportingRegistration { .. } => REPORTING_REGISTRATION,
            Change::ReportsToChanged { .. } => REPORTS_TO_CHANGED,
            Change::ChangeProfile { .. } => CHANGE_PROFILE,
            Change::BindLogin { .. } => BIND_LOGIN,
            Change::Transition { .. } => TRANSITION,
            Change::LinkAudit(_) => LINK_AUDIT,
            Change::AgentCall(_) => AGENT_CALL,
        }
    }

    /// The actor signed in through OIDC.
    pub(crate) const OIDC: u64 = 1;
    /// An agent the actor is responsible for signed the request.
    pub(crate) const AGENT_SIGNATURE: u64 = 2;
    /// Closed historical code: no actor key 5 is the operator; a 16-byte
    /// key 5 is a service-account bearer. Future methods need a fresh code.
    pub(crate) const OPERATOR_OR_SERVICE_ACCOUNT: u64 = 3;
    /// A live pass authenticated the agent whose id is carried under key 5.
    pub(crate) const AGENT_PASS: u64 = 4;

    pub(crate) fn method(value: AuthMethod) -> u64 {
        match value {
            AuthMethod::Oidc => OIDC,
            AuthMethod::AgentSignature(_) => AGENT_SIGNATURE,
            AuthMethod::AgentPass(_) => AGENT_PASS,
            AuthMethod::Operator | AuthMethod::ServiceAccountBearer(_) => {
                OPERATOR_OR_SERVICE_ACCOUNT
            }
        }
    }

    /// The method `code` names with the agent id the actor carried, or what
    /// is wrong with the two. An OIDC sign-in and the operator name no
    /// principal; agent signatures and service-account bearers name one.
    pub(crate) fn method_from(
        code: u64,
        agent: Option<AgentId>,
    ) -> Result<AuthMethod, &'static str> {
        match (code, agent) {
            (OIDC, None) => Ok(AuthMethod::Oidc),
            (AGENT_SIGNATURE, Some(agent)) => Ok(AuthMethod::AgentSignature(agent)),
            (AGENT_PASS, Some(agent)) => Ok(AuthMethod::AgentPass(agent)),
            (OPERATOR_OR_SERVICE_ACCOUNT, None) => Ok(AuthMethod::Operator),
            (OPERATOR_OR_SERVICE_ACCOUNT, Some(account)) => Ok(AuthMethod::ServiceAccountBearer(
                crate::ServiceAccountId::from_bytes(*account.as_bytes()),
            )),
            (OIDC, Some(_)) => Err("an OIDC actor carries no agent id under key 5"),
            (AGENT_SIGNATURE, None) => {
                Err("an agent-signature actor carries the agent's id under key 5")
            }
            (AGENT_PASS, None) => Err("an agent-pass actor carries the agent's id under key 5"),
            _ => Err("an authentication method code is 1, 2, 3 or 4"),
        }
    }

    pub(crate) fn state(value: LifecycleState) -> u64 {
        match value {
            LifecycleState::Registered => 1,
            LifecycleState::Active => 2,
            LifecycleState::Suspended => 3,
            LifecycleState::Retired => 4,
        }
    }

    pub(crate) fn state_from(code: u64) -> Option<LifecycleState> {
        match code {
            1 => Some(LifecycleState::Registered),
            2 => Some(LifecycleState::Active),
            3 => Some(LifecycleState::Suspended),
            4 => Some(LifecycleState::Retired),
            _ => None,
        }
    }

    pub(crate) fn transition(value: Transition) -> u64 {
        match value {
            Transition::Activate => 1,
            Transition::Suspend => 2,
            Transition::Reinstate => 3,
            Transition::Retire => 4,
        }
    }

    pub(crate) fn transition_from(code: u64) -> Option<Transition> {
        match code {
            1 => Some(Transition::Activate),
            2 => Some(Transition::Suspend),
            3 => Some(Transition::Reinstate),
            4 => Some(Transition::Retire),
            _ => None,
        }
    }

    pub(crate) fn link(value: LinkChange) -> u64 {
        match value {
            LinkChange::Linked => 1,
            LinkChange::Unlinked => 2,
        }
    }

    pub(crate) fn link_from(code: u64) -> Option<LinkChange> {
        match code {
            1 => Some(LinkChange::Linked),
            2 => Some(LinkChange::Unlinked),
            _ => None,
        }
    }
}
