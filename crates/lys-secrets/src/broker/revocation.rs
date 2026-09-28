//! Revoking a handle has two parts that are never merged: use stops here
//! when the handle is dropped, and the provider is asked to revoke the
//! grant behind it. The provider's part is confirmed only by its own
//! answer for that grant; no timer and no second revocation confirms it.

use crate::audit::{AuditKind, AuditLine};
use crate::error::{RevocationRefusal, SecretsError};
use crate::handle::HandleId;
use crate::permission::PermissionCheck;

use super::Broker;

const CONFIRMED: &str = "revoked_upstream";
const UNCONFIRMED: &str = "revocation_unconfirmed";

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
            upstream: record.upstream.clone(),
        })
    }

    /// Both parts of the revocation of the handle `id`, as `identity` may
    /// discover them: only when it may discover the handle's secret. A
    /// handle on a secret it may not discover answers as one never issued.
    ///
    /// # Errors
    ///
    /// `HandleUnknown`, and the audit log's refusals.
    pub fn revocation_state_as(
        &self,
        identity: &str,
        id: &HandleId,
    ) -> Result<RevocationState, SecretsError> {
        let record = self
            .handles
            .get(id.as_str())
            .ok_or(SecretsError::HandleUnknown)?;
        if !self.discovers(identity, &record.secret) {
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
        let outcome = match answer {
            Ok(()) => CONFIRMED.to_owned(),
            Err(reason) => format!("{UNCONFIRMED}: {reason}"),
        };
        self.upstream_line(id, &outcome)
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
        self.upstream_line(id, CONFIRMED)?;
        Ok(true)
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
        if let (Some(record), Some(upstream)) =
            (self.handles.get_mut(id.as_str()), upstream_of(&line))
        {
            record.upstream = upstream;
        }
        Ok(())
    }
}
