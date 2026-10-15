//! Registrations, profiles, login bindings and lifecycle changes through one commit path.

use super::{
    Actor, AgentId, Change, Directory, IdentityError, IdentityEvent, IdentityId, LeafStore,
    OperationId, PersonId, Profile, Receipt, Transition,
};

impl<S: LeafStore> Directory<S> {
    /// Create one active person bound to the authenticated actor in one signed leaf.
    ///
    /// The service must admit the configured administrator before calling this.
    /// Retrying the same operation, login and profile returns its original receipt,
    /// including after a new authenticated session or a reopen. It never reactivates
    /// a person whose state was subsequently changed.
    ///
    /// # Errors
    /// - `IdentityError::LogUnavailable` if the directory is broken, the store cannot be reopened, or an adopted event is refused by the projection.
    /// - `IdentityError::LeafNotAnEvent` if reconciliation finds a missing or invalid signed leaf.
    /// - `IdentityError::OperationReused` if the operation already names a different request.
    /// - `IdentityError::InstallEntry` if a retried operation names an install leaf.
    /// - `IdentityError::DraftEntry` if a retried operation names a draft leaf.
    /// - `IdentityError::AppendRefused` if reconciliation proves this change was not committed.
    /// - `IdentityError::RandomSourceUnavailable` if the new person id cannot be generated.
    /// - `IdentityError::AlreadyRegistered` if the generated identity already exists.
    /// - `IdentityError::AlreadyBootstrapped` if the login already belongs to a person.
    pub fn setup_person(
        &mut self,
        actor: Actor,
        operation: OperationId,
        profile: Profile,
        recorded_at: u64,
    ) -> Result<(PersonId, Receipt), IdentityError> {
        self.settle()?;
        let change = Change::SetupPerson { profile };
        if let Some((event, receipt)) = self.answered(operation)? {
            if event.actor().binding() == actor.binding() && event.change() == &change {
                match event.identity() {
                    IdentityId::Person(person) => return Ok((person, receipt)),
                    IdentityId::Agent(_)
                    | IdentityId::ServiceAccount(_)
                    | IdentityId::Connector(_)
                    | IdentityId::Machine(_) => {}
                }
            }
            return Err(IdentityError::OperationReused {
                operation: operation.to_string(),
            });
        }
        if let Some(person) = self.projection.person_for(actor.binding()) {
            return Err(IdentityError::AlreadyBootstrapped {
                person: person.to_string(),
            });
        }
        let person = PersonId::generate()?;
        let event = IdentityEvent::new(
            operation,
            actor,
            IdentityId::Person(person),
            recorded_at,
            change,
        )?;
        Ok((person, self.commit(event)?))
    }

    /// Register a person, answered with their new enduring id.
    ///
    /// # Errors
    /// - `IdentityError::LogUnavailable` if the directory is broken, the store cannot be reopened, or an adopted event is refused by the projection.
    /// - `IdentityError::LeafNotAnEvent` if reconciliation finds a missing or invalid signed leaf.
    /// - `IdentityError::OperationReused` if the operation already names a different request.
    /// - `IdentityError::InstallEntry` if a retried operation names an install leaf.
    /// - `IdentityError::DraftEntry` if a retried operation names a draft leaf.
    /// - `IdentityError::AppendRefused` if reconciliation proves this change was not committed.
    /// - `IdentityError::RandomSourceUnavailable` if the new person id cannot be generated.
    /// - `IdentityError::AlreadyRegistered` if the generated identity already exists.
    pub fn register_person(
        &mut self,
        actor: Actor,
        operation: OperationId,
        profile: Profile,
        recorded_at: u64,
    ) -> Result<(PersonId, Receipt), IdentityError> {
        self.settle()?;
        let change = Change::RegisterPerson { profile };
        if let Some((IdentityId::Person(id), receipt)) =
            self.retry(operation, &actor, None, &change)?
        {
            return Ok((id, receipt));
        }
        let id = PersonId::generate()?;
        let event = IdentityEvent::new(
            operation,
            actor,
            IdentityId::Person(id),
            recorded_at,
            change,
        )?;
        Ok((id, self.commit(event)?))
    }

    /// Register an agent under `responsible`, the person who registers it, for life.
    ///
    /// Nothing is started, and no login, credential, handle or certificate is issued.
    ///
    /// # Errors
    /// - `IdentityError::LogUnavailable` if the directory is broken, the store cannot be reopened, or an adopted event is refused by the projection.
    /// - `IdentityError::LeafNotAnEvent` if reconciliation finds a missing or invalid signed leaf.
    /// - `IdentityError::OperationReused` if the operation already names a different request.
    /// - `IdentityError::InstallEntry` if a retried operation names an install leaf.
    /// - `IdentityError::DraftEntry` if a retried operation names a draft leaf.
    /// - `IdentityError::AppendRefused` if reconciliation proves this change was not committed.
    /// - `IdentityError::RandomSourceUnavailable` if the new agent id cannot be generated.
    /// - `IdentityError::AlreadyRegistered` if the generated identity already exists.
    /// - `IdentityError::IdentityUnknown` if the responsible person is absent.
    /// - `IdentityError::ChangeMismatch` if the responsible reporting target cannot supply accountability.
    pub fn register_agent(
        &mut self,
        actor: Actor,
        operation: OperationId,
        responsible: PersonId,
        profile: Profile,
        recorded_at: u64,
    ) -> Result<(AgentId, Receipt), IdentityError> {
        self.settle()?;
        let change = Change::RegisterAgent {
            responsible,
            profile,
        };
        if let Some((IdentityId::Agent(id), receipt)) =
            self.retry(operation, &actor, None, &change)?
        {
            return Ok((id, receipt));
        }
        let id = AgentId::generate()?;
        let event =
            IdentityEvent::new(operation, actor, IdentityId::Agent(id), recorded_at, change)?;
        Ok((id, self.commit(event)?))
    }

