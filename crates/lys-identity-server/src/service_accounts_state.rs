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

/// A refused import entry, with no credential or request payload retained.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ImportRefused {
    /// The actual authenticated service account.
    pub account: String,
    /// Content-derived operation, which may succeed after authority changes.
    pub operation: String,
    /// Document section and entry name.
    pub entry: String,
    /// The original named refusal.
    pub refusal: String,
    /// When this outcome was first recorded.
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
    /// A refused import, without changing account or grant authority.
    ImportRefused(ImportRefused),
}

/// One service account, with its retirement once it has one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Account {
    /// How it was created.
    pub created: Created,
    /// How it was retired, null while it is in use.
    pub retired: Option<Retired>,
    /// Refused imports, visible under the same account ownership rules.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub import_refusals: Vec<ImportRefused>,
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

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Sealed {
    format: String,
    held: Held,
}

impl Held {
    /// The service account named `id`.
    pub fn account(&self, id: &str) -> Option<&Account> {
        self.accounts.iter().find(|account| {
            #[cfg(test)]
            crate::folded_work::visit(crate::folded_work::Work::Account);
            account.created.id == id
        })
    }

    /// The line kept under `operation`, whichever kind it is.
    pub fn operation(&self, operation: &str) -> Option<Line> {
        self.accounts.iter().find_map(|account| {
            #[cfg(test)]
            crate::folded_work::visit(crate::folded_work::Work::Account);
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
        if let Line::ImportRefused(refused) = line {
            let account = self
                .accounts
                .iter_mut()
                .find(|account| account.created.id == refused.account)
                .ok_or_else(|| "import refusal names no existing account".to_owned())?;
            if account
                .import_refusals
                .iter()
                .any(|kept| kept.operation == refused.operation && kept.refusal == refused.refusal)
            {
                return Err("import refusal already recorded".to_owned());
            }
            account.import_refusals.push(refused);
            return Ok(());
        }
        let operation = match &line {
            Line::Created(created) => &created.id,
            Line::Retired(retired) => &retired.operation,
            Line::ImportRefused(_) => return Err("unexpected import refusal".to_owned()),
        };
        if self.operation(operation).is_some() {
            return Err(format!("operation `{operation}` already names a line"));
        }
        match line {
            Line::ImportRefused(_) => Err("unexpected import refusal".to_owned()),
            Line::Created(created) => {
                self.accounts.push(Account {
                    created,
                    retired: None,
                    import_refusals: Vec::new(),
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
        serde_json::to_vec(&Sealed {
            format: FORMAT.to_owned(),
            held: self.clone(),
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
