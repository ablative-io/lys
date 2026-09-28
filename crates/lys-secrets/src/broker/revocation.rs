//! Revoking a handle has two parts that are never merged: use stops here
//! when the handle is dropped, and the provider is asked to revoke the
//! grant behind it. The provider's part is confirmed only by its own
//! answer for that grant; no timer and no second revocation confirms it.
//!
//! A lease record holds its provider state in an [`Upstream`], whose one
//! field is private to this file, so no code outside it writes the state.
//! A revoke or a relinquish moves it to unconfirmed, read as `pending`,
//! through the ending's `end_with_upstream_pending`, and only
//! [`Broker::deliver_upstream_ack`] moves it to confirmed.

use crate::audit::{AuditKind, AuditLine};
use crate::error::{RevocationRefusal, SecretsError};
use crate::handle::HandleId;
use crate::permission::PermissionCheck;

use super::Broker;
use super::Handles;

const CONFIRMED: &str = "revoked_upstream";
const UNCONFIRMED: &str = "revocation_unconfirmed";
/// The reason an ended lease's provider state is unconfirmed until the
/// system behind acknowledges it.
const ASKED: &str = "the system behind is asked and has not confirmed";

/// Where the provider's part of a revocation stands.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum UpstreamRevocation {
    /// The provider was never asked.
    #[default]
    NotAsked,
    /// The provider was asked and has not confirmed, with the reason.
    Unconfirmed(String),
    /// The provider confirmed.
    Confirmed,
}

/// A lease's provider state as its record holds it: one of the
/// [`UpstreamRevocation`] states and no second one. Its field is private to
/// this file, and outside the crate the record's field is not reachable at
/// all:
///
/// ```compile_fail,E0616
/// fn write(lease: &mut lys_secrets::Lease) {
///     lease.upstream = Default::default();
/// }
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Upstream(UpstreamRevocation);

impl Upstream {
    /// The state it holds.
    pub(crate) fn state(&self) -> &UpstreamRevocation {
        &self.0
    }

    /// The state a snapshot carries, laid back over its record.
    pub(super) fn restored(state: UpstreamRevocation) -> Self {
        Self(state)
    }
}

/// The state after `line`: a provider revocation line sets its handle's
/// provider state to what the line says.
pub(super) fn fold(handles: &mut Handles, line: &AuditLine) {
    let (Some(id), Some(upstream)) = (&line.handle, upstream_of(line)) else {
        return;
    };
    if let Some(record) = handles.get_mut(id) {
        record.upstream = Upstream(upstream);
    }
}

/// What a provider revocation line says, when `line` is one for a handle.
pub(super) fn upstream_of(line: &AuditLine) -> Option<UpstreamRevocation> {
    if line.kind != AuditKind::Refresh {
        return None;
    }
    if line.outcome == CONFIRMED {
        return Some(UpstreamRevocation::Confirmed);
    }
    line.outcome
        .strip_prefix(UNCONFIRMED)
        .map(|reason| UpstreamRevocation::Unconfirmed(reason.trim_start_matches(": ").to_owned()))
}

/// Both parts of a handle's revocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevocationState {
    /// Whether use has stopped here: the handle, or one above it, dropped.
    pub stopped_here: bool,
    /// The provider's part.
    pub upstream: UpstreamRevocation,
}

impl<P: PermissionCheck> Broker<P> {
    /// Both parts of the revocation of the handle `id`, as the log's fold
    /// holds them.
    ///
    /// # Errors
    ///
    /// `HandleUnknown`.
    pub fn revocation_state(&self, id: &HandleId) -> Result<RevocationState, SecretsError> {
        let record = self
            .handles
            .get(id.as_str())
            .ok_or(SecretsError::HandleUnknown)?;
        Ok(RevocationState {
            stopped_here: self.line_dropped(id.as_str()),
            upstream: record.upstream.state().clone(),
        })
    }

    /// Both parts of the revocation of the handle `id`, as `identity` may
    /// discover them: only when the access seam lets it discover the handle,
    /// as its holder or the person it is acted for. Any other answers as a
    /// handle never issued; seeing or owning its secret discovers nothing.
    ///
    /// # Errors
    ///
    /// `HandleUnknown`, and the audit log's refusals.
    pub fn revocation_state_as(
        &self,
        identity: &str,
        id: &HandleId,
    ) -> Result<RevocationState, SecretsError> {
        if !self.discovers_lease(identity, id) {
            return Err(SecretsError::HandleUnknown);
        }
        self.revocation_state(id)
    }

