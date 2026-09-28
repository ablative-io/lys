//! A seat's own login at spawn. The engine that starts a seat asks for it
//! by the seat's identity; the login set is a secret's accounts, taken in
//! turn, so each spawn rotates and a running seat keeps its login. One
//! audit line records which account's login went to which seat. No engine
//! is named: any engine asks the same way, and one without the broker reads
//! its own pool as before.

use crate::audit::AuditKind;
use crate::error::SecretsError;
use crate::permission::PermissionCheck;
use crate::secret::Secret;
use crate::store::EntryClass;

use super::Broker;

impl<P: PermissionCheck> Broker<P> {
    /// The login for the seat `seat` from the login set `secret`, and the
    /// account it came from. The seat must hold the use relation on it.
    ///
    /// # Errors
    ///
    /// `PermissionDenied`, `not_a_value_secret` for a signing key, which
    /// hands no login out, `SecretUnknown`, `AccountsRested`, the store's
    /// opening refusals, and the audit log's.
    pub fn spawn_login(
        &mut self,
        seat: &str,
        secret: &str,
    ) -> Result<(String, Secret), SecretsError> {
        let taken = self
            .within_scope(seat, secret)
            .and_then(|()| {
                self.permissions
                    .may_use(seat, secret)
                    .map(|_permit| ())
                    .map_err(|denied| denied.reason)
            })
            .map_err(|reason| SecretsError::PermissionDenied {
                holder: seat.to_owned(),
                secret: secret.to_owned(),
                reason,
            })
            .and_then(|()| self.gives_value(secret, secret))
            .and_then(|()| self.store.take_turn(secret))
            .and_then(|(entry, account)| {
                self.store
                    .open_for_use(&self.store_key, &entry, EntryClass::Credential)
                    .map(|login| (account, login))
            });
        match taken {
            Ok((account, login)) => {
                let named = format!("{secret}@{account}");
                self.record(
                    AuditKind::SpawnLogin,
                    (None, Some(seat), Some(&named)),
                    None,
                    None,
                    "handed",
                )?;
                Ok((account, login))
            }
            Err(refusal) => {
                self.record(
                    AuditKind::SpawnLogin,
                    (None, Some(seat), Some(secret)),
                    None,
                    None,
                    refusal.name(),
                )?;
                Err(refusal)
            }
        }
    }
}
