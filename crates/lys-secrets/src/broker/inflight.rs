//! Calls in flight when their handle is dropped. A drop, and a revocation
//! of the access the handle was issued under, lands on every admitted call
//! of that handle: a call not yet forwarded is cancelled at the forward
//! boundary, its use and reservation released, and is answered
//! `HandleDropped`; a call already forwarded finishes and is recorded as
//! `completed_after_drop`.

use crate::error::SecretsError;
use crate::permission::{PermissionCheck, Relation};

use super::checked::Checked;
use super::{Broker, Ticket};

/// The outcome of a call cancelled before it was forwarded.
pub(super) const CANCELLED_AT_BOUNDARY: &str = "cancelled_at_boundary";

impl<P: PermissionCheck> Broker<P> {
    /// The forward boundary: answers the ticket back when its handle still
    /// stands, and cancels the call when the handle was dropped or its
    /// access revoked since admission. Call it with the ticket immediately
    /// before sending anything upstream.
    ///
    /// # Errors
    ///
    /// `HandleDropped` for a cancelled call, and the audit log's refusals.
    pub fn at_forward_boundary(&mut self, ticket: Ticket) -> Result<Ticket, SecretsError> {
        self.boundary(ticket, None)
    }

    /// The forward boundary as [`Broker::at_forward_boundary`], taking the
    /// permission source's answers from `checked`, asked again after
    /// admission and before the broker was taken, where it holds them.
    ///
    /// # Errors
    ///
    /// As [`Broker::at_forward_boundary`].
    pub fn at_forward_boundary_checked(
        &mut self,
        ticket: Ticket,
        checked: &Checked,
    ) -> Result<Ticket, SecretsError> {
        self.boundary(ticket, Some(checked))
    }

    fn boundary(
        &mut self,
        ticket: Ticket,
        checked: Option<&Checked>,
    ) -> Result<Ticket, SecretsError> {
        let subject = (ticket.identity.as_str(), ticket.secret.as_str());
        if !self.cut_off(&ticket.handle, subject, checked) {
            return Ok(ticket);
        }
        let Ticket {
            handle,
            identity,
            secret,
            operation,
            mark,
            reserved,
            ..
        } = ticket;
        self.close(
            &handle,
            (&identity, &secret),
            (&operation, &mark),
            reserved.map(|_reserved| 0),
            CANCELLED_AT_BOUNDARY,
        )?;
        Err(SecretsError::HandleDropped { handle })
    }

    /// Whether the handle was dropped, or the access it was issued under
    /// revoked.
    pub(super) fn cut_off(
        &self,
        handle: &str,
        (identity, secret): (&str, &str),
        checked: Option<&Checked>,
    ) -> bool {
        self.line_dropped(handle)
            || self.within_scope_as(identity, secret, checked).is_err()
            || checked
                .and_then(|checked| checked.get(Relation::Use, identity, secret))
                .unwrap_or_else(|| self.permissions.may_use(identity, secret))
                .is_err()
    }
}