    /// Records what the provider answered to revoking the grant behind the
    /// dropped handle `id`: confirmed, or unconfirmed with the reason.
    ///
    /// # Errors
    ///
    /// `HandleUnknown`, `RevocationBeforeDrop` while the handle still
    /// admits uses here, and the audit log's refusals.
    pub fn record_upstream_revocation(
        &mut self,
        id: &HandleId,
        answer: Result<(), String>,
    ) -> Result<(), SecretsError> {
        match answer {
            Ok(()) => self.deliver_upstream_ack(id).map(|_changed| ()),
            Err(reason) => self.upstream_line(id, &format!("{UNCONFIRMED}: {reason}")),
        }
    }

    /// Confirms an unconfirmed provider revocation from the provider's own
    /// later answer, which must name the grant's provider subject. Answers
    /// whether the state changed; an already confirmed one stays as it is.
    ///
    /// # Errors
    ///
    /// `HandleUnknown`, `RevocationNotPending` when the provider was never
    /// asked, `ProviderMismatch` when the answer names another subject, and
    /// the store's and audit log's refusals.
    pub fn confirm_upstream_revocation(
        &mut self,
        id: &HandleId,
        answered_subject: &str,
    ) -> Result<bool, SecretsError> {
        match self.revocation_state(id)?.upstream {
            UpstreamRevocation::Confirmed => return Ok(false),
            UpstreamRevocation::NotAsked => {
                return Err(RevocationRefusal::NotPending {
                    handle: id.as_str().to_owned(),
                }
                .into());
            }
            UpstreamRevocation::Unconfirmed(_reason) => {}
        }
        let grant = self
            .oauth_grant_of(id)?
            .ok_or_else(|| RevocationRefusal::NotPending {
                handle: id.as_str().to_owned(),
            })?;
        if grant.provenance().provider_subject != answered_subject {
            return Err(RevocationRefusal::ProviderMismatch {
                handle: id.as_str().to_owned(),
                answered: answered_subject.to_owned(),
            }
            .into());
        }
        self.deliver_upstream_ack(id)
    }

    /// Delivers the system behind's acknowledgement that the credential
    /// behind the lease `id` is revoked: its provider state moves to
    /// confirmed, with one audit line for the move. This is the one writer
    /// of confirmed, so no timer and no other operation moves `pending` to
    /// `confirmed`. Answers whether the state changed; one already
    /// confirmed stays as it is and nothing is appended.
    ///
    /// # Errors
    ///
    /// `HandleUnknown`, `RevocationBeforeDrop` while the lease still admits
    /// uses here, and the audit log's refusals.
    pub fn deliver_upstream_ack(&mut self, id: &HandleId) -> Result<bool, SecretsError> {
        if self.revocation_state(id)?.upstream == UpstreamRevocation::Confirmed {
            return Ok(false);
        }
        self.upstream_line(id, CONFIRMED)?;
        Ok(true)
    }

    /// Moves the provider state of the ended lease `id` to unconfirmed, read
    /// as `pending`, with one audit line for the move. Only the ending's
    /// `end_with_upstream_pending` calls it.
    pub(super) fn upstream_pending(&mut self, id: &HandleId) -> Result<(), SecretsError> {
        self.upstream_line(id, &format!("{UNCONFIRMED}: {ASKED}"))
    }

    fn upstream_line(&mut self, id: &HandleId, outcome: &str) -> Result<(), SecretsError> {
        let record = self
            .handles
            .get(id.as_str())
            .ok_or(SecretsError::HandleUnknown)?;
        if !self.line_dropped(id.as_str()) {
            return Err(RevocationRefusal::BeforeDrop {
                handle: id.as_str().to_owned(),
            }
            .into());
        }
        let (identity, secret) = (record.identity.clone(), record.secret.clone());
        let line = self.line(
            AuditKind::Refresh,
            (Some(id.as_str()), Some(&identity), Some(&secret)),
            None,
            None,
            outcome,
        );
        self.append(&line)?;
        fold(&mut self.handles, &line);
        Ok(())
    }
}
