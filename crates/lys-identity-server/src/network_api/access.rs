//! All service runner assignments share one check under the network lock.

use std::ops::{Deref, DerefMut};

use crate::error::ServerError;
use crate::network_store::NetworkStore;
use crate::runner_client::RunnerRecord;

pub(crate) struct NetworkAccess<'a> {
    store: &'a mut NetworkStore,
}

impl<'a> NetworkAccess<'a> {
    pub(super) fn new(store: &'a mut NetworkStore) -> Self {
        Self { store }
    }

    pub(crate) fn name_runner(
        &mut self,
        id: &str,
        runner: Option<RunnerRecord>,
    ) -> Result<(), ServerError> {
        let machine = self.store.machine(id).ok_or(ServerError::MachineUnknown)?;
        if machine.retired.is_some() {
            return Err(ServerError::MachineRetired);
        }
        if let Some(candidate) = &runner {
            for machine in self.store.machines() {
                if machine.id != id
                    && machine.retired.is_none()
                    && self
                        .store
                        .runner(&machine.id)
                        .is_some_and(|held| same_runner(candidate, held))
                {
                    return Err(ServerError::Runner {
                        refusal: "runner_held_by_machine".to_owned(),
                        words: format!(
                            "machine {} already holds this runner; retire it or clear its runner before assigning it to {id}",
                            machine.id
                        ),
                    });
                }
            }
        }
        self.store.name_runner(id, runner)
    }
}

fn same_runner(first: &RunnerRecord, second: &RunnerRecord) -> bool {
    match (first, second) {
        (RunnerRecord::Socket { path: first }, RunnerRecord::Socket { path: second }) => {
            std::path::Path::new(first) == std::path::Path::new(second)
        }
        (
            RunnerRecord::Dialled {
                key: first_key,
                runner: first_id,
            },
            RunnerRecord::Dialled {
                key: second_key,
                runner: second_id,
            },
        ) => {
            first_key.eq_ignore_ascii_case(second_key)
                || first_id
                    .as_ref()
                    .zip(second_id.as_ref())
                    .is_some_and(|(a, b)| a == b)
        }
        // The store preserves the existing refusal for two own-runner assignments.
        _ => false,
    }
}

impl Deref for NetworkAccess<'_> {
    type Target = NetworkStore;
    fn deref(&self) -> &Self::Target {
        self.store
    }
}

impl DerefMut for NetworkAccess<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.store
    }
}
