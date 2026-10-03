//! Only active account changes occupy indexed async lock slots.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use tokio::sync::{Mutex as AsyncMutex, OwnedMutexGuard};

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
    lock: Arc<AsyncMutex<()>>,
    users: usize,
}

pub(crate) struct Guard<'a> {
    owner: &'a Changes,
    id: String,
    held: Option<OwnedMutexGuard<()>>,
}

fn unavailable(reason: impl Into<String>) -> ServerError {
    ServerError::SignInProvidersUnavailable {
        reason: reason.into(),
    }
}

impl Changes {
    pub(super) async fn lock(&self, id: &str) -> Result<Guard<'_>, ServerError> {
        let lock = {
            let mut table = self.table.lock().map_err(|error| {
                unavailable(format!(
                    "the account-change index lock is poisoned: {error}"
                ))
            })?;
            let entry = table
                .accounts
                .entry(id.to_owned())
                .or_insert_with(|| Entry {
                    lock: Arc::new(AsyncMutex::new(())),
                    users: 0,
                });
            entry.users += 1;
            let lock = Arc::clone(&entry.lock);
            table.active += 1;
            lock
        };
        let mut guard = Guard {
            owner: self,
            id: id.to_owned(),
            held: None,
        };
        guard.held = Some(if let Ok(held) = Arc::clone(&lock).try_lock_owned() {
            #[cfg(test)]
            crate::accounts::changes_tests::started(false);
            held
        } else {
            #[cfg(test)]
            crate::accounts::changes_tests::started(true);
            lock.lock_owned().await
        });
        Ok(guard)
    }
}

impl Drop for Guard<'_> {
    fn drop(&mut self) {
        drop(self.held.take());
        match self.owner.table.lock() {
            Ok(mut table) => {
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
