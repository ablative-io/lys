//! The scope boundary: a secret is discoverable, readable, usable and
//! lendable only by the identities inside its scope, whatever grants exist.
//! Knowing a secret's name grants nothing: outside its scope a secret
//! answers exactly as one that is not there.

use crate::audit::AuditKind;
use crate::error::SecretsError;
use crate::permission::PermissionCheck;
use crate::store::{EntryView, Scope};

use super::Broker;

impl<P: PermissionCheck> Broker<P> {
    /// Whether `identity` stands inside the scope of `secret`: its owner,
    /// the person a personal secret belongs to, or an identity the
    /// permission source makes a member of the scope. The refusal says why.
    pub(super) fn within_scope(&self, identity: &str, secret: &str) -> Result<(), String> {
        let (Some(entry), Some(scope)) = (self.store.entry(secret), self.store.scope(secret))
        else {
            return Err("no such secret".to_owned());
        };
        if entry.owner == identity {
            return Ok(());
        }
        if let Scope::Personal(person) = &scope {
            if person == identity {
                return Ok(());
            }
        }
        self.permissions
            .member_of(identity, &scope.target())
            .map(|_permit| ())
            .map_err(|denied| format!("outside the secret's scope: {}", denied.reason))
    }

    /// The same refusal an unsealed name gets, when `identity` is outside
    /// the scope of `secret`.
    pub(super) fn discoverable(&self, identity: &str, secret: &str) -> Result<(), SecretsError> {
        self.within_scope(identity, secret)
            .map_err(|_reason| SecretsError::SecretUnknown {
                name: secret.to_owned(),
            })
    }

    /// The secrets `identity` may discover, without their values.
    pub fn listing(&self, identity: &str) -> Vec<EntryView> {
        self.store
            .entries()
            .filter(|entry| self.within_scope(identity, &entry.name).is_ok())
            .cloned()
            .collect()
    }

    /// One secret's description, as `identity` may discover it.
    ///
    /// # Errors
    ///
    /// `SecretUnknown` when no such secret is sealed or `identity` is
    /// outside its scope; the two are not told apart.
    pub fn metadata(&self, identity: &str, secret: &str) -> Result<EntryView, SecretsError> {
        self.discoverable(identity, secret)?;
        self.store
            .entry(secret)
            .cloned()
            .ok_or_else(|| SecretsError::SecretUnknown {
                name: secret.to_owned(),
            })
    }

    /// Sets the scope of `secret`, as its owner.
    ///
    /// # Errors
    ///
    /// `LendingNotPermitted` when `owner` does not own it, the store's
    /// refusals, and the audit log's.
    pub fn set_scope(
        &mut self,
        owner: &str,
        secret: &str,
        scope: Scope,
    ) -> Result<(), SecretsError> {
        let owns = self
            .store
            .entry(secret)
            .is_some_and(|entry| entry.owner == owner);
        if !owns {
            return Err(SecretsError::LendingNotPermitted {
                holder: owner.to_owned(),
                secret: secret.to_owned(),
            });
        }
        let outcome = format!("scope {}", scope.target());
        self.store.set_scope(secret, scope)?;
        self.record(
            AuditKind::Seal,
            (None, Some(owner), Some(secret)),
            None,
            None,
            &outcome,
        )?;
        Ok(())
    }
}
