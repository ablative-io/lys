//! The rights an agent holds on an agent it registers (AGENTS-015 R2): when
//! an agent registers an agent under its grant of `agent.create`, the person
//! answering for both gives it exactly [`CREATED_RIGHTS`] on the new agent,
//! each one relation carrying one action, use-only, standing in the chain of
//! that `agent.create` grant, so revoking it withdraws them. All five are
//! written in one durable act: one append, one flush, one pin.

use lys_log_store::LeafStore;

use super::authority::{Grants, Recorded};
use super::error::GrantError;
use super::events::{GrantChange, GrantEvent, SignedGrantEvent, sign_grant_event};
use super::lineage::{CREATE, CREATED_RIGHTS, created_hop};
use super::permission::RelationshipStore;
use super::receipt::GrantReceipt;
use super::shipped::ONE;
use super::types::{Grant, GrantId, GrantParts, PassOn, Relation, Resource, Source, Window};
use crate::id::{AgentId, IdentityId};
use crate::operation::OperationId;
use crate::projection::Projection;

/// A registration's ask for its creator's rights on the agent it registered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreatedRequest {
    /// The registration's operation: each right's operation is derived from it.
    pub operation: OperationId,
    /// The `agent.create` grant the registration was admitted under.
    pub source: GrantId,
    /// The agent registered.
    pub agent: AgentId,
}

/// The operation of the right to `action` recorded for the registration
/// `operation`: the same every time it is asked, so a retry is answered.
fn derived(operation: OperationId, action: &str) -> OperationId {
    use sha2::{Digest, Sha256};
    let hash = Sha256::digest(format!("lys/agent-created/v1/{operation}/{action}"));
    let mut id = [0; 16];
    id.copy_from_slice(&hash[..16]);
    OperationId::from_bytes(id)
}

impl<S: LeafStore, R: RelationshipStore> Grants<S, R> {
    /// Record the creator's [`CREATED_RIGHTS`] on `request.agent`, under its
    /// grant `request.source` of `agent.create`, in one append. Asked again for
    /// the same registration, the rights already recorded are answered.
    ///
    /// # Errors
    /// - `GrantError::SourceUnknown` if the source grant is absent.
    /// - `GrantError::LineageMalformed` if it is not an agent's grant of `agent.create`.
    /// - `GrantError::Revoked` if it was revoked.
    /// - `GrantError::OperationReused` if the agent answers to another person, or the operations name other changes.
    /// - `GrantError::LogUnavailable` or `GrantError::AppendRefused` if the batch is not recorded.
    pub fn record_created(
        &mut self,
        directory: &Projection,
        request: &CreatedRequest,
        at: u64,
    ) -> Result<Vec<Recorded>, GrantError> {
        self.settle_for_change()?;
        let operations: Vec<OperationId> = CREATED_RIGHTS
            .iter()
            .map(|action| derived(request.operation, action))
            .collect();
        let mut answered = Vec::new();
        for operation in &operations {
            if let Some(found) = self.answered(*operation)? {
                answered.push(found);
            }
        }
        if !answered.is_empty() {
            return self.answered_created(request, answered, operations.len());
        }
        let source =
            self.book
                .grant(request.source)
                .cloned()
                .ok_or_else(|| GrantError::SourceUnknown {
                    grant: request.source.to_string(),
                })?;
        if !matches!(source.holder(), IdentityId::Agent(_))
            || !source
                .actions()
                .iter()
                .any(|action| action.as_str() == CREATE)
        {
            return Err(GrantError::LineageMalformed {
                reason: "an agent's rights on an agent it registered stand under its grant of agent.create",
            });
        }
        if self
            .book
            .record(source.id())
            .is_some_and(|record| record.revoked().is_some())
        {
            return Err(GrantError::Revoked {
                grant: source.id().to_string(),
            });
        }
        let responsible = source.responsible();
        if directory
            .record(IdentityId::Agent(request.agent))
            .is_some_and(|record| record.responsible() != Some(responsible))
        {
            return Err(GrantError::OperationReused {
                operation: request.operation.to_string(),
            });
        }
        let resource = Resource::new("agent", &request.agent.to_string())?;
        let mut events = Vec::with_capacity(CREATED_RIGHTS.len());
        for (action, operation) in CREATED_RIGHTS.iter().zip(operations) {
            let relation = Relation::new(&format!("{ONE}{action}"))?;
            let actions = self.model.actions_on("agent", &relation)?.clone();
            let grant = Grant::new(GrantParts {
                id: GrantId::generate()?,
                issuer: IdentityId::Person(responsible),
                holder: source.holder(),
                responsible,
                resource: resource.clone(),
                relation,
                actions,
                pass_on: PassOn::UseOnly,
                source: Source::Grant(source.id()),
                window: Window::new(at, source.window().ends_at())?,
                model_version: self.model.version_on("agent"),
                operation,
            })?;
            let event = GrantEvent::new(
                operation,
                IdentityId::Person(responsible),
                at,
                GrantChange::Issue(Box::new(grant)),
            )?;
            self.book.check(&event)?;
            events.push(event);
        }
        self.commit_batch(events)
    }

