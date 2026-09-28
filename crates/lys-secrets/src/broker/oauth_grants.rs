//! OAuth service grants at the broker. A grant is sealed like any secret
//! and used through a handle; the proxy refreshes its access token from the
//! refresh token when it expires, and the broker reseals the refreshed
//! grant and records the refresh. A drop may also revoke the grant upstream;
//! the log says whether the provider confirmed it.

use crate::audit::AuditKind;
use crate::error::SecretsError;
use crate::handle::HandleId;
use crate::oauth::OAuthGrant;
use crate::permission::PermissionCheck;
use crate::store::EntryClass;

use super::{Broker, Ticket};

impl Ticket {
    /// The OAuth grant this ticket opened, when its secret is one.
    ///
    /// # Errors
    ///
    /// `Encoding` when the sealed grant does not read.
    pub fn oauth(&self) -> Result<Option<OAuthGrant>, SecretsError> {
        if self.class != EntryClass::OAuth {
            return Ok(None);
        }
        OAuthGrant::from_sealed(&self.credential).map(Some)
    }
}

impl<P: PermissionCheck> Broker<P> {
    /// Seals an OAuth service grant as the secret `name`, owned by `owner`.
    ///
    /// # Errors
    ///
    /// Every refusal of [`crate::SecretStore::add`], and the audit log's.
    pub fn seal_oauth(
        &mut self,
        name: &str,
        owner: &str,
        grant: &OAuthGrant,
    ) -> Result<(), SecretsError> {
        self.store.add(
            &self.store_key,
            name,
            EntryClass::OAuth,
            owner,
            &grant.to_sealed()?,
        )?;
        self.record(
            AuditKind::Seal,
            (None, Some(owner), Some(name)),
            None,
            None,
            "sealed",
        )?;
        Ok(())
    }

    /// Reseals `grant`, refreshed for the call `ticket` admits, in place of
    /// the ticket's entry, and records the refresh. The anchor moves at the
    /// call's settlement, the line that follows.
    ///
    /// # Errors
    ///
    /// The store's and the audit log's refusals.
    pub fn refreshed(&mut self, ticket: &Ticket, grant: &OAuthGrant) -> Result<(), SecretsError> {
        self.store
            .replace(&self.store_key, &ticket.entry, &grant.to_sealed()?)?;
        let subject = (
            Some(ticket.handle.as_str()),
            Some(ticket.identity.as_str()),
            Some(ticket.secret.as_str()),
        );
        let call = Some((ticket.operation.as_str(), ticket.mark.as_str()));
        let line = self.line(AuditKind::Refresh, subject, call, None, "refreshed");
        self.append_unanchored(&line)?;
        Ok(())
    }

    /// The OAuth grant behind the handle `id`, for revoking it upstream
    /// after a drop. `None` when the handle's secret is not one.
    ///
    /// # Errors
    ///
    /// `HandleUnknown`, and the store's refusals.
    pub fn oauth_grant_of(&self, id: &HandleId) -> Result<Option<OAuthGrant>, SecretsError> {
        let record = self
            .handles
            .get(id.as_str())
            .ok_or(SecretsError::HandleUnknown)?;
        let (entry, _account) = self.store.current_entry(&record.secret)?;
        if self.store.entry(&entry).map(|view| view.class) != Some(EntryClass::OAuth) {
            return Ok(None);
        }
        let sealed = self
            .store
            .open_for_use(&self.store_key, &entry, EntryClass::OAuth)?;
        OAuthGrant::from_sealed(&sealed).map(Some)
    }
}
