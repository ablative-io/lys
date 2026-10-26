//! Applying committed identity changes and maintaining their record indexes.

use super::{
    Arc, Change, Entry, IdentityError, IdentityEvent, IdentityId, LifecycleState, LoginBinding,
    PersonId, Profile, Projection, Record, unknown,
};

impl Projection {
    /// Advance the directory by the leaf `entry`, committed at log index `index`.
    ///
    /// # Errors
    /// - `IdentityError::OperationReused` if the operation already names a committed event.
    /// - `IdentityError::AlreadyRegistered` if a registration repeats an identity.
    /// - `IdentityError::AlreadyBootstrapped` if setup repeats a bound login.
    /// - `IdentityError::IdentityUnknown` if an identity, responsible person or reporting descendant is absent.
    /// - `IdentityError::BindingTaken` if a login is already bound to another identity.
    /// - `IdentityError::BindingMalformed` if an issuer move produces an invalid login.
    /// - `IdentityError::StateMismatch` if a transition names another current state.
    /// - `IdentityError::LinkSourceSeen` if the link source operation was already accepted.
    /// - `IdentityError::ChangeMismatch` if identity, edge or responsibility fields disagree.
    /// - `IdentityError::AnswersToUnknown` if a reporting target is absent or of another kind.
    /// - `IdentityError::AnswersToInactive` if the immediate reporting target is inactive.
    /// - `IdentityError::AnswersToCycle` if a reporting edge creates a cycle.
    /// - `IdentityError::NoAccountablePerson` if a reporting chain cannot reach an active person.
    /// - `IdentityError::DraftChangeInvalid` if the draft shape, approver or closing app is invalid.
    /// - `IdentityError::DraftHashMismatch` if a decision or correction names another creation payload.
    /// - `IdentityError::DraftNotFound` if a decision names an absent draft.
    /// - `IdentityError::DraftNotPending` if a decision or close cannot apply in the current draft state.
    pub fn apply_entry(&mut self, entry: &Entry, index: u64) -> Result<(), IdentityError> {
        match entry {
            Entry::Identity(event) => self.apply(event, index),
            Entry::Install(event) => self.apply_install(event, index),
            Entry::Draft(event) => self.apply_draft(event, index),
            Entry::ProductDraft(event) => self.apply_product_draft(event, index),
        }
    }

    /// Advance the directory by `event`, committed at log index `index`.
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
    pub fn apply(&mut self, event: &IdentityEvent, index: u64) -> Result<(), IdentityError> {
        self.check(event)?;
        let identity = event.identity();
        Arc::make_mut(&mut self.operations).insert(event.operation(), index);
        let registered_by = event.actor().binding().clone();
        match event.change() {
            Change::SetupPerson {
                profile,
                organisation,
            } => {
                let person = match identity {
                    IdentityId::Person(person) => person,
                    IdentityId::Agent(_)
                    | IdentityId::ServiceAccount(_)
                    | IdentityId::Connector(_)
                    | IdentityId::Machine(_)
                    | IdentityId::Organisation(_) => {
                        return Err(IdentityError::ChangeMismatch {
                            reason: "setup names a person",
                        });
                    }
                };
                let mut record = fresh(profile, None, registered_by.clone(), index);
                record.state = LifecycleState::Active;
                record.organisation = *organisation;
                record.bindings.push(registered_by.clone());
                Arc::make_mut(&mut self.records).insert(identity, record);
                Arc::make_mut(&mut self.bindings).insert(registered_by, person);
            }
            Change::RegisterPerson {
                profile,
                organisation,
            } => {
                let mut record = fresh(profile, None, registered_by, index);
                record.organisation = *organisation;
                Arc::make_mut(&mut self.records).insert(identity, record);
            }
            Change::FoundOrganisation { profile, founder } => {
                let organisation = match identity {
                    IdentityId::Organisation(organisation) => organisation,
                    IdentityId::Person(_)
                    | IdentityId::Agent(_)
                    | IdentityId::ServiceAccount(_)
                    | IdentityId::Connector(_)
                    | IdentityId::Machine(_) => {
                        return Err(IdentityError::ChangeMismatch {
                            reason: "a founding names an organisation",
                        });
                    }
                };
                let mut record = fresh(profile, None, registered_by, index);
                record.organisation = Some(organisation);
                record.founder = Some(*founder);
                Arc::make_mut(&mut self.records).insert(identity, record);
            }
            Change::AttachOrganisation { organisation } => {
                let record = self.held(identity)?;
                record.organisation = Some(*organisation);
                record.events.push(index);
            }
            Change::RegisterAgent {
                responsible,
                profile,
            } => {
                let mut record = fresh(profile, Some(*responsible), registered_by, index);
                record.organisation = self.organisation_of(IdentityId::Person(*responsible));
                self.register_reporting(identity, record, IdentityId::Person(*responsible))?;
            }
            Change::ReportingRegistration {
                responsible,
                profile,
                reports_to,
            } => {
                let mut record = fresh(profile, Some(*responsible), registered_by, index);
                record.organisation = self.organisation_of(IdentityId::Person(*responsible));
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
            Change::UnbindLogin { binding } => {
                let record = self.held(identity)?;
                record.bindings.retain(|held| held != binding);
                record.events.push(index);
                Arc::make_mut(&mut self.bindings).remove(binding);
            }
            Change::BindLogin { binding } => {
                self.apply_login_binding(identity, binding, index)?;
            }
            Change::Transition { to, .. } => {
                let record = self.held(identity)?;
                record.state = *to;
                record.events.push(index);
                self.refresh_reporting_children(identity, None)?;
            }
            Change::AgentCall(_) | Change::AdmissionWithdrawn { .. } => {
                self.held(identity)?.events.push(index);
            }
            Change::LinkAudit(seen) => {
                self.held(identity)?.events.push(index);
                Arc::make_mut(&mut self.link_sources)
                    .insert(seen.source_operation_id().to_owned(), index);
            }
        }
        match identity {
            IdentityId::Person(person) => {
                Arc::make_mut(&mut self.people).insert(identity.to_string(), person);
            }
            // Only a person is indexed by name.
            IdentityId::Agent(_)
            | IdentityId::ServiceAccount(_)
            | IdentityId::Connector(_)
            | IdentityId::Machine(_)
            | IdentityId::Organisation(_) => {}
        }
        Ok(())
    }

    fn apply_login_binding(
        &mut self,
        identity: IdentityId,
        binding: &LoginBinding,
        index: u64,
    ) -> Result<(), IdentityError> {
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
            IdentityId::Connector(_) => {
                return Err(IdentityError::ChangeMismatch {
                    reason: "a connector never acquires a sign-in binding",
                });
            }
            IdentityId::Machine(_) => {
                return Err(IdentityError::ChangeMismatch {
                    reason: "a machine never acquires a sign-in binding",
                });
            }
            IdentityId::Organisation(_) => {
                return Err(IdentityError::ChangeMismatch {
                    reason: "an organisation never acquires a sign-in binding",
                });
            }
        }
        Ok(())
    }

    pub(super) fn held(&mut self, identity: IdentityId) -> Result<&mut Record, IdentityError> {
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
        organisation: None,
        founder: None,
    }
}
