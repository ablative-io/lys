//! The roles as they are kept: an initial snapshot and durable changes in
//! one file. Changes append only their own records; periodic snapshots
//! bound restart replay. An existing snapshot is migrated atomically with
//! the first change.
//!
//! A change is written before it is answered. When a write fails, what the
//! file holds is read again before anything else is answered, so memory
//! never runs ahead of or behind the file.
//!
//! A role, a version and a holding are each named by the operation id they
//! were made with, so making one again in the same words answers what is
//! already kept, and the same operation in other words is refused.
//!
//! Nothing here moves a holder. A holder is at the version it was assigned
//! or last moved to until someone moves it, and the end of a holding is set
//! when it is assigned and changed by nothing after.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::ServerError;
use crate::roles_records::{Ending, Holding, Move, Role, Version, Words};

#[path = "roles_changes.rs"]
mod changes;

#[path = "roles_persistence.rs"]
mod persistence;

use changes::Change;
use persistence::Persistence;

#[cfg(test)]
std::thread_local! {
    static WRITTEN_BYTES: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static REPLAYED_CHANGES: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
    static RECOVERED_TAILS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

#[derive(Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Kept {
    roles: Vec<Role>,
}

/// The roles, read from their file and written to it.
pub struct RolesStore {
    persistence: Persistence,
    kept: Kept,
    uncertain: bool,
}

fn unavailable(what: impl std::fmt::Display) -> ServerError {
    ServerError::RolesUnavailable {
        reason: what.to_string(),
    }
}

/// The holding of `holder` that was assigned as `assignment`.
fn assigned<'a>(
    role: &'a Role,
    holder: &str,
    assignment: &str,
) -> Result<&'a Holding, ServerError> {
    if role.holding(holder).is_none() {
        return Err(ServerError::HolderUnknown);
    }
    role.holdings
        .iter()
        .find(|kept| kept.operation == assignment && kept.holder == holder)
        .ok_or(ServerError::HoldingChanged)
}

/// Whether the holding assigned as `assignment` is the last of `holder`.
fn stands_last(role: &Role, holder: &str, assignment: &str) -> bool {
    role.holding(holder)
        .is_some_and(|last| last.operation == assignment)
}

fn reused(operation: &str) -> ServerError {
    ServerError::RoleReused {
        operation: operation.to_owned(),
    }
}

#[cfg(test)]
#[path = "roles_persistence_tests.rs"]
mod persistence_tests;

impl RolesStore {
    /// The roles kept in the file `path`, none when it does not exist.
    pub fn open(path: &Path) -> Result<Self, ServerError> {
        let (persistence, kept) = Persistence::open(path)?;
        Ok(Self {
            persistence,
            kept,
            uncertain: false,
        })
    }

    /// Resolve a write whose outcome is not known, by reading the file again.
    pub fn settle(&mut self) -> Result<(), ServerError> {
        if self.uncertain {
            self.kept = self.persistence.read()?;
            self.uncertain = false;
        }
        Ok(())
    }

    fn write(&mut self, change: Change) -> Result<(), ServerError> {
        change.validate(&self.kept)?;
        if let Err(failure) = self.persistence.append(&self.kept, &change) {
            self.uncertain = true;
            self.settle()?;
            return Err(failure);
        }
        if let Err(failure) = change.apply(&mut self.kept) {
            self.uncertain = true;
            self.settle()?;
            return Err(failure);
        }
        if let Err(failure) = self.persistence.checkpoint_if_due(&self.kept) {
            self.uncertain = true;
            self.settle()?;
            return Err(failure);
        }
        Ok(())
    }

    /// Every role, in the order made.
    pub fn roles(&self) -> &[Role] {
        &self.kept.roles
    }

    /// The role named `id`.
    pub fn role(&self, id: &str) -> Option<&Role> {
        self.kept.roles.iter().find(|role| role.id == id)
    }

    /// Whether `operation` already names a role, a version or a holding.
    fn names(&self, operation: &str) -> bool {
        self.kept.roles.iter().any(|role| {
            role.id == operation
                || role
                    .versions
                    .iter()
                    .any(|version| version.operation == operation)
                || role
                    .holdings
                    .iter()
                    .any(|holding| holding.operation == operation)
        })
    }

    fn role_position(&self, id: &str) -> Result<usize, ServerError> {
        self.kept
            .roles
            .iter()
            .position(|role| role.id == id)
            .ok_or(ServerError::RoleUnknown)
    }

    /// Keep a new role at its first version. Made again in the same words it
    /// is kept once; the same operation in other words is refused.
    pub fn make(&mut self, name: String, first: Version) -> Result<(), ServerError> {
        self.settle()?;
        let id = first.operation.clone();
        if let Some(kept) = self.role(&id) {
            let same = kept.name == name
                && kept
                    .version(1)
                    .is_some_and(|version| version.words == first.words);
            return if same { Ok(()) } else { Err(reused(&id)) };
        }
        if self.names(&id) {
            return Err(reused(&id));
        }
        self.write(Change::Make {
            role: Role {
                id,
                name,
                versions: vec![first],
                holdings: Vec::new(),
            },
        })
    }

