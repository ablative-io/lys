//! Several accounts under one secret. Each account is its own sealed entry,
//! named `<secret>@<account>`; the secret's ring says which is in use and
//! which are resting. A holder's handle names the secret, never an account,
//! so moving to the next account needs no new handle.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::{EntryClass, SecretStore, check_name};
use crate::error::SecretsError;
use crate::keys::StoreKey;
use crate::secret::Secret;

/// The order a secret's accounts are used in and which are resting.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(crate) struct Ring {
    order: Vec<String>,
    current: usize,
    resting: BTreeSet<String>,
}

/// One account of a secret, as a screen shows it. No secret byte.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AccountView {
    /// The account's name.
    pub account: String,
    /// Whether it is the one in use.
    pub current: bool,
    /// Whether it is resting.
    pub resting: bool,
}

fn entry_name(secret: &str, account: &str) -> String {
    format!("{secret}@{account}")
}

impl SecretStore {
    /// Seals `value` as another account of `secret`. The secret's own value
    /// stays its first account, named `primary`.
    ///
    /// # Errors
    ///
    /// `SecretUnknown`, `InvalidName`, `AccountExists` and the sealing
    /// refusals.
    pub fn add_account(
        &mut self,
        key: &StoreKey,
        secret: &str,
        account: &str,
        value: &Secret,
    ) -> Result<(), SecretsError> {
        check_name("account", account)?;
        let base = self
            .entry(secret)
            .cloned()
            .ok_or_else(|| SecretsError::SecretUnknown {
                name: secret.to_owned(),
            })?;
        let taken = account == "primary"
            || self
                .index
                .accounts
                .get(secret)
                .is_some_and(|ring| ring.order.iter().any(|held| held == account));
        if taken {
            return Err(SecretsError::AccountExists {
                secret: secret.to_owned(),
                account: account.to_owned(),
            });
        }
        self.add(
            key,
            &entry_name(secret, account),
            EntryClass::Credential,
            &base.owner,
            value,
        )?;
        self.index
            .accounts
            .entry(secret.to_owned())
            .or_insert_with(|| Ring {
                order: vec!["primary".to_owned()],
                current: 0,
                resting: BTreeSet::new(),
            })
            .order
            .push(account.to_owned());
        self.write_index()?;
        Ok(())
    }

    /// The accounts of `secret` in order.
    pub fn accounts(&self, secret: &str) -> Vec<AccountView> {
        match self.index.accounts.get(secret) {
            None => vec![AccountView {
                account: "primary".to_owned(),
                current: true,
                resting: false,
            }],
            Some(ring) => ring
                .order
                .iter()
                .enumerate()
                .map(|(at, account)| AccountView {
                    account: account.clone(),
                    current: at == ring.current,
                    resting: ring.resting.contains(account),
                })
                .collect(),
        }
    }

    /// The entry a use of `secret` opens: its current account's.
    pub(crate) fn current_entry(&self, secret: &str) -> Result<(String, String), SecretsError> {
        let Some(ring) = self.index.accounts.get(secret) else {
            return Ok((secret.to_owned(), "primary".to_owned()));
        };
        let account = ring
            .order
            .get(ring.current)
            .filter(|account| !ring.resting.contains(*account))
            .ok_or_else(|| SecretsError::NoAccountAvailable {
                secret: secret.to_owned(),
            })?;
        let entry = if account == "primary" {
            secret.to_owned()
        } else {
            entry_name(secret, account)
        };
        Ok((entry, account.clone()))
    }

    /// Rests the account in use and moves to the next one not resting.
    /// Answers the account now in use.
    ///
    /// # Errors
    ///
    /// `SecretUnknown`, and `NoAccountAvailable` when every account rests.
    pub(crate) fn next_account(&mut self, secret: &str) -> Result<String, SecretsError> {
        if self.entry(secret).is_none() {
            return Err(SecretsError::SecretUnknown {
                name: secret.to_owned(),
            });
        }
        let ring = self.index.accounts.get_mut(secret).ok_or_else(|| {
            SecretsError::NoAccountAvailable {
                secret: secret.to_owned(),
            }
        })?;
        let len = ring.order.len();
        let next = (1..len)
            .map(|step| (ring.current + step) % len)
            .find(|at| {
                ring.order
                    .get(*at)
                    .is_some_and(|account| !ring.resting.contains(account))
            })
            .ok_or_else(|| SecretsError::NoAccountAvailable {
                secret: secret.to_owned(),
            })?;
        if let Some(current) = ring.order.get(ring.current) {
            ring.resting.insert(current.clone());
        }
        ring.current = next;
        let account = ring.order.get(next).cloned().unwrap_or_default();
        self.write_index()?;
        Ok(account)
    }

    /// Returns a resting account to service.
    ///
    /// # Errors
    ///
    /// `AccountUnknown`.
    pub(crate) fn restore_account(
        &mut self,
        secret: &str,
        account: &str,
    ) -> Result<(), SecretsError> {
        let ring = self
            .index
            .accounts
            .get_mut(secret)
            .filter(|ring| ring.order.iter().any(|held| held == account))
            .ok_or_else(|| SecretsError::AccountUnknown {
                secret: secret.to_owned(),
                account: account.to_owned(),
            })?;
        ring.resting.remove(account);
        self.write_index()
    }
}
