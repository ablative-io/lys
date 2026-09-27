//! Whose a secret is: one person's, a team's or an organisation's. A secret
//! sealed with no scope named is personal to its owner. The scope bounds
//! who may discover, read, use or be lent it, whatever grants exist.

use serde::{Deserialize, Serialize};

use super::SecretStore;
use crate::error::SecretsError;

/// A secret's scope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "name")]
pub enum Scope {
    /// One person's: that person, and the identities the permission source
    /// says act for them.
    Personal(String),
    /// A team's: the identities the permission source says are its members.
    Team(String),
    /// An organisation's: the identities the permission source says are
    /// its members.
    Organisation(String),
}

impl Scope {
    /// The scope as the permission source names it: `person/<id>`,
    /// `team/<name>` or `organisation/<name>`.
    pub fn target(&self) -> String {
        match self {
            Self::Personal(person) => format!("person/{person}"),
            Self::Team(team) => format!("team/{team}"),
            Self::Organisation(organisation) => format!("organisation/{organisation}"),
        }
    }

    /// Reads `personal:<person>`, `team:<name>` or `organisation:<name>`.
    ///
    /// # Errors
    ///
    /// `InvalidScope` when the text is none of those, or names nothing.
    pub fn parse(text: &str) -> Result<Self, SecretsError> {
        let invalid = || SecretsError::InvalidScope {
            text: text.to_owned(),
        };
        let (kind, name) = text.split_once(':').ok_or_else(invalid)?;
        if name.is_empty() {
            return Err(invalid());
        }
        match kind {
            "personal" => Ok(Self::Personal(name.to_owned())),
            "team" => Ok(Self::Team(name.to_owned())),
            "organisation" => Ok(Self::Organisation(name.to_owned())),
            _ => Err(invalid()),
        }
    }
}

impl SecretStore {
    /// The scope of `secret`: the one set on it, or personal to its owner.
    /// `None` when no such secret is sealed.
    pub fn scope(&self, secret: &str) -> Option<Scope> {
        let entry = self.entry(secret)?;
        Some(
            self.index
                .scopes
                .get(secret)
                .cloned()
                .unwrap_or_else(|| Scope::Personal(entry.owner.clone())),
        )
    }

    /// Sets the scope of `secret`.
    ///
    /// # Errors
    ///
    /// `SecretUnknown`, and the index write's refusals.
    pub(crate) fn set_scope(&mut self, secret: &str, scope: Scope) -> Result<(), SecretsError> {
        if self.entry(secret).is_none() {
            return Err(SecretsError::SecretUnknown {
                name: secret.to_owned(),
            });
        }
        self.index.scopes.insert(secret.to_owned(), scope);
        self.write_index()
    }
}
