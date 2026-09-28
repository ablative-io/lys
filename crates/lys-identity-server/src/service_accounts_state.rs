//! What the service accounts' log folds to, and how that fold is sealed in
//! the log's signed snapshot so a start reads only the leaves after it.
//!
//! A service account is owned by one person. It is created under the
//! operation id that names it, and retired once, under an operation id of
//! its own. Every operation id names one line only.

use serde::{Deserialize, Serialize};

use crate::read_views::Login;

/// The snapshot domain the service accounts' folded state is sealed under.
pub const DOMAIN: &str = "lys/identity/service-accounts-state/v1";

const FORMAT: &str = "lys-service-accounts-state/v1";

/// A service account as it was created.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Created {
    /// The operation id it was created with, which names it.
    pub id: String,
    /// The person who owns it.
    pub owner: String,
    /// Its name.
    pub name: String,
    /// What it is for, in its creator's words; empty when they said nothing.
    pub description: String,
    /// The login that created it.
    pub by: Login,
    /// When it was created, in seconds since the Unix epoch.
    pub at: u64,
}

/// A service account retired.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Retired {
    /// The operation id it was retired with.
    pub operation: String,
    /// The service account retired.
    pub account: String,
    /// The login that retired it.
    pub by: Login,
    /// When it was retired, in seconds since the Unix epoch.
    pub at: u64,
}

/// One line of the log.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "line", rename_all = "snake_case")]
pub enum Line {
    /// A service account created.
    Created(Created),
    /// A service account retired.
    Retired(Retired),
}

/// One service account, with its retirement once it has one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Account {
    /// How it was created.
    pub created: Created,
    /// How it was retired, null while it is in use.
    pub retired: Option<Retired>,
}

impl Account {
    /// Whether the account is retired.
    pub fn is_retired(&self) -> bool {
        self.retired.is_some()
    }
}

/// The service accounts as their log folds them, in the order created.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Held {
    /// The service accounts.
    pub accounts: Vec<Account>,
}

/// The sealed state as it is read back: owned, since decoding makes it.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Sealed {
    format: String,
    held: Held,
}

/// The sealed state as it is written: the format and a borrow of what is
/// held, so sealing serialises the state where it lies and copies none of it.
/// Its fields and their order are `Sealed`'s, so the bytes are the same.
#[derive(Serialize)]
struct SealedRef<'a> {
    format: &'a str,
    held: &'a Held,
}

impl Held {
    /// The service account named `id`.
    pub fn account(&self, id: &str) -> Option<&Account> {
        self.accounts
            .iter()
            .find(|account| account.created.id == id)
    }

    /// The line kept under `operation`, whichever kind it is.
    pub fn operation(&self, operation: &str) -> Option<Line> {
        self.accounts.iter().find_map(|account| {
            if account.created.id == operation {
                return Some(Line::Created(account.created.clone()));
            }
            account
                .retired
                .as_ref()
                .filter(|retired| retired.operation == operation)
                .map(|retired| Line::Retired(retired.clone()))
        })
    }

    /// Fold one line. A line the lines before it do not allow is refused by
    /// reason, since every kept line was checked against what came before.
    pub fn hold(&mut self, line: Line) -> Result<(), String> {
        let operation = match &line {
            Line::Created(created) => &created.id,
            Line::Retired(retired) => &retired.operation,
        };
        if self.operation(operation).is_some() {
            return Err(format!("operation `{operation}` already names a line"));
        }
        match line {
            Line::Created(created) => {
                self.accounts.push(Account {
                    created,
                    retired: None,
                });
                Ok(())
            }
            Line::Retired(retired) => {
                let account = self
                    .accounts
                    .iter_mut()
                    .find(|account| account.created.id == retired.account)
                    .ok_or_else(|| {
                        format!(
                            "retirement `{}` names service account `{}`, which was never created",
                            retired.operation, retired.account
                        )
                    })?;
                if account.retired.is_some() {
                    return Err(format!(
                        "retirement `{}` names service account `{}`, which was already retired",
                        retired.operation, retired.account
                    ));
                }
                account.retired = Some(retired);
                Ok(())
            }
        }
    }

    /// Fold every leaf of `tail`, in order.
    pub fn fold(&mut self, tail: &lys_log_store::Tail) -> Result<(), String> {
        for (index, bytes) in (tail.from..).zip(&tail.leaves) {
            let line = serde_json::from_slice(bytes)
                .map_err(|error| format!("leaf {index} is not a service account line: {error}"))?;
            self.hold(line)
                .map_err(|reason| format!("leaf {index}: {reason}"))?;
        }
        Ok(())
    }

    /// The state a snapshot seals.
    pub fn encode(&self) -> Result<Vec<u8>, String> {
        serde_json::to_vec(&SealedRef {
            format: FORMAT,
            held: self,
        })
        .map_err(|error| format!("service accounts state: {error}"))
    }

    /// The state a snapshot sealed, refused by reason unless it reads whole
    /// in this format.
    pub fn decode(bytes: &[u8]) -> Result<Self, String> {
        let sealed: Sealed = serde_json::from_slice(bytes)
            .map_err(|error| format!("service accounts state: {error}"))?;
        if sealed.format != FORMAT {
            return Err(format!(
                "service accounts state is in format {}, not {FORMAT}",
                sealed.format
            ));
        }
        Ok(sealed.held)
    }
}

#[cfg(test)]
#[path = "service_accounts_state_tests.rs"]
mod tests;
