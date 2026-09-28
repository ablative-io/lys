//! The directory as its events say it is.
//!
//! The projection is rebuilt from the log when the directory opens, one event
//! at a time, and advanced by each event as it is committed. Both paths go
//! through [`Projection::apply`], so the directory answered after a reopen is
//! the one that was answered live. [`Projection::check`] answers whether an
//! event would apply without applying it, so a change is refused by name
//! before it is signed and never after it is in the log.

use std::collections::{BTreeMap, HashMap};

use crate::binding::LoginBinding;
use crate::error::IdentityError;
use crate::event::{Change, IdentityEvent};
use crate::id::{AgentId, IdentityId, PersonId};
use crate::lifecycle::LifecycleState;
use crate::operation::OperationId;
use crate::profile::Profile;

/// One identity, as the directory holds it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    profile: Profile,
    state: LifecycleState,
    responsible: Option<PersonId>,
    bindings: Vec<LoginBinding>,
    registered_by: LoginBinding,
    events: Vec<u64>,
}

impl Record {
    /// How the identity is shown.
    pub fn profile(&self) -> &Profile {
        &self.profile
    }

    /// The identity's lifecycle state. Recorded, never enforced.
    pub fn state(&self) -> LifecycleState {
        self.state
    }

    /// An agent's responsible person, for life. `None` for a person.
    pub fn responsible(&self) -> Option<PersonId> {
        self.responsible
    }

    /// The logins bound to a person, in the order they were bound.
    pub fn bindings(&self) -> &[LoginBinding] {
        &self.bindings
    }

    /// The login of the person who registered the identity.
    pub fn registered_by(&self) -> &LoginBinding {
        &self.registered_by
    }

    /// The log indices of every event about the identity, oldest first.
    pub fn events(&self) -> &[u64] {
        &self.events
    }
}

/// Every identity the directory holds, and the indexes a change is judged against.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Projection {
    records: BTreeMap<IdentityId, Record>,
    bindings: HashMap<LoginBinding, PersonId>,
    agent_bindings: HashMap<LoginBinding, AgentId>,
    operations: HashMap<OperationId, u64>,
    link_sources: HashMap<String, u64>,
}

fn unknown(identity: IdentityId) -> IdentityError {
    IdentityError::IdentityUnknown {
        identity: identity.to_string(),
    }
}

impl Projection {
    /// An empty directory.
    pub fn new() -> Self {
        Self::default()
    }

    /// The identity `id`, if the directory holds it.
    pub fn record(&self, id: IdentityId) -> Option<&Record> {
        self.records.get(&id)
    }

    /// Every identity, in identifier order.
    pub fn records(&self) -> impl Iterator<Item = (&IdentityId, &Record)> {
        self.records.iter()
    }

    /// The person a login is bound to.
    pub fn person_for(&self, binding: &LoginBinding) -> Option<PersonId> {
        self.bindings.get(binding).copied()
    }

    /// The agent a login is bound to, as its own machine account.
    pub fn agent_for(&self, binding: &LoginBinding) -> Option<AgentId> {
        self.agent_bindings.get(binding).copied()
    }

    /// The log index of the event an operation id made.
    pub fn operation(&self, operation: OperationId) -> Option<u64> {
        self.operations.get(&operation).copied()
    }

    /// The log index of the event a link-audit source operation made.
    pub fn link_source(&self, source_operation_id: &str) -> Option<u64> {
        self.link_sources.get(source_operation_id).copied()
    }

