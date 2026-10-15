//! One reporting request records one atomic event and replays its original answer.

use super::{
    Actor, AgentId, Change, Directory, IdentityError, IdentityEvent, IdentityId, LeafStore,
    OperationId, PersonId, Profile, Receipt, same_actor,
};

/// The stable answer to a reporting registration.
pub struct Registered {
    /// The agent registered.
    pub agent: AgentId,
    /// Its chosen reporting edge.
    pub reports_to: IdentityId,
    /// The accountable person when this operation was recorded.
    pub responsible: PersonId,
    /// The receipt of the atomic registration.
    pub receipt: Receipt,
}

/// The stable answer to a reporting-edge change.
pub struct Changed {
    /// The edge after this operation.
    pub reports_to: IdentityId,
    /// The accountable person before this operation.
    pub responsible_from: PersonId,
    /// The accountable person after this operation.
    pub responsible_to: PersonId,
    /// The one receipt proving the edge and responsibility transition.
    pub receipt: Receipt,
}

fn reused(operation: OperationId) -> IdentityError {
    IdentityError::OperationReused {
        operation: operation.to_string(),
    }
}

impl<S: LeafStore> Directory<S> {
    /// Register beneath a person or agent after resolving its active reporting chain.
    ///
    /// # Errors
    /// - `IdentityError::LogUnavailable` if the directory is broken, the store cannot be reopened, or an adopted event is refused by the projection.
    /// - `IdentityError::LeafNotAnEvent` if reconciliation finds a missing or invalid signed leaf.
    /// - `IdentityError::OperationReused` if the operation already names a different request.
    /// - `IdentityError::InstallEntry` if a retried operation names an install leaf.
    /// - `IdentityError::DraftEntry` if a retried operation names a draft leaf.
    /// - `IdentityError::AppendRefused` if reconciliation proves this change was not committed.
    /// - `IdentityError::AnswersToUnknown` if a reporting target is absent or is not a person or agent.
    /// - `IdentityError::AnswersToInactive` if the immediate target is inactive.
    /// - `IdentityError::AnswersToCycle` if the new edge creates a cycle.
    /// - `IdentityError::NoAccountablePerson` if the chain cannot reach an active person.
    /// - `IdentityError::RandomSourceUnavailable` if the new agent id cannot be generated.
    /// - `IdentityError::AlreadyRegistered` if the generated identity already exists.
    /// - `IdentityError::IdentityUnknown` if the responsible person or reporting identity is absent.
    /// - `IdentityError::ChangeMismatch` if a reporting edge or responsibility is invalid.
    pub fn register_reporting_agent(
        &mut self,
        actor: Actor,
        operation: OperationId,
        reports_to: IdentityId,
        profile: Profile,
        recorded_at: u64,
    ) -> Result<Registered, IdentityError> {
        self.settle()?;
        if let Some((event, receipt)) = self.answered(operation)? {
            let (responsible, target, shown) = match event.change() {
                Change::RegisterAgent {
                    responsible,
                    profile,
                } => (*responsible, IdentityId::Person(*responsible), profile),
                Change::ReportingRegistration {
                    responsible,
                    profile,
                    reports_to,
                } => (*responsible, *reports_to, profile),
                _ => return Err(reused(operation)),
            };
            if target != reports_to || shown != &profile || !same_actor(event.actor(), &actor) {
                return Err(reused(operation));
            }
            let agent = match event.identity() {
                IdentityId::Agent(agent) => agent,
                IdentityId::Person(_)
                | IdentityId::ServiceAccount(_)
                | IdentityId::Connector(_)
                | IdentityId::Machine(_) => {
                    return Err(reused(operation));
                }
            };
            return Ok(Registered {
                agent,
                reports_to,
                responsible,
                receipt,
            });
        }
        let responsible = self.projection.resolve_reporting(reports_to, None)?;
        match reports_to {
            IdentityId::Person(person) => {
                let (agent, receipt) =
                    self.register_agent(actor, operation, person, profile, recorded_at)?;
                return Ok(Registered {
                    agent,
                    reports_to,
                    responsible,
                    receipt,
                });
            }
            IdentityId::Agent(_) => {}
            IdentityId::ServiceAccount(_) | IdentityId::Connector(_) | IdentityId::Machine(_) => {
                return Err(IdentityError::ChangeMismatch {
                    reason: "a reporting target must be a person or agent",
                });
            }
        }
        let agent = AgentId::generate()?;
        let change = Change::ReportingRegistration {
            responsible,
            profile,
            reports_to,
        };
        let event = IdentityEvent::new(
            operation,
            actor,
            IdentityId::Agent(agent),
            recorded_at,
            change,
        )?;
        let receipt = self.commit(event)?;
        Ok(Registered {
            agent,
            reports_to,
            responsible,
            receipt,
        })
    }

    /// Change an edge and the affected subtree's responsibility in one signed leaf.
    ///
    /// # Errors
    /// - `IdentityError::LogUnavailable` if the directory is broken, the store cannot be reopened, or an adopted event is refused by the projection.
    /// - `IdentityError::LeafNotAnEvent` if reconciliation finds a missing or invalid signed leaf.
    /// - `IdentityError::OperationReused` if the operation already names a different request.
    /// - `IdentityError::InstallEntry` if a retried operation names an install leaf.
    /// - `IdentityError::DraftEntry` if a retried operation names a draft leaf.
    /// - `IdentityError::AppendRefused` if reconciliation proves this change was not committed.
    /// - `IdentityError::AnswersToUnknown` if a reporting target is absent or is not a person or agent.
    /// - `IdentityError::AnswersToInactive` if the immediate target is inactive.
    /// - `IdentityError::AnswersToCycle` if the new edge creates a cycle.
    /// - `IdentityError::NoAccountablePerson` if the chain cannot reach an active person.
    /// - `IdentityError::IdentityUnknown` if the moving agent or a reporting descendant is absent.
    /// - `IdentityError::ChangeMismatch` if the current edge or responsibility is absent or inconsistent.
    pub fn change_reports_to(
        &mut self,
        actor: Actor,
        operation: OperationId,
        agent: AgentId,
        target: IdentityId,
        recorded_at: u64,
    ) -> Result<Changed, IdentityError> {
        self.settle()?;
        let identity = IdentityId::Agent(agent);
        if let Some((event, receipt)) = self.answered(operation)? {
            let Change::ReportsToChanged {
                to,
                responsible_from,
                responsible_to,
                ..
            } = event.change()
            else {
                return Err(reused(operation));
            };
            if *to != target || event.identity() != identity || !same_actor(event.actor(), &actor) {
                return Err(reused(operation));
            }
            return Ok(Changed {
                reports_to: target,
                responsible_from: *responsible_from,
                responsible_to: *responsible_to,
                receipt,
            });
        }
        let record =
            self.projection
                .record(identity)
                .ok_or_else(|| IdentityError::IdentityUnknown {
                    identity: identity.to_string(),
                })?;
        let from = record.reports_to().ok_or(IdentityError::ChangeMismatch {
            reason: "the agent has no reporting edge",
        })?;
        let responsible_from = record.responsible().ok_or(IdentityError::ChangeMismatch {
            reason: "the agent has no accountable person",
        })?;
        let responsible_to = self.projection.resolve_reporting(target, Some(agent))?;
        let change = Change::ReportsToChanged {
            from,
            to: target,
            responsible_from,
            responsible_to,
        };
        let receipt = self.change(actor, operation, identity, change, recorded_at)?;
        Ok(Changed {
            reports_to: target,
            responsible_from,
            responsible_to,
            receipt,
        })
    }
}
