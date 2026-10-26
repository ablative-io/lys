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
use crate::id::{AgentId, IdentityId, OrganisationId, PersonId};
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

#[path = "projection_product_draft.rs"]
pub mod product_draft;

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
    organisation: Option<OrganisationId>,
    founder: Option<PersonId>,
}

impl Record {
    /// The organisation the identity belongs to: its own for an
    /// organisation, the one it was registered under for a person, its
    /// responsible person's for an agent. `None` only for a record written
    /// before organisations and not yet attached (DIRECTORY-109 R2).
    #[must_use]
    pub fn organisation(&self) -> Option<OrganisationId> {
        self.organisation
    }

    /// The person who founded an organisation. `None` for every other kind.
    #[must_use]
    pub fn founder(&self) -> Option<PersonId> {
        self.founder
    }

    /// How the identity is shown.
    #[must_use]
    pub fn profile(&self) -> &Profile {
        &self.profile
    }

    /// The identity's lifecycle state. Recorded, never enforced.
    #[must_use]
    pub fn state(&self) -> LifecycleState {
        self.state
    }

    /// An agent's current accountable person. `None` for a person.
    #[must_use]
    pub fn responsible(&self) -> Option<PersonId> {
        self.responsible
    }

    /// The immediate reporting target recorded for an agent.
    #[must_use]
    pub fn reports_to(&self) -> Option<IdentityId> {
        self.reports_to
    }

    /// The current gap in an agent's reporting chain, if one exists.
    #[must_use]
    pub fn reporting_gap(&self) -> Option<ReportingGap> {
        self.reporting_gap
    }

    /// The logins bound to a person, in the order they were bound.
    #[must_use]
    pub fn bindings(&self) -> &[LoginBinding] {
        &self.bindings
    }

    /// The login of the person who registered the identity.
    #[must_use]
    pub fn registered_by(&self) -> &LoginBinding {
        &self.registered_by
    }