    /// The rights a registration already recorded, each answered; refused
    /// when they are not the whole batch for this agent and creator.
    fn answered_created(
        &mut self,
        request: &CreatedRequest,
        answered: Vec<(GrantEvent, GrantReceipt)>,
        expected: usize,
    ) -> Result<Vec<Recorded>, GrantError> {
        let resource = Resource::new("agent", &request.agent.to_string())?;
        let whole = answered.len() == expected
            && answered.iter().all(|(event, _)| match event.change() {
                GrantChange::Issue(grant) => {
                    grant.resource() == &resource
                        && self
                            .book
                            .grant(request.source)
                            .is_some_and(|source| created_hop(grant, source))
                }
                GrantChange::Revoke { .. } | GrantChange::Use { .. } => false,
            });
        if !whole {
            return Err(GrantError::OperationReused {
                operation: request.operation.to_string(),
            });
        }
        answered
            .into_iter()
            .map(|(event, receipt)| self.answer(event, receipt))
            .collect()
    }

    /// Commit `events` in one append: each checked, signed and folded, and
    /// answered once projected. A batch whose append is held uncertain is
    /// answered from the log once reconciled, or refused when it is not there.
    pub(super) fn commit_batch(
        &mut self,
        events: Vec<GrantEvent>,
    ) -> Result<Vec<Recorded>, GrantError> {
        for event in &events {
            if let GrantChange::Issue(grant) = event.change() {
                self.relationships.admit_resource(grant.resource())?;
            }
        }
        let signed = events
            .into_iter()
            .map(|event| sign_grant_event(event, &self.key))
            .collect::<Result<Vec<SignedGrantEvent>, GrantError>>()?;
        match self.ledger.append_batch(&signed) {
            Ok(coordinates) => {
                for (event, coordinate) in signed.iter().zip(&coordinates) {
                    self.record_committed(event, *coordinate)?;
                }
                self.snapshot();
                signed
                    .iter()
                    .zip(coordinates)
                    .map(|(event, coordinate)| {
                        self.answer(event.event().clone(), GrantReceipt::of(event, coordinate))
                    })
                    .collect()
            }
            Err(failure) => {
                if self.ledger.uncertain().is_none() {
                    return Err(GrantError::AppendRefused {
                        reason: failure.to_string(),
                    });
                }
                self.settle_for_change()?;
                let mut recorded = Vec::with_capacity(signed.len());
                for event in &signed {
                    match self.answered(event.event().operation())? {
                        Some((kept, receipt)) => recorded.push(self.answer(kept, receipt)?),
                        None => {
                            return Err(GrantError::AppendRefused {
                                reason: failure.to_string(),
                            });
                        }
                    }
                }
                Ok(recorded)
            }
        }
    }
}
