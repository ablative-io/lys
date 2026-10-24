//! The rights an agent holds on an agent it registers (AGENTS-015 R2,
//! AGENTS-016 R3): when an agent registers an agent under its grant of
//! `agent.create`, or under the administrator standing that covers it
//! (24 October 2026), the person answering for both gives it exactly
//! [`CREATED_RIGHTS`] on the new agent, each one relation carrying one
//! action, use-only, standing in the chain of the grant that admitted the
//! registration, so revoking it withdraws them.
//!
//! The agent exists before its rights do: they are recorded only on an agent
//! the directory already holds, answering to the same person, and refused by
//! name otherwise. The rights and the use of `agent.create` they record are
//! written in one durable act: one append, one flush, one pin. Asked again,
//! only the rights not already standing are written, so a finished
//! registration asked again appends nothing.

use lys_log_store::LeafStore;

use super::admission::Route;
use super::authority::{Grants, Recorded};
use super::error::GrantError;
use super::events::{GrantChange, GrantEvent, SignedGrantEvent, sign_grant_event};
use super::lineage::{CREATED_RIGHTS, admits_create, created_hop};
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
    /// How the registration arrived, recorded on the use of `source`.
    pub route: Route,
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

/// The use of `agent.create` a registration records with its rights.
const USE: &str = "use-of-agent.create";

impl<S: LeafStore, R: RelationshipStore> Grants<S, R> {
    /// Record the creator's [`CREATED_RIGHTS`] on `request.agent`, under its
    /// grant `request.source` of `agent.create`, with that grant's use and
    /// under its organisation (DIRECTORY-109 R4), in one append. Asked again for the same registration, the rights already
    /// recorded are answered and only the missing ones are written; when none
    /// is missing nothing is appended.
    ///
    /// # Errors
    /// - `GrantError::SourceUnknown` if the source grant is absent.
    /// - `GrantError::LineageMalformed` if it is not an agent's grant of
    ///   `agent.create` or of administrator standing, or the directory does
    ///   not hold the agent yet.
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
        let source = self.created_source(directory, request)?;
        let resource = Resource::new("agent", &request.agent.to_string())?;
        let responsible = source.responsible();
        let mut answered = Vec::new();
        let mut events = Vec::new();
        for action in CREATED_RIGHTS {
            let operation = derived(request.operation, action);
            if let Some((event, receipt)) = self.answered(operation)? {
                let standing = match event.change() {
                    GrantChange::Issue(grant) => {
                        grant.resource() == &resource && created_hop(grant, &source)
                    }
                    GrantChange::Revoke { .. }
                    | GrantChange::Use { .. }
                    | GrantChange::Stamp { .. } => false,
                };
                if !standing {
                    return Err(GrantError::OperationReused {
                        operation: operation.to_string(),
                    });
                }
                answered.push(self.answer(event, receipt)?);
                continue;
            }
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
            })?
            .with_organisation(source.organisation());
            let event = GrantEvent::new(
                operation,
                IdentityId::Person(responsible),
                at,
                GrantChange::Issue(Box::new(grant)),
            )?;
            self.book.check(&event)?;
            events.push(event);
        }
        if events.is_empty() {
            return Ok(answered);
        }
        let used = derived(request.operation, USE);
        if self.answered(used)?.is_none() {
            let event = GrantEvent::new(
                used,
                source.holder(),
                at,
                GrantChange::Use {
                    grant: source.id(),
                    route: request.route,
                },
            )?;
            self.book.check(&event)?;
            events.insert(0, event);
        }
        let mut recorded = self.commit_batch(events)?;
        recorded.retain(|one| matches!(one.event.change(), GrantChange::Issue(_)));
        answered.extend(recorded);
        Ok(answered)
    }

    /// The live grant `request` stands under, of `agent.create` or of the
    /// standing that covers it, once the directory holds its agent answering
    /// to that grant's person.
    fn created_source(
        &self,
        directory: &Projection,
        request: &CreatedRequest,
    ) -> Result<Grant, GrantError> {
        let source =
            self.book
                .grant(request.source)
                .cloned()
                .ok_or_else(|| GrantError::SourceUnknown {
                    grant: request.source.to_string(),
                })?;
        if !matches!(source.holder(), IdentityId::Agent(_)) || !admits_create(&source) {
            return Err(GrantError::LineageMalformed {
                reason: "an agent's rights on an agent it registered stand under its grant of agent.create or its administrator standing",
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
        match directory.record(IdentityId::Agent(request.agent)) {
            Some(record) if record.responsible() == Some(source.responsible()) => Ok(source),
            Some(_) => Err(GrantError::OperationReused {
                operation: request.operation.to_string(),
            }),
            None => Err(GrantError::LineageMalformed {
                reason: "an agent's rights are recorded only on an agent the directory already holds: the agent is registered first",
            }),
        }
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
