//! What the service accounts' log folds to, and how that fold is sealed in
//! the log's signed snapshot so a start reads only the leaves after it.
//!
//! A service account is owned by one person. It is created under the
//! operation id that names it, and retired once, under an operation id of
//! its own. Every operation id names one line only.

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::read_views::Login;

#[path = "service_accounts_grant_index.rs"]
mod grant_index;

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
#[serde(deny_unknown_fields, from = "Records")]
pub struct Held {
    /// The service accounts.
    pub accounts: Vec<Account>,
    #[serde(skip)]
    index: Arc<Index>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct Index {
    accounts: HashMap<String, usize>,
    operations: HashMap<String, (usize, bool)>,
    refusals: HashMap<String, HashMap<String, HashMap<String, usize>>>,
    grants: grant_index::GrantIndex,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Records {
    accounts: Vec<Account>,
}

impl From<Records> for Held {
    fn from(records: Records) -> Self {
        let mut index = Index::default();
        for (position, account) in records.accounts.iter().enumerate() {
            index.grants.hold(&account.created, account.is_retired());
            index
                .accounts
                .entry(account.created.id.clone())
                .or_insert(position);
            index
                .operations
                .entry(account.created.id.clone())
                .or_insert((position, false));
            for (refusal_position, refused) in account.import_refusals.iter().enumerate() {
                index
                    .refusals
                    .entry(account.created.id.clone())
                    .or_default()
                    .entry(refused.operation.clone())
                    .or_default()
                    .entry(refused.refusal.clone())
                    .or_insert(refusal_position);
            }
            if let Some(retired) = &account.retired {
                index
                    .operations
                    .entry(retired.operation.clone())
                    .or_insert((position, true));
            }
        }
        Self {
            accounts: records.accounts,
            index: Arc::new(index),
        }
    }
}

#[derive(Serialize)]
struct Sealing<'a> {
    format: &'static str,
    held: &'a Held,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Sealed {
    format: String,
    held: Held,
}

impl Held {
    pub(crate) fn grant_accounts(
        &self,
    ) -> Result<Arc<lys_identity::projection::accounts::Accounts>, lys_identity::IdentityError>
    {
        self.index.grants.accounts()
    }

    /// The service account named `id`.
    pub fn account(&self, id: &str) -> Option<&Account> {
        self.index.accounts.get(id).and_then(|position| {
            #[cfg(test)]
            crate::folded_work::visit(crate::folded_work::Work::Account);
            self.accounts.get(*position)
        })
    }

    /// The creation or retirement kept under `operation`.
    pub fn operation(&self, operation: &str) -> Option<Line> {
        let (position, retired) = self.index.operations.get(operation)?;
        #[cfg(test)]
        crate::folded_work::visit(crate::folded_work::Work::Account);
        let account = self.accounts.get(*position)?;
        if *retired {
            account.retired.clone().map(Line::Retired)
        } else {
            Some(Line::Created(account.created.clone()))
        }
    }

    /// The first refusal for this account, operation and named outcome.
    pub fn import_refusal(
        &self,
        account: &str,
        operation: &str,
        refusal: &str,
    ) -> Option<&ImportRefused> {
        let position = self
            .index
            .refusals
            .get(account)?
            .get(operation)?
            .get(refusal)?;
        #[cfg(test)]
        crate::folded_work::visit(crate::folded_work::Work::Refusal);
        self.account(account)?.import_refusals.get(*position)
    }

    /// Fold one line. A line the lines before it do not allow is refused by
    /// reason, since every kept line was checked against what came before.
    pub fn hold(&mut self, line: Line) -> Result<(), String> {
        if let Line::ImportRefused(refused) = line {
            let position = self
                .index
                .accounts
                .get(&refused.account)
                .ok_or_else(|| "import refusal names no existing account".to_owned())?;
            if self
                .import_refusal(&refused.account, &refused.operation, &refused.refusal)
                .is_some()
            {
                return Err("import refusal already recorded".to_owned());
            }
            let account = &mut self.accounts[*position];
            Arc::make_mut(&mut self.index)
                .refusals
                .entry(refused.account.clone())
                .or_default()
                .entry(refused.operation.clone())
                .or_default()
                .insert(refused.refusal.clone(), account.import_refusals.len());
            account.import_refusals.push(refused);
            return Ok(());
        }
        let operation = match &line {
            Line::Created(created) => &created.id,
            Line::Retired(retired) => &retired.operation,
            Line::ImportRefused(_) => return Err("unexpected import refusal".to_owned()),
        };
        if self.index.operations.contains_key(operation) {
            return Err(format!("operation `{operation}` already names a line"));
        }
        match line {
            Line::ImportRefused(_) => Err("unexpected import refusal".to_owned()),
            Line::Created(created) => {
                let index = Arc::make_mut(&mut self.index);
                index.grants.hold(&created, false);
                index
                    .accounts
                    .insert(created.id.clone(), self.accounts.len());
                index
                    .operations
                    .insert(created.id.clone(), (self.accounts.len(), false));
                self.accounts.push(Account {
                    created,
                    retired: None,
                    import_refusals: Vec::new(),
                });
                Ok(())
            }
            Line::Retired(retired) => {
                let position = *self.index.accounts.get(&retired.account).ok_or_else(|| {
                    format!(
                        "retirement `{}` names service account `{}`, which was never created",
                        retired.operation, retired.account
                    )
                })?;
                let account = &mut self.accounts[position];
                if account.retired.is_some() {
                    return Err(format!(
                        "retirement `{}` names service account `{}`, which was already retired",
                        retired.operation, retired.account
                    ));
                }
                Arc::make_mut(&mut self.index)
                    .operations
                    .insert(retired.operation.clone(), (position, true));
                account.retired = Some(retired);
                Arc::make_mut(&mut self.index)
                    .grants
                    .hold(&account.created, true);
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
        serde_json::to_vec(&Sealing {
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
