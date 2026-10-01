//! The directory as its events say it is.
//!
//! The projection is rebuilt from the log when the directory opens, one event
//! at a time, and advanced by each event as it is committed. Both paths go
//! through [`Projection::apply`], so the directory answered after a reopen is
//! the one that was answered live. [`Projection::check`] answers whether an
//! event would apply without applying it, so a change is refused by name
//! before it is signed and never after it is in the log.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Arc;

use crate::binding::LoginBinding;
use crate::error::IdentityError;
use crate::event::{Change, IdentityEvent};
use crate::id::{AgentId, IdentityId, PersonId};
use crate::lifecycle::LifecycleState;
use crate::operation::OperationId;
use crate::profile::Profile;
use crate::signer::Entry;

#[path = "projection_reporting.rs"]
mod reporting;

#[path = "projection_accounts.rs"]
pub mod accounts;

#[path = "projection_draft.rs"]
pub mod draft;

/// The inactive identity that interrupts a reporting chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReportingGap {
    /// The identity at the gap.
    pub identity: IdentityId,
    /// Its recorded lifecycle state.
    pub state: LifecycleState,
}

/// One identity, as the directory holds it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    profile: Profile,
    state: LifecycleState,
    responsible: Option<PersonId>,
    reports_to: Option<IdentityId>,
    reporting_gap: Option<ReportingGap>,
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

    /// An agent's current accountable person. `None` for a person.
    pub fn responsible(&self) -> Option<PersonId> {
        self.responsible
    }

    /// The immediate reporting target recorded for an agent.
    pub fn reports_to(&self) -> Option<IdentityId> {
        self.reports_to
    }

    /// The current gap in an agent's reporting chain, if one exists.
    pub fn reporting_gap(&self) -> Option<ReportingGap> {
        self.reporting_gap
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
    records: Arc<BTreeMap<IdentityId, Record>>,
    people: Arc<BTreeMap<String, PersonId>>,
    agents_by_person: Arc<BTreeMap<PersonId, BTreeSet<IdentityId>>>,
    reporting_children: Arc<BTreeMap<IdentityId, BTreeSet<IdentityId>>>,
    agent_count: usize,
    bindings: Arc<HashMap<LoginBinding, PersonId>>,
    agent_bindings: Arc<HashMap<LoginBinding, AgentId>>,
    operations: Arc<HashMap<OperationId, u64>>,
    link_sources: Arc<HashMap<String, u64>>,
    accounts: Arc<accounts::Accounts>,
    drafts: Arc<BTreeMap<OperationId, Arc<draft::DraftRecord>>>,
}

fn unknown(identity: IdentityId) -> IdentityError {
    IdentityError::IdentityUnknown {
        identity: identity.to_string(),
    }
}

impl Projection {
    /// A request snapshot sharing every directory index without copying its records.
    #[must_use]
    pub fn shared(&self) -> Self {
        self.with_accounts(Arc::clone(&self.accounts))
    }

    /// A shared directory snapshot with separately indexed service accounts.
    #[must_use]
    pub fn with_accounts(&self, accounts: Arc<accounts::Accounts>) -> Self {
        Self {
            records: Arc::clone(&self.records),
            people: Arc::clone(&self.people),
            agents_by_person: Arc::clone(&self.agents_by_person),
            reporting_children: Arc::clone(&self.reporting_children),
            agent_count: self.agent_count,
            bindings: Arc::clone(&self.bindings),
            agent_bindings: Arc::clone(&self.agent_bindings),
            operations: Arc::clone(&self.operations),
            link_sources: Arc::clone(&self.link_sources),
            accounts,
            drafts: Arc::clone(&self.drafts),
        }
    }

    /// Add an account from the separately signed service-account log to a
    /// request's projection. Call only on a clone after settling that log.
    /// This never creates a login binding or changes the directory's log.
    /// The recorded owner must be a known person; retirement and suspension
    /// of that person also stop the account from exercising grants.
    pub fn service_account(
        &mut self,
        id: crate::ServiceAccountId,
        owner: PersonId,
        profile: Profile,
        retired: bool,
        created_by: LoginBinding,
    ) -> Result<(), IdentityError> {
        let person = self
            .record(IdentityId::Person(owner))
            .ok_or_else(|| unknown(IdentityId::Person(owner)))?;
        let state = if retired {
            LifecycleState::Retired
        } else {
            person.state()
        };
        let identity = IdentityId::ServiceAccount(id);
        if self.records.contains_key(&identity) {
            return Err(IdentityError::AlreadyRegistered {
                identity: identity.to_string(),
            });
        }
        Arc::make_mut(&mut self.records).insert(
            identity,
            Record {
                profile,
                state,
                responsible: Some(owner),
                reports_to: None,
                reporting_gap: None,
                bindings: Vec::new(),
                registered_by: created_by,
                events: Vec::new(),
            },
        );
        Ok(())
    }

    /// An empty directory.
    pub fn new() -> Self {
        Self::default()
    }

    /// The identity `id`, if the directory holds it.
    pub fn record(&self, id: IdentityId) -> Option<&Record> {
        self.records
            .get(&id)
            .or_else(|| self.accounts.record(id, self))
    }

    /// Every identity, in identifier order.
    pub fn records(&self) -> impl Iterator<Item = (&IdentityId, &Record)> {
        self.records.iter().chain(self.accounts.records(self))
    }