    /// Whether `event` would apply to the directory as it stands, refused by name if not.
    pub fn check(&self, event: &IdentityEvent) -> Result<(), IdentityError> {
        if self.operations.contains_key(&event.operation()) {
            return Err(IdentityError::OperationReused {
                operation: event.operation().to_string(),
            });
        }
        let identity = event.identity();
        let held = self.records.get(&identity);
        match event.change() {
            Change::SetupPerson { .. }
            | Change::RegisterPerson { .. }
            | Change::RegisterAgent { .. }
                if held.is_some() =>
            {
                Err(IdentityError::AlreadyRegistered {
                    identity: identity.to_string(),
                })
            }
            Change::SetupPerson { .. } => match self.bindings.get(event.actor().binding()) {
                Some(person) => Err(IdentityError::AlreadyBootstrapped {
                    person: person.to_string(),
                }),
                None => Ok(()),
            },
            Change::RegisterPerson { .. } => Ok(()),
            Change::RegisterAgent { responsible, .. } => {
                if self.records.contains_key(&IdentityId::Person(*responsible)) {
                    Ok(())
                } else {
                    Err(unknown(IdentityId::Person(*responsible)))
                }
            }
            Change::ChangeProfile { .. } => held.map(|_| ()).ok_or_else(|| unknown(identity)),
            Change::BindLogin { binding } => {
                held.ok_or_else(|| unknown(identity))?;
                match self.bindings.get(binding) {
                    Some(person) => Err(IdentityError::BindingTaken {
                        issuer: binding.issuer().to_owned(),
                        subject: binding.subject().to_owned(),
                        person: person.to_string(),
                    }),
                    None => Ok(()),
                }
            }
            Change::Transition { from, .. } => {
                let record = held.ok_or_else(|| unknown(identity))?;
                if record.state == *from {
                    Ok(())
                } else {
                    Err(IdentityError::StateMismatch {
                        identity: identity.to_string(),
                        recorded: record.state,
                        from: *from,
                    })
                }
            }
            Change::LinkAudit(seen) => {
                held.ok_or_else(|| unknown(identity))?;
                if self.link_sources.contains_key(seen.source_operation_id()) {
                    Err(IdentityError::LinkSourceSeen {
                        source_operation_id: seen.source_operation_id().to_owned(),
                    })
                } else {
                    Ok(())
                }
            }
        }
    }

    /// Advance the directory by `event`, committed at log index `index`.
    pub fn apply(&mut self, event: &IdentityEvent, index: u64) -> Result<(), IdentityError> {
        self.check(event)?;
        let identity = event.identity();
        self.operations.insert(event.operation(), index);
        let registered_by = event.actor().binding().clone();
        match event.change() {
            Change::SetupPerson { profile } => {
                let IdentityId::Person(person) = identity else {
                    return Err(IdentityError::ChangeMismatch {
                        reason: "setup names a person",
                    });
                };
                let mut record = fresh(profile, None, registered_by.clone(), index);
                record.state = LifecycleState::Active;
                record.bindings.push(registered_by.clone());
                self.records.insert(identity, record);
                self.bindings.insert(registered_by, person);
            }
            Change::RegisterPerson { profile } => {
                self.records
                    .insert(identity, fresh(profile, None, registered_by, index));
            }
            Change::RegisterAgent {
                responsible,
                profile,
            } => {
                self.records.insert(
                    identity,
                    fresh(profile, Some(*responsible), registered_by, index),
                );
            }
            Change::ChangeProfile { profile } => {
                let record = self.held(identity)?;
                record.profile = profile.clone();
                record.events.push(index);
            }
            Change::BindLogin { binding } => {
                let record = self.held(identity)?;
                record.bindings.push(binding.clone());
                record.events.push(index);
                match identity {
                    IdentityId::Person(person) => {
                        self.bindings.insert(binding.clone(), person);
                    }
                    IdentityId::Agent(agent) => {
                        self.agent_bindings.insert(binding.clone(), agent);
                    }
                }
            }
            Change::Transition { to, .. } => {
                let record = self.held(identity)?;
                record.state = *to;
                record.events.push(index);
            }
            Change::LinkAudit(seen) => {
                self.held(identity)?.events.push(index);
                self.link_sources
                    .insert(seen.source_operation_id().to_owned(), index);
            }
        }
        Ok(())
    }

    fn held(&mut self, identity: IdentityId) -> Result<&mut Record, IdentityError> {
        self.records
            .get_mut(&identity)
            .ok_or_else(|| unknown(identity))
    }
}

fn fresh(
    profile: &Profile,
    responsible: Option<PersonId>,
    registered_by: LoginBinding,
    index: u64,
) -> Record {
    Record {
        profile: profile.clone(),
        state: LifecycleState::Registered,
        responsible,
        bindings: Vec::new(),
        registered_by,
        events: vec![index],
    }
}
