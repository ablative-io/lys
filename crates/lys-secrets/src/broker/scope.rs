//! The scope boundary: a secret whose owner set a scope is discoverable,
//! readable, usable and lendable only by the identities inside it, whatever
//! grants exist. Knowing a secret's name grants nothing: outside its scope
//! a secret answers exactly as one that is not there, and a secret is
//! listed only to its owner and the identities it is granted to.

use crate::error::SecretsError;
use crate::handle::{HandleToken, Presentation};
use crate::permission::PermissionCheck;
use crate::store::{EntryView, Recipients, Scope};

use super::Broker;
use super::owner::{Admission, Call, OwnerChanged, SCOPE};

/// A secret's owner settings as they stand.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SecretSettings {
    /// The scope its owner set; none bounds no one.
    pub scope: Option<Scope>,
    /// Who it may be handed to.
    pub recipients: Recipients,
    /// The operation id of the last owner change applied, when it carried
    /// one.
    pub last_operation: Option<String>,
}

impl<P: PermissionCheck> Broker<P> {
    /// Whether `identity` stands inside the scope of `secret`: its owner;
    /// for a personal secret, its person; or an identity the permission
    /// source makes a member of the scope (for a person, one acting for
    /// them). A grant alone, from whoever, never crosses a scope. A secret with no scope set bounds no one here. The
    /// refusal says why.
    pub(super) fn within_scope(&self, identity: &str, secret: &str) -> Result<(), String> {
        let Some(entry) = self.store.entry(secret) else {
            return Err("no such secret".to_owned());
        };
        let Some(scope) = self.store.scope(secret) else {
            return Ok(());
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

    /// The identity a presented handle speaks for, when the handle is live
    /// and the presentation signed by its holder. Counts no use.
    ///
    /// # Errors
    ///
    /// The presentation's refusals, as a use would meet them.
    pub fn caller(
        &self,
        token: &HandleToken,
        presentation: &Presentation,
    ) -> Result<String, SecretsError> {
        let record = self.presented(token, presentation)?;
        self.live(record, presentation)?;
        Ok(record.identity.clone())
    }

    /// Whether `identity` may discover `secret`: inside its scope, and its
    /// owner or granted it. An account sealed under a secret is granted as
    /// that secret is.
    pub fn discovers(&self, identity: &str, secret: &str) -> bool {
        let granted_as = self.store.account_parent(secret).unwrap_or(secret);
        let granted = self
            .store
            .entry(secret)
            .is_some_and(|entry| entry.owner == identity)
            || self.permissions.may_use(identity, granted_as).is_ok()
            || self.permissions.may_read(identity, granted_as).is_ok()
            || self.permissions.may_lend(identity, granted_as).is_ok();
        granted && self.within_scope(identity, secret).is_ok()
    }

    /// The secrets `identity` may discover, without their values.
    pub fn listing(&self, identity: &str) -> Vec<EntryView> {
        self.store
            .entries()
            .filter(|entry| self.discovers(identity, &entry.name))
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
        self.store
            .entry(secret)
            .filter(|_entry| self.discovers(identity, secret))
            .cloned()
            .ok_or_else(|| SecretsError::SecretUnknown {
                name: secret.to_owned(),
            })
    }

    /// The owner settings of `secret` as they stand, to an identity that may
    /// discover it: the scope, when one is set, who it may be handed to, and
    /// the operation id of the last owner change applied.
    ///
    /// # Errors
    ///
    /// `SecretUnknown` when no such secret is sealed or `identity` may not
    /// discover it; the two are not told apart.
    pub fn settings(&self, identity: &str, secret: &str) -> Result<SecretSettings, SecretsError> {
        self.metadata(identity, secret)?;
        Ok(SecretSettings {
            scope: self.store.scope(secret),
            recipients: self.store.recipients(secret),
            last_operation: self.last_operation(secret),
        })
    }

    /// Sets the scope of `secret`, as its owner, from the owner's own
    /// command. The change carries no operation id.
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
        self.owns(owner, secret)?;
        self.apply_scope(owner, secret, scope, None, None)
    }

    /// Sets the scope of `secret`, as its owner, under the operation id
    /// `operation`, asked through the screen service `via` when one carried
    /// the owner's word; the audit line names both. An id already applied
    /// to the secret with this same change answers the outcome recorded
    /// then, and applies nothing.
    ///
    /// # Errors
    ///
    /// As `set_scope`, `OperationMissing` for no operation id or one of the
    /// wrong shape, and `OperationReused` for an id already applied to the
    /// secret with another change.
    pub fn set_scope_via(
        &mut self,
        owner: &str,
        secret: &str,
        scope: Scope,
        via: Option<&str>,
        operation: Option<&str>,
    ) -> Result<OwnerChanged, SecretsError> {
        self.owns(owner, secret)?;
        let change = format!("{SCOPE}{}", scope.target());
        match self.owner_admission(secret, operation, &change)? {
            Admission::Repeated(outcome) => Ok(OwnerChanged::Repeated { outcome }),
            Admission::Fresh(call) => {
                self.apply_scope(owner, secret, scope, via, Some(&call))?;
                Ok(OwnerChanged::Applied)
            }
        }
    }

    fn apply_scope(
        &mut self,
        owner: &str,
        secret: &str,
        scope: Scope,
        via: Option<&str>,
        call: Option<&Call>,
    ) -> Result<(), SecretsError> {
        let outcome = with_via(format!("{SCOPE}{}", scope.target()), via);
        self.store.set_scope(secret, scope)?;
        self.record_owner_change(owner, secret, &outcome, call)
    }
}

/// An audit outcome, naming the screen service that carried it when one did.
pub(super) fn with_via(outcome: String, via: Option<&str>) -> String {
    match via {
        Some(service) => format!("{outcome} via {service}"),
        None => outcome,
    }
}
