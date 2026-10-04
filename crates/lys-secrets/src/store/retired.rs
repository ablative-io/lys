//! Retiring a secret. Its sealings, its accounts and its owner settings
//! leave the store in one index write, and its name is kept as retired, by
//! whom and when, so the name is never sealed again and every audit line
//! that names it keeps naming the one secret it was.

use serde::{Deserialize, Serialize};

use super::{EntryView, SecretStore};
use crate::error::SecretsError;

/// Who retired a secret's name, and when.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Retired {
    /// The identity that retired it.
    pub by: String,
    /// When, in milliseconds since the epoch.
    pub at_ms: i64,
}

impl SecretStore {
    /// Who retired `name`, and when, when it was retired.
    pub fn retired(&self, name: &str) -> Option<&Retired> {
        self.index.retired.get(name)
    }

    /// Retires `name`: its entry, each account sealed under it, its ring,
    /// its recipients and its scope leave the index, and the name is kept
    /// as retired by `by` at `at_ms`, all in one index write. The sealings
    /// the index no longer names are then swept. Answers the entry as it
    /// stood.
    ///
    /// # Errors
    ///
    /// `SecretRetired` when it was retired already, `SecretUnknown` when no
    /// such secret is sealed, and the store's write refusals. A write that
    /// fails leaves the store as it was.
    pub(crate) fn retire(
        &mut self,
        name: &str,
        by: &str,
        at_ms: i64,
    ) -> Result<EntryView, SecretsError> {
        if self.index.retired.contains_key(name) {
            return Err(SecretsError::SecretRetired {
                name: name.to_owned(),
            });
        }
        let entry =
            self.index
                .entries
                .get(name)
                .cloned()
                .ok_or_else(|| SecretsError::SecretUnknown {
                    name: name.to_owned(),
                })?;
        let accounts: Vec<String> = self
            .accounts(name)
            .into_iter()
            .map(|view| format!("{name}@{}", view.account))
            .collect();
        let before = (
            self.index.entries.clone(),
            self.index.accounts.remove(name),
            self.index.recipients.remove(name),
            self.index.scopes.remove(name),
        );
        self.index.entries.remove(name);
        for account in &accounts {
            self.index.entries.remove(account);
        }
        self.index.retired.insert(
            name.to_owned(),
            Retired {
                by: by.to_owned(),
                at_ms,
            },
        );
        if let Err(error) = self.write_index() {
            let (entries, ring, recipients, scope) = before;
            self.index.entries = entries;
            self.index.retired.remove(name);
            if let Some(ring) = ring {
                self.index.accounts.insert(name.to_owned(), ring);
            }
            if let Some(recipients) = recipients {
                self.index.recipients.insert(name.to_owned(), recipients);
            }
            if let Some(scope) = scope {
                self.index.scopes.insert(name.to_owned(), scope);
            }
            return Err(error);
        }
        self.sweep()?;
        Ok(entry)
    }
}
