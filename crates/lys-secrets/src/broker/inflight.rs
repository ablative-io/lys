//! Calls in flight when their handle is dropped. A drop, and a revocation
//! of the access the handle was issued under, lands on every admitted call
//! of that handle: a call not yet forwarded is cancelled at the forward
//! boundary, its use and reservation released, and is answered
//! `HandleDropped`; a call already forwarded finishes and is recorded as
//! `completed_after_drop`.

use crate::error::SecretsError;
use crate::permission::PermissionCheck;

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
        if !self.cut_off(&ticket.handle, &ticket.identity, &ticket.secret) {
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
    pub(super) fn cut_off(&self, handle: &str, identity: &str, secret: &str) -> bool {
        self.line_dropped(handle)
            || self.within_scope(identity, secret).is_err()
            || self.permissions.may_use(identity, secret).is_err()
    }
}