    /// Keep the next version of the role `id`, and answer its number. No
    /// holder is moved.
    pub fn revise(
        &mut self,
        id: &str,
        operation: &str,
        words: Words,
        by: &str,
        at: u64,
    ) -> Result<u32, ServerError> {
        self.settle()?;
        let role = self.role(id).ok_or(ServerError::RoleUnknown)?;
        let made = role
            .versions
            .iter()
            .find(|version| version.operation == operation);
        if let Some(kept) = made {
            let same = kept.number > 1 && kept.words == words;
            return if same {
                Ok(kept.number)
            } else {
                Err(reused(operation))
            };
        }
        if self.names(operation) {
            return Err(reused(operation));
        }
        let number = role
            .latest()
            .checked_add(1)
            .ok_or_else(|| unavailable("role version number overflow"))?;
        let version = Version {
            number,
            operation: operation.to_owned(),
            words,
            made_by: by.to_owned(),
            made_at: at,
        };
        self.write(Change::Revise {
            role: self.role_position(id)?,
            version,
        })?;
        Ok(number)
    }

    /// Keep `holding` of the role `id`, at the role's latest version.
    /// Assigned again in the same words it is kept once. A holder whose
    /// holding still stands at `holding.assigned_at` is refused another.
    pub fn assign(&mut self, id: &str, holding: Holding) -> Result<(), ServerError> {
        self.settle()?;
        let role = self.role(id).ok_or(ServerError::RoleUnknown)?;
        let assigned = role
            .holdings
            .iter()
            .find(|kept| kept.operation == holding.operation);
        if let Some(kept) = assigned {
            let same = kept.holder == holding.holder && kept.ends_at == holding.ends_at;
            return if same {
                Ok(())
            } else {
                Err(reused(&holding.operation))
            };
        }
        if self.names(&holding.operation) {
            return Err(reused(&holding.operation));
        }
        let stands = role
            .holding(&holding.holder)
            .is_some_and(|kept| kept.state(holding.assigned_at) == "holding");
        if stands {
            return Err(ServerError::RoleHeld {
                holder: holding.holder,
            });
        }
        let holding = Holding {
            version: role.latest(),
            ..holding
        };
        self.write(Change::Assign {
            role: self.role_position(id)?,
            holding,
        })
    }

    /// Move the holding of `holder` assigned as `assignment` by `moved`,
    /// from the version it was seen at to a newer one. A move already made
    /// in the same words stays as it was made. A holding that is no longer
    /// the holder's last, or no longer at the version moved from, is
    /// refused. The end of the holding is not changed.
    pub fn move_holder(
        &mut self,
        id: &str,
        holder: &str,
        assignment: &str,
        moved: Move,
    ) -> Result<(), ServerError> {
        self.settle()?;
        let role = self.role(id).ok_or(ServerError::RoleUnknown)?;
        let (from, to) = (moved.from, moved.to);
        if role.version(to).is_none() || role.version(from).is_none() {
            return Err(ServerError::RoleVersionUnknown);
        }
        let kept = assigned(role, holder, assignment)?;
        let made = kept
            .moves
            .last()
            .is_some_and(|last| last.from == from && last.to == to);
        if made {
            return Ok(());
        }
        if !stands_last(role, holder, assignment) || kept.version != from {
            return Err(ServerError::HoldingChanged);
        }
        let state = kept.state(moved.at);
        if state != "holding" {
            return Err(ServerError::HoldingOver { state });
        }
        if to <= from {
            return Err(ServerError::RequestMalformed {
                reason: format!(
                    "the holder is at version {from}: a holder is moved to a newer version"
                ),
            });
        }
        let holding = role
            .holdings
            .iter()
            .position(|holding| holding.operation == kept.operation)
            .ok_or(ServerError::HolderUnknown)?;
        self.write(Change::Move {
            role: self.role_position(id)?,
            holding,
            moved,
        })
    }

    /// End the holding of `holder` assigned as `assignment` in the role
    /// `id`. A holding already ended stays as it was ended. A holding that
    /// is no longer the holder's last is refused.
    pub fn end(
        &mut self,
        id: &str,
        holder: &str,
        assignment: &str,
        ending: Ending,
    ) -> Result<(), ServerError> {
        self.settle()?;
        let role = self.role(id).ok_or(ServerError::RoleUnknown)?;
        let kept = assigned(role, holder, assignment)?;
        if kept.ended.is_some() {
            return Ok(());
        }
        if !stands_last(role, holder, assignment) {
            return Err(ServerError::HoldingChanged);
        }
        let holding = role
            .holdings
            .iter()
            .position(|holding| holding.operation == kept.operation)
            .ok_or(ServerError::HolderUnknown)?;
        self.write(Change::End {
            role: self.role_position(id)?,
            holding,
            ending,
        })
    }
}
