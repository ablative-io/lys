//! The machines agents can run on, as they are kept: one file holding every
//! machine as it stands, replaced whole and atomically at each change. A
//! start reads that one file and nothing else, whatever was changed before.
//!
//! A change is written before it is answered. When a write fails, what the
//! file holds is read again before anything else is answered, so memory
//! never runs ahead of or behind the file.
//!
//! A machine is named by the operation id it was named with, so naming it
//! again in the same words answers the machine already kept, and the same
//! operation in other words is refused.

use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard, PoisonError};

use serde::{Deserialize, Serialize};

use crate::error::ServerError;

/// A machine's retirement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Retirement {
    /// The person who retired it.
    pub by: String,
    /// When, in seconds since the Unix epoch.
    pub at: u64,
}

/// A machine as it is kept.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Machine {
    /// The operation id it was named with, which names it.
    pub id: String,
    /// Its name, as people call it.
    pub name: String,
    /// What kind of machine it is, in the namer's words.
    pub kind: String,
    /// The runtime installed on it, null for none.
    pub runtime: Option<String>,
    /// How many agents it runs at once.
    pub slots: u32,
    /// The agents that may run on it.
    pub may_run: Vec<String>,
    /// The roles whose holders may run on it, by role id.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub may_run_roles: Vec<String>,
    /// The hosts an agent on it may reach.
    pub may_reach: Vec<String>,
    /// The person who named it.
    pub named_by: String,
    /// When it was named, in seconds since the Unix epoch.
    pub named_at: u64,
    /// Its retirement, null while it is in use.
    pub retired: Option<Retirement>,
}

#[derive(Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Kept {
    machines: Vec<Machine>,
}

/// A machine as a session answer names it: its name and its runtime.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Named {
    /// The machine's name, as people call it.
    pub name: String,
    /// The runtime installed on it, null for none.
    pub runtime: Option<String>,
}

/// The machines behind one lock, counting each time it is taken, so a test
/// counts how often a route locks the machines rather than timing it.
pub struct NetworkLock {
    store: Mutex<NetworkStore>,
    taken: AtomicU64,
}

impl NetworkLock {
    /// The machines of `store`, behind their lock.
    pub fn new(store: NetworkStore) -> Self {
        Self {
            store: Mutex::new(store),
            taken: AtomicU64::new(0),
        }
    }

    /// Take the lock, counting it. A lock a panicking holder poisoned is
    /// taken as it stands, as every store's lock is.
    pub fn lock(&self) -> MutexGuard<'_, NetworkStore> {
        self.taken.fetch_add(1, Ordering::Relaxed);
        self.store.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// How many times the lock was taken.
    pub fn taken(&self) -> u64 {
        self.taken.load(Ordering::Relaxed)
    }
}

/// The machines, read from their file and written to it.
pub struct NetworkStore {
    path: PathBuf,
    kept: Kept,
    uncertain: bool,
}

fn unavailable(what: impl std::fmt::Display) -> ServerError {
    ServerError::NetworkUnavailable {
        reason: what.to_string(),
    }
}

fn read(path: &Path) -> Result<Kept, ServerError> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Kept::default());
        }
        Err(error) => {
            return Err(unavailable(format!("reading {}: {error}", path.display())));
        }
    };
    serde_json::from_slice(&bytes)
        .map_err(|error| unavailable(format!("{} does not read: {error}", path.display())))
}

fn replace(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let beside = path.with_extension("writing");
    let mut file = fs::File::create(&beside)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    drop(file);
    fs::rename(&beside, path)?;
    match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => fs::File::open(parent)?.sync_all(),
        _ => Ok(()),
    }
}

impl NetworkStore {
    /// The machines kept in the file `path`, none when it does not exist.
    pub fn open(path: &Path) -> Result<Self, ServerError> {
        Ok(Self {
            path: path.to_owned(),
            kept: read(path)?,
            uncertain: false,
        })
    }

    /// Resolve a write whose outcome is not known, by reading the file again.
    pub fn settle(&mut self) -> Result<(), ServerError> {
        if self.uncertain {
            self.kept = read(&self.path)?;
            self.uncertain = false;
        }
        Ok(())
    }

    fn write(&mut self, next: Kept) -> Result<(), ServerError> {
        let bytes = serde_json::to_vec_pretty(&next).map_err(unavailable)?;
        if let Err(failure) = replace(&self.path, &bytes) {
            self.uncertain = true;
            self.settle()?;
            return Err(unavailable(format!(
                "writing {}: {failure}",
                self.path.display()
            )));
        }
        self.kept = next;
        Ok(())
    }

    /// Every machine, in the order named.
    pub fn machines(&self) -> &[Machine] {
        &self.kept.machines
    }

    /// The machine named `id`.
    pub fn machine(&self, id: &str) -> Option<&Machine> {
        self.kept.machines.iter().find(|machine| machine.id == id)
    }

    /// The name of the machine named `id` and the runtime installed on it.
    pub fn named(&self, id: &str) -> Option<Named> {
        self.machine(id).map(|machine| Named {
            name: machine.name.clone(),
            runtime: machine.runtime.clone(),
        })
    }

    /// Each machine's name and the runtime installed on it, by id, read in
    /// one pass; the first machine kept under an id answers for it, as it
    /// does for [`NetworkStore::machine`].
    pub fn names(&self) -> HashMap<String, Named> {
        let mut names = HashMap::with_capacity(self.kept.machines.len());
        for machine in &self.kept.machines {
            names.entry(machine.id.clone()).or_insert_with(|| Named {
                name: machine.name.clone(),
                runtime: machine.runtime.clone(),
            });
        }
        names
    }

    /// Keep `machine`. Named again in the same words it is kept once; the
    /// same operation in other words is refused.
    pub fn name(&mut self, machine: Machine) -> Result<(), ServerError> {
        self.settle()?;
        match self.machine(&machine.id) {
            Some(kept) if same_words(kept, &machine) => Ok(()),
            Some(_) => Err(ServerError::MachineReused {
                machine: machine.id,
            }),
            None => {
                let mut machines = self.kept.machines.clone();
                machines.push(machine);
                self.write(Kept { machines })
            }
        }
    }

    /// Retire the machine `id`. A machine already retired stays as it was retired.
    pub fn retire(&mut self, id: &str, retirement: Retirement) -> Result<(), ServerError> {
        self.settle()?;
        let mut machines = self.kept.machines.clone();
        let machine = machines
            .iter_mut()
            .find(|machine| machine.id == id)
            .ok_or(ServerError::MachineUnknown)?;
        if machine.retired.is_some() {
            return Ok(());
        }
        machine.retired = Some(retirement);
        self.write(Kept { machines })
    }
}

/// Whether two namings are the same machine, whenever each was named.
fn same_words(kept: &Machine, named: &Machine) -> bool {
    let timeless = Machine {
        named_at: kept.named_at,
        retired: kept.retired.clone(),
        ..named.clone()
    };
    *kept == timeless
}
