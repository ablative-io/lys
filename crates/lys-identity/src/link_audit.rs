//! The link-audit receiver's core (R4).
//!
//! The source reports each link or unlink it observed under its own operation
//! id. The receiver records it once: a delivery of a source operation id it
//! already accepted is answered with the first receipt and records nothing, so a
//! lost acknowledgement or a redelivery never makes a second event. The first
//! receipt answers only the delivery it was made for: the same source operation
//! id with another person or another observation is refused as `LinkSourceSeen`,
//! so one operation's acknowledgement is never handed to another. What is
//! recorded is the issuer's observation, kept apart from any claim a person
//! made, and the actor is the authenticated source as the service attests it,
//! so the actor's provenance survives replay with the event.

use lys_log_store::LeafStore;

use crate::directory::Directory;
use crate::error::IdentityError;
use crate::event::{Change, LinkObservation};
use crate::id::{IdentityId, PersonId};
use crate::operation::OperationId;
use crate::provenance::Actor;
use crate::receipt::Receipt;

impl<S: LeafStore> Directory<S> {
    /// Accept one observation about `person` from the authenticated `source`.
    pub fn accept_link_audit(
        &mut self,
        source: Actor,
        person: PersonId,
        observation: LinkObservation,
        recorded_at: u64,
    ) -> Result<Receipt, IdentityError> {
        self.settle()?;
        if let Some(index) = self
            .projection()?
            .link_source(observation.source_operation_id())
        {
            let (first, coordinate) =
                self.committed_at(index)?
                    .ok_or(IdentityError::ReceiptInvalid {
                        reason: "an accepted source operation has no receipt",
                    })?;
            let accepted = first.event()?;
            if accepted.identity() != IdentityId::Person(person)
                || accepted.change() != &Change::LinkAudit(observation.clone())
            {
                return Err(IdentityError::LinkSourceSeen {
                    source_operation_id: observation.source_operation_id().to_owned(),
                });
            }
            return Receipt::of(&first, coordinate);
        }
        self.commit_change(
            source,
            OperationId::generate()?,
            IdentityId::Person(person),
            Change::LinkAudit(observation),
            recorded_at,
        )
    }
}
