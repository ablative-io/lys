//! OAuth service grants at the broker. A grant is sealed like any secret
//! and used through a handle; the proxy refreshes its access token from the
//! refresh token when it expires, and the broker reseals the refreshed
//! grant and records the refresh. A refresh keeps the client the grant was
//! consented to: a grant for a new client is a new consent, reconnected by
//! name, and a refresh that names one is refused. A drop may also revoke the
//! grant upstream; the log says whether the provider confirmed it.

use crate::audit::AuditKind;
use crate::error::{OAuthRefusal, SecretsError};
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
    /// the ticket's entry, and records the refresh. A refresh keeps the
    /// client the grant was consented to: one naming another client is
    /// refused, recorded, and reseals nothing, until the grant is
    /// reconnected by name with [`Broker::reconnect_oauth`].
    ///
    /// # Errors
    ///
    /// `ReconnectRequired` for a grant naming another client, the store's
    /// refusals and the audit log's.
    pub fn refreshed(&mut self, ticket: &Ticket, grant: &OAuthGrant) -> Result<(), SecretsError> {
        let subject = (
            Some(ticket.handle.as_str()),
            Some(ticket.identity.as_str()),
            Some(ticket.secret.as_str()),
        );
        let call = Some((ticket.operation.as_str(), ticket.mark.as_str()));
        let consented = ticket
            .oauth()?
            .map(|held| held.provenance().client_id.clone());
        if let Some(consented) = consented.filter(|client| *client != grant.provenance().client_id)
        {
            let refusal = SecretsError::from(OAuthRefusal::ReconnectRequired {
                secret: ticket.secret.clone(),
                consented,
                presented: grant.provenance().client_id.clone(),
            });
            self.record(AuditKind::Refresh, subject, call, None, refusal.name())?;
            return Err(refusal);
        }
        self.store
            .replace(&self.store_key, &ticket.entry, &grant.to_sealed()?)?;
        self.record(AuditKind::Refresh, subject, call, None, "refreshed")?;
        Ok(())
    }

    /// Reconnects the OAuth grant sealed as `name` with `grant`, a new
    /// consent by its owner, for example to a new client: from here the
    /// grant's client, subject and tokens are the new consent's. Recorded as
    /// one refresh line naming the client.
    ///
    /// # Errors
    ///
    /// `LendingNotPermitted` when `owner` does not own it, `SecretUnknown`
    /// when no OAuth grant is sealed as `name`, the store's refusals and
    /// the audit log's.
    pub fn reconnect_oauth(
        &mut self,
        name: &str,
        owner: &str,
        grant: &OAuthGrant,
    ) -> Result<(), SecretsError> {
        self.owns(owner, name)?;
        if self.store.entry(name).map(|entry| entry.class) != Some(EntryClass::OAuth) {
            return Err(SecretsError::SecretUnknown {
                name: name.to_owned(),
            });
        }
        self.store
            .replace(&self.store_key, name, &grant.to_sealed()?)?;
        let outcome = format!("reconnected to client {}", grant.provenance().client_id);
        self.record(
            AuditKind::Refresh,
            (None, Some(owner), Some(name)),
            None,
            None,
            &outcome,
        )?;
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