    /// Replace an identity's display profile.
    ///
    /// # Errors
    /// - `IdentityError::LogUnavailable` if the directory is broken, the store cannot be reopened, or an adopted event is refused by the projection.
    /// - `IdentityError::LeafNotAnEvent` if reconciliation finds a missing or invalid signed leaf.
    /// - `IdentityError::OperationReused` if the operation already names a different request.
    /// - `IdentityError::InstallEntry` if a retried operation names an install leaf.
    /// - `IdentityError::DraftEntry` if a retried operation names a draft leaf.
    /// - `IdentityError::AppendRefused` if reconciliation proves this change was not committed.
    /// - `IdentityError::IdentityUnknown` if the identity is absent.
    /// - `IdentityError::ChangeMismatch` if the identity kind is outside the directory event contract.
    pub fn change_profile(
        &mut self,
        actor: Actor,
        operation: OperationId,
        identity: IdentityId,
        profile: Profile,
        recorded_at: u64,
    ) -> Result<Receipt, IdentityError> {
        self.change(
            actor,
            operation,
            identity,
            Change::ChangeProfile { profile },
            recorded_at,
        )
    }

    /// Bind a login to a person. One login names at most one person.
    ///
    /// # Errors
    /// - `IdentityError::LogUnavailable` if the directory is broken, the store cannot be reopened, or an adopted event is refused by the projection.
    /// - `IdentityError::LeafNotAnEvent` if reconciliation finds a missing or invalid signed leaf.
    /// - `IdentityError::OperationReused` if the operation already names a different request.
    /// - `IdentityError::InstallEntry` if a retried operation names an install leaf.
    /// - `IdentityError::DraftEntry` if a retried operation names a draft leaf.
    /// - `IdentityError::AppendRefused` if reconciliation proves this change was not committed.
    /// - `IdentityError::IdentityUnknown` if the person is absent.
    /// - `IdentityError::BindingTaken` if the login is already bound to a person.
    pub fn bind_login(
        &mut self,
        actor: Actor,
        operation: OperationId,
        person: PersonId,
        binding: crate::binding::LoginBinding,
        recorded_at: u64,
    ) -> Result<Receipt, IdentityError> {
        self.change(
            actor,
            operation,
            IdentityId::Person(person),
            Change::BindLogin { binding },
            recorded_at,
        )
    }

    /// Move an identity through one lifecycle transition, from the state it is in.
    ///
    /// # Errors
    /// - `IdentityError::LogUnavailable` if the directory is broken, the store cannot be reopened, or an adopted event is refused by the projection.
    /// - `IdentityError::LeafNotAnEvent` if reconciliation finds a missing or invalid signed leaf.
    /// - `IdentityError::OperationReused` if the operation already names a different request.
    /// - `IdentityError::InstallEntry` if a retried operation names an install leaf.
    /// - `IdentityError::DraftEntry` if a retried operation names a draft leaf.
    /// - `IdentityError::AppendRefused` if reconciliation proves this change was not committed.
    /// - `IdentityError::IdentityUnknown` if the identity or a reporting descendant is absent.
    /// - `IdentityError::ChangeMismatch` if the identity kind or reporting accountability is invalid.
    /// - `IdentityError::TransitionRefused` if the lifecycle table has no such transition.
    /// - `IdentityError::ReasonRequired` if suspension or retirement has no nonblank reason.
    /// - `IdentityError::StateMismatch` if the recorded state differs from the transition source.
    pub fn transition(
        &mut self,
        actor: Actor,
        operation: OperationId,
        identity: IdentityId,
        transition: Transition,
        reason: &str,
        recorded_at: u64,
    ) -> Result<Receipt, IdentityError> {
        self.settle()?;
        let from = match self.answered(operation)? {
            Some((event, _)) => match event.change() {
                Change::Transition { from, .. } => *from,
                _ => {
                    return Err(IdentityError::OperationReused {
                        operation: operation.to_string(),
                    });
                }
            },
            None => self
                .projection
                .record(identity)
                .ok_or_else(|| IdentityError::IdentityUnknown {
                    identity: identity.to_string(),
                })?
                .state(),
        };
        let to = transition.target(from)?;
        self.change(
            actor,
            operation,
            identity,
            Change::Transition {
                transition,
                from,
                to,
                reason: reason.to_owned(),
            },
            recorded_at,
        )
    }
}
