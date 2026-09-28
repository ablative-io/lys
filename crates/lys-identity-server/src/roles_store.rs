//! The roles as they are kept: one file holding every role, its versions and
//! its holdings as they stand, replaced whole and atomically at each change.
//! A start reads that one file and nothing else, whatever was changed before.
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

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::ServerError;
use crate::roles_records::{Ending, Holding, Move, Role, Version, Words};

#[derive(Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Kept {
    roles: Vec<Role>,
}

/// The roles, read from their file and written to it.
pub struct RolesStore {
    path: PathBuf,
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

impl RolesStore {
    /// The roles kept in the file `path`, none when it does not exist.
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

    fn write(&mut self, roles: Vec<Role>) -> Result<(), ServerError> {
        let next = Kept { roles };
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

    /// The roles with the role `id` changed by `change`, written.
    fn change(
        &mut self,
        id: &str,
        change: impl FnOnce(&mut Role) -> Result<(), ServerError>,
    ) -> Result<(), ServerError> {
        let mut roles = self.kept.roles.clone();
        let role = roles
            .iter_mut()
            .find(|role| role.id == id)
            .ok_or(ServerError::RoleUnknown)?;
        change(role)?;
        self.write(roles)
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
        let mut roles = self.kept.roles.clone();
        roles.push(Role {
            id,
            name,
            versions: vec![first],
            holdings: Vec::new(),
        });
        self.write(roles)
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
        let number = role.latest() + 1;
        let version = Version {
            number,
            operation: operation.to_owned(),
            words,
            made_by: by.to_owned(),
            made_at: at,
        };
        self.change(id, |role| {
            role.versions.push(version);
            Ok(())
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
        self.change(id, |role| {
            role.holdings.push(holding);
            Ok(())
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
        let operation = kept.operation.clone();
        self.change(id, |role| {
            let holding = role
                .holdings
                .iter_mut()
                .find(|holding| holding.operation == operation)
                .ok_or(ServerError::HolderUnknown)?;
            holding.version = to;
            holding.moves.push(moved);
            Ok(())
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
        let operation = kept.operation.clone();
        self.change(id, |role| {
            let holding = role
                .holdings
                .iter_mut()
                .find(|holding| holding.operation == operation)
                .ok_or(ServerError::HolderUnknown)?;
            holding.ended = Some(ending);
            Ok(())
        })
    }
}