    /// People in wire identifier order, starting after the supplied identifier.
    pub fn people(&self, after: std::ops::Bound<&str>) -> impl Iterator<Item = (&str, PersonId)> {
        self.people
            .range::<str, _>((after, std::ops::Bound::Unbounded))
            .map(|(id, person)| (id.as_str(), *person))
    }

    /// The maintained number of people.
    pub fn people_count(&self) -> usize {
        self.people.len()
    }

    /// The maintained number of agents answering to people.
    pub fn people_agents_count(&self) -> usize {
        self.agent_count
    }

    /// The maintained number of one person's agents.
    pub fn person_agents_count(&self, person: PersonId) -> usize {
        self.agents_by_person.get(&person).map_or(0, BTreeSet::len)
    }

    /// The named person's agents, without visiting unrelated directory records.
    pub fn agents_of(
        &self,
        person: PersonId,
    ) -> impl Iterator<Item = Result<(&IdentityId, &Record), IdentityError>> {
        self.agents_by_person
            .get(&person)
            .into_iter()
            .flat_map(|agents| agents.iter())
            .map(move |identity| {
                let held = self.records.get_key_value(identity).ok_or_else(|| {
                    IdentityError::LogUnavailable {
                        reason: format!(
                            "person {person}'s agent index names missing identity {identity}"
                        ),
                    }
                })?;
                if !matches!(held.0, IdentityId::Agent(_)) || held.1.responsible != Some(person) {
                    return Err(IdentityError::LogUnavailable {
                        reason: format!(
                            "person {person}'s agent index disagrees with identity {identity}"
                        ),
                    });
                }
                Ok(held)
            })
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
            | Change::ReportingRegistration { .. }
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
            Change::ReportingRegistration {
                responsible,
                reports_to,
                ..
            } => {
                if self.resolve_reporting(*reports_to, None)? != *responsible {
                    return Err(IdentityError::ChangeMismatch {
                        reason: "registration responsibility differs from its reporting chain",
                    });
                }
                Ok(())
            }
            Change::ReportsToChanged {
                from,
                to,
                responsible_from,
                responsible_to,
            } => self.check_reporting_change(
                identity,
                *from,
                *to,
                *responsible_from,
                *responsible_to,
            ),
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

    /// Advance the directory by the leaf `entry`, committed at log index `index`.
    pub fn apply_entry(&mut self, entry: &Entry, index: u64) -> Result<(), IdentityError> {
        match entry {
            Entry::Identity(event) => self.apply(event, index),
            Entry::Install(event) => self.apply_install(event, index),
            Entry::Draft(event) => self.apply_draft(event, index),
        }
    }

    /// Advance the directory by `event`, committed at log index `index`.
    pub fn apply(&mut self, event: &IdentityEvent, index: u64) -> Result<(), IdentityError> {
        self.check(event)?;
        let identity = event.identity();
        Arc::make_mut(&mut self.operations).insert(event.operation(), index);
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
                Arc::make_mut(&mut self.records).insert(identity, record);
                Arc::make_mut(&mut self.bindings).insert(registered_by, person);
            }
            Change::RegisterPerson { profile } => {
                Arc::make_mut(&mut self.records)
                    .insert(identity, fresh(profile, None, registered_by, index));
            }
            Change::RegisterAgent {
                responsible,
                profile,
            } => {
                let record = fresh(profile, Some(*responsible), registered_by, index);
                self.register_reporting(identity, record, IdentityId::Person(*responsible))?;
            }
            Change::ReportingRegistration {
                responsible,
                profile,
                reports_to,
            } => {
                let record = fresh(profile, Some(*responsible), registered_by, index);
                self.register_reporting(identity, record, *reports_to)?;
            }
            Change::ReportsToChanged { to, .. } => {
                self.apply_reporting_change(identity, *to, index)?;
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
                        Arc::make_mut(&mut self.bindings).insert(binding.clone(), person);
                    }
                    IdentityId::Agent(agent) => {
                        Arc::make_mut(&mut self.agent_bindings).insert(binding.clone(), agent);
                    }
                    IdentityId::ServiceAccount(_) => {
                        return Err(IdentityError::ChangeMismatch {
                            reason: "a service account never acquires a sign-in binding",
                        });
                    }
                }
            }
            Change::Transition { to, .. } => {
                let record = self.held(identity)?;
                record.state = *to;
                record.events.push(index);
                self.refresh_reporting_children(identity, None)?;
            }
            Change::LinkAudit(seen) => {
                self.held(identity)?.events.push(index);
                Arc::make_mut(&mut self.link_sources)
                    .insert(seen.source_operation_id().to_owned(), index);
            }
        }
        if let IdentityId::Person(person) = identity {
            Arc::make_mut(&mut self.people).insert(identity.to_string(), person);
        }
        Ok(())
    }

    fn held(&mut self, identity: IdentityId) -> Result<&mut Record, IdentityError> {
        Arc::make_mut(&mut self.records)
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
        reports_to: None,
        reporting_gap: None,
        bindings: Vec::new(),
        registered_by,
        events: vec![index],
    }
}

#[path = "projection_state.rs"]
pub(crate) mod state;

#[path = "projection_install.rs"]
mod install;

#[cfg(test)]
#[path = "projection_reporting_tests.rs"]
mod reporting_tests;

#[cfg(test)]
#[path = "projection_shared_tests.rs"]
mod shared_tests;
