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

use crate::binding::{ISSUER_MAX_BYTES, LoginBinding};
use crate::error::IdentityError;
use crate::id::{IdentityId, PersonId};
use crate::lifecycle::{LifecycleState, Transition};
use crate::operation::OperationId;
use crate::profile::Profile;
use crate::provenance::{Actor, AuthMethod};

/// The envelope version this crate writes and reads.
pub const EVENT_VERSION: u64 = 1;

/// The longest source operation id a link-audit observation may carry, in bytes.
pub const SOURCE_OPERATION_ID_MAX_BYTES: usize = 128;

/// The longest reason a lifecycle transition may give, in bytes.
pub const REASON_MAX_BYTES: usize = 1024;

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
        if source_operation_id.is_empty()
            || source_operation_id.len() > SOURCE_OPERATION_ID_MAX_BYTES
        {
            return Err(IdentityError::ChangeMismatch {
                reason: "a source operation id is between 1 and 128 bytes",
            });
        }
        if observer.is_empty() || observer.len() > ISSUER_MAX_BYTES {
            return Err(IdentityError::ChangeMismatch {
                reason: "an observation names the issuer that observed it, in 1 to 2048 bytes",
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
    /// A person is registered with a display profile.
    RegisterPerson {
        /// How the person is shown.
        profile: Profile,
    },
    /// An agent is registered under the person responsible for it, for life.
    RegisterAgent {
        /// The signed-in person who registered the agent.
        responsible: PersonId,
        /// How the agent is shown.
        profile: Profile,
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

/// Refuse `change` if it does not fit `identity`.
fn check_fit(identity: IdentityId, change: &Change) -> Result<(), IdentityError> {
    match (change, identity) {
        (Change::RegisterPerson { .. }, IdentityId::Agent(_)) => {
            Err(IdentityError::ChangeMismatch {
                reason: "a person registration names a person",
            })
        }
        (Change::RegisterAgent { .. }, IdentityId::Person(_)) => {
            Err(IdentityError::ChangeMismatch {
                reason: "an agent registration names an agent",
            })
        }
        (Change::BindLogin { .. }, IdentityId::Agent(_)) => Err(IdentityError::ChangeMismatch {
            reason: "a login is bound to a person, never to an agent",
        }),
        (Change::LinkAudit(_), IdentityId::Agent(_)) => Err(IdentityError::ChangeMismatch {
            reason: "a link observation names the person the login belongs to",
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
            if reason.len() > REASON_MAX_BYTES {
                return Err(IdentityError::ChangeMismatch {
                    reason: "a transition's reason is longer than 1024 bytes",
                });
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

impl IdentityEvent {
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
    use super::{AuthMethod, Change, LifecycleState, LinkChange, Transition};

    /// An identity that is a person.
    pub(crate) const PERSON: u64 = 1;
    /// An identity that is an agent.
    pub(crate) const AGENT: u64 = 2;

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

    pub(crate) fn change(value: &Change) -> u64 {
        match value {
            Change::RegisterPerson { .. } => REGISTER_PERSON,
            Change::RegisterAgent { .. } => REGISTER_AGENT,
            Change::ChangeProfile { .. } => CHANGE_PROFILE,
            Change::BindLogin { .. } => BIND_LOGIN,
            Change::Transition { .. } => TRANSITION,
            Change::LinkAudit(_) => LINK_AUDIT,
        }
    }

    pub(crate) fn method(value: AuthMethod) -> u64 {
        match value {
            AuthMethod::Oidc => 1,
        }
    }

    pub(crate) fn method_from(code: u64) -> Option<AuthMethod> {
        match code {
            1 => Some(AuthMethod::Oidc),
            _ => None,
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
