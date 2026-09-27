//! Who a secret may be handed to. With no policy set, any identity the
//! permission source permits; with `people_only`, persons alone, so no
//! handle on the secret, issued or derived, reaches an agent.

use serde::{Deserialize, Serialize};

use super::SecretStore;
use crate::error::SecretsError;

/// A secret's recipient policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Recipients {
    /// Any identity the permission source permits.
    Anyone,
    /// Persons only.
    PeopleOnly,
}

impl Recipients {
    /// The policy as it is written.
    pub fn label(self) -> &'static str {
        match self {
            Self::Anyone => "anyone",
            Self::PeopleOnly => "people_only",
        }
    }

    /// Whether `identity` may receive a handle under this policy. A person
    /// is named `person-...` in the directory and `person:...` locally.
    pub fn admits(self, identity: &str) -> bool {
        match self {
            Self::Anyone => true,
            Self::PeopleOnly => identity.starts_with("person-") || identity.starts_with("person:"),
        }
    }
}

impl SecretStore {
    /// The recipient policy of `secret`.
    pub fn recipients(&self, secret: &str) -> Recipients {
        self.index
            .recipients
            .get(secret)
            .copied()
            .unwrap_or(Recipients::Anyone)
    }

    /// Sets the recipient policy of `secret`.
    ///
    /// # Errors
    ///
    /// `SecretUnknown`, and the index write's refusals.
    pub(crate) fn set_recipients(
        &mut self,
        secret: &str,
        policy: Recipients,
    ) -> Result<(), SecretsError> {
        if self.entry(secret).is_none() {
            return Err(SecretsError::SecretUnknown {
                name: secret.to_owned(),
            });
        }
        self.index.recipients.insert(secret.to_owned(), policy);
        self.write_index()
    }
}