    /// The log indices of every event about the identity, oldest first.
    #[must_use]
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
    product_drafts: Arc<BTreeMap<OperationId, Arc<product_draft::ProductDraftRecord>>>,
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
            product_drafts: Arc::clone(&self.product_drafts),
        }
    }

    /// Add an account from the separately signed service-account log to a
    /// request's projection. Call only on a clone after settling that log.
    /// This never creates a login binding or changes the directory's log.
    /// The recorded owner must be a known person; retirement and suspension
    /// of that person also stop the account from exercising grants.
    ///
    /// # Errors
    /// - `IdentityError::IdentityUnknown` if the account owner is not a known person.
    /// - `IdentityError::AlreadyRegistered` if the account identity already exists.
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
        let organisation = person.organisation;
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
                organisation,
                founder: None,
            },
        );
        Ok(())
    }

    /// An empty directory.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The identity `id`, if the directory holds it.
    #[must_use]
    pub fn record(&self, id: IdentityId) -> Option<&Record> {
        self.records
            .get(&id)
            .or_else(|| self.accounts.record(id, self))
    }

    /// The organisation an account held in the separately folded accounts
    /// belongs to: its owner's.
    pub(crate) fn account_organisation(&self, id: IdentityId) -> Option<OrganisationId> {
        self.accounts.organisation_of(id, self)
    }

    /// Refuse, by name, a registration under an organisation the directory
    /// does not hold or has retired.
    fn organisation_open(&self, organisation: OrganisationId) -> Result<(), IdentityError> {
        let identity = IdentityId::Organisation(organisation);
        let record = self
            .records
            .get(&identity)
            .ok_or_else(|| unknown(identity))?;
        if record.state == LifecycleState::Retired {
            return Err(IdentityError::ChangeMismatch {
                reason: "nothing is registered under a retired organisation",
            });
        }
        Ok(())
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
    #[must_use]
    pub fn people_count(&self) -> usize {
        self.people.len()
    }

    /// The maintained number of agents answering to people.
    #[must_use]
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
                let agent = match held.0 {
                    IdentityId::Agent(_) => true,
                    IdentityId::Person(_)
                    | IdentityId::ServiceAccount(_)
                    | IdentityId::Connector(_)
                    | IdentityId::Machine(_)
                    | IdentityId::Organisation(_) => false,
                };
                if !agent || held.1.responsible != Some(person) {
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
    #[must_use]
    pub fn person_for(&self, binding: &LoginBinding) -> Option<PersonId> {
        self.bindings.get(binding).copied()
    }

    /// The agent a login is bound to, as its own machine account.
    #[must_use]
    pub fn agent_for(&self, binding: &LoginBinding) -> Option<AgentId> {
        self.agent_bindings.get(binding).copied()
    }

    /// The log index of the event an operation id made.
    #[must_use]
    pub fn operation(&self, operation: OperationId) -> Option<u64> {
        self.operations.get(&operation).copied()
    }

    /// The log index of the event a link-audit source operation made.
    #[must_use]
    pub fn link_source(&self, source_operation_id: &str) -> Option<u64> {
        self.link_sources.get(source_operation_id).copied()
    }

    /// Whether `event` would apply to the directory as it stands, refused by name if not.
    ///
    /// # Errors
    /// - `IdentityError::OperationReused` if the operation is already recorded.
    /// - `IdentityError::AlreadyRegistered` if a registration names an existing identity.
    /// - `IdentityError::AlreadyBootstrapped` if setup repeats an existing login binding.
    /// - `IdentityError::IdentityUnknown` if an identity or responsible person is absent.
    /// - `IdentityError::BindingTaken` if a login already belongs to a person.
    /// - `IdentityError::StateMismatch` if a transition names another current state.
    /// - `IdentityError::LinkSourceSeen` if a link observation was already accepted.
    /// - `IdentityError::ChangeMismatch` if a reporting change disagrees with its identity, edge or accountable person.
    /// - `IdentityError::AnswersToUnknown` if the target is absent or is not a person or agent.
    /// - `IdentityError::AnswersToInactive` if the immediate target is not active.
    /// - `IdentityError::AnswersToCycle` if the reporting walk revisits an identity.
    /// - `IdentityError::NoAccountablePerson` if the chain has a missing or inactive ancestor or no reporting edge.
    /// - `IdentityError::OrganisationMismatch` if a reporting change would move an agent across organisations, or an attachment names a record already attached.
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
            | Change::FoundOrganisation { .. }
                if held.is_some() =>
            {
                Err(IdentityError::AlreadyRegistered {
                    identity: identity.to_string(),
                })
            }
            Change::SetupPerson { organisation, .. } => {
                if let Some(organisation) = organisation {
                    self.organisation_open(*organisation)?;
                }
                match self.bindings.get(event.actor().binding()) {
                    Some(person) => Err(IdentityError::AlreadyBootstrapped {
                        person: person.to_string(),
                    }),
                    None => Ok(()),
                }
            }
            Change::RegisterPerson { organisation, .. } => {
                organisation.map_or(Ok(()), |organisation| self.organisation_open(organisation))
            }
            // The founder is named, not yet held: the batch that founds an
            // organisation registers its founder in the next leaf.
            Change::FoundOrganisation { .. } => Ok(()),
            Change::AttachOrganisation { organisation } => {
                let record = held.ok_or_else(|| unknown(identity))?;
                self.organisation_open(*organisation)?;
                match record.organisation {
                    None => Ok(()),
                    Some(current) => Err(IdentityError::OrganisationMismatch {
                        identity: identity.to_string(),
                        organisation: current.to_string(),
                        named: organisation.to_string(),
                    }),
                }
            }
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
            Change::UnbindLogin { binding } => {
                let record = held.ok_or_else(|| unknown(identity))?;
                if record.bindings().contains(binding) {
                    Ok(())
                } else {
                    Err(IdentityError::ChangeMismatch {
                        reason: "an unbound login must be held by the event's person",
                    })
                }
            }
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
            Change::AgentCall(_) | Change::AdmissionWithdrawn { .. } => {
                held.map(drop).ok_or_else(|| unknown(identity))
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
}

#[path = "projection_apply.rs"]
mod apply;

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
