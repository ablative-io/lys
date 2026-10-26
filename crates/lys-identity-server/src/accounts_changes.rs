//! Only active account claims occupy indexed slots. A claim holds no lock
//! across issuer I/O; its outcome wakes only waiters for that account.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use tokio::sync::Notify;

use crate::error::ServerError;

#[derive(Default)]
pub(super) struct Changes {
    table: Mutex<Table>,
}

#[derive(Default)]
struct Table {
    accounts: HashMap<String, Entry>,
    active: usize,
}

struct Entry {
    outcome: Arc<Notify>,
    busy: bool,
    users: usize,
}

pub(crate) struct Guard<'a> {
    owner: &'a Changes,
    id: String,
    held: bool,
}

fn unavailable(reason: impl Into<String>) -> ServerError {
    ServerError::SignInProvidersUnavailable {
        reason: reason.into(),
    }
}

impl Changes {
    pub(super) async fn lock(&self, id: &str) -> Result<Guard<'_>, ServerError> {
        let outcome = {
            let mut table = self.table.lock().map_err(|error| {
                unavailable(format!(
                    "the account-change index lock is poisoned: {error}"
                ))
            })?;
            let entry = table
                .accounts
                .entry(id.to_owned())
                .or_insert_with(|| Entry {
                    outcome: Arc::new(Notify::new()),
                    busy: false,
                    users: 0,
                });
            entry.users += 1;
            let outcome = Arc::clone(&entry.outcome);
            table.active += 1;
            outcome
        };
        let mut guard = Guard {
            owner: self,
            id: id.to_owned(),
            held: false,
        };
        loop {
            let changed = outcome.notified();
            tokio::pin!(changed);
            changed.as_mut().enable();
            let claimed = {
                let mut table = self
                    .table
                    .lock()
                    .map_err(|error| unavailable(error.to_string()))?;
                let entry = table
                    .accounts
                    .get_mut(id)
                    .ok_or_else(|| unavailable("the account claim is missing"))?;
                if entry.busy {
                    false
                } else {
                    entry.busy = true;
                    true
                }
            };
            #[cfg(test)]
            crate::accounts::changes_tests::started(!claimed);
            if claimed {
                guard.held = true;
                return Ok(guard);
            }
            changed.await;
        }
    }
}

impl Drop for Guard<'_> {
    fn drop(&mut self) {
        match self.owner.table.lock() {
            Ok(mut table) => {
                if self.held
                    && let Some(entry) = table.accounts.get_mut(&self.id)
                {
                    entry.busy = false;
                    entry.outcome.notify_waiters();
                }
                match table.accounts.get_mut(&self.id) {
                    Some(entry) if entry.users > 1 => entry.users -= 1,
                    Some(_) => {
                        table.accounts.remove(&self.id);
                    }
                    None => tracing::error!(
                        refusal = "SignInProvidersUnavailable",
                        "an active account-change lock is missing"
                    ),
                }
                if table.active > 0 {
                    table.active -= 1;
                } else {
                    tracing::error!(
                        refusal = "SignInProvidersUnavailable",
                        "the active account-change count is missing"
                    );
                }
            }
            Err(error) => {
                tracing::error!(refusal = "SignInProvidersUnavailable", reason = %error, "the account-change index lock is poisoned");
            }
        }
    }
}

#[cfg(test)]
#[path = "accounts_lock_tests.rs"]
mod tests;
