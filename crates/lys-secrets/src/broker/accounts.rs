//! Accounts under one secret, moved between without a new handle, each move
//! one audit line naming accounts and never their values.

use crate::audit::AuditKind;
use crate::error::SecretsError;
use crate::permission::PermissionCheck;
use crate::secret::Secret;

use super::Broker;

impl<P: PermissionCheck> Broker<P> {
    /// Seals another account of `secret`.
    ///
    /// # Errors
    ///
    /// Every refusal of [`crate::SecretStore::add_account`].
    pub fn add_account(
        &mut self,
        secret: &str,
        account: &str,
        value: &Secret,
    ) -> Result<(), SecretsError> {
        self.store
            .add_account(&self.store_key, secret, account, value)?;
        let named = format!("{secret}@{account}");
        self.record(
            AuditKind::Seal,
            (None, None, Some(&named)),
            None,
            None,
            "sealed",
        )?;
        Ok(())
    }

    /// Rests the account in use, for example at its usage limit, and moves
    /// every handle on `secret` to the next account. Answers that account.
    ///
    /// # Errors
    ///
    /// `SecretUnknown` and `NoAccountAvailable`.
    pub fn next_account(&mut self, secret: &str) -> Result<String, SecretsError> {
        match self.store.next_account(secret) {
            Ok(account) => {
                let outcome = format!("now {account}");
                self.record(
                    AuditKind::NextAccount,
                    (None, None, Some(secret)),
                    None,
                    None,
                    &outcome,
                )?;
                Ok(account)
            }
            Err(refusal) => {
                self.record(
                    AuditKind::NextAccount,
                    (None, None, Some(secret)),
                    None,
                    None,
                    refusal.name(),
                )?;
                Err(refusal)
            }
        }
    }

    /// Returns a resting account of `secret` to service.
    ///
    /// # Errors
    ///
    /// `AccountUnknown`.
    pub fn restore_account(&mut self, secret: &str, account: &str) -> Result<(), SecretsError> {
        self.store.restore_account(secret, account)?;
        let named = format!("{secret}@{account}");
        self.record(
            AuditKind::NextAccount,
            (None, None, Some(&named)),
            None,
            None,
            "restored",
        )?;
        Ok(())
    }
}
