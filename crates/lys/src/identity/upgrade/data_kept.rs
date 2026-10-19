//! The data an upgrade starts from, kept so a put-back has it to return to,
//! and the put-back that returns it while removing nothing (DIRECTORY-082
//! R1, carried by DIRECTORY-107 R1).
//!
//! A new build may write its stores in a shape the previous build cannot
//! read, as an issuer move does. Once the service and the broker are
//! stopped, and before anything is placed, `data/` is copied whole to
//! `data.previous/`, and the join set outside it (the service key, this
//! computer's key, its saved join and the server key its runner trusts) to
//! `join.previous/`, so a machine's record on the server and the computer's
//! saved connection to it always come back from the same moment
//! (DIRECTORY-107 R3). A put-back moves the window's `data/` whole to
//! `data.rolled-back/<rfc3339>/`, the instant it began, and the window's join
//! files to `data.rolled-back/<rfc3339>.join-set/`, before the kept copies
//! are put in place; it then says that folder and each store under it with
//! its count of records, read from the store's head, and records every store
//! it shrank beside the receipt, so the service's next boot refuses the
//! shrunk stores by name until an operator accepts the loss. The installer
//! never prunes `data.rolled-back`, and the kept copies stay, so a put-back
//! that stops part-way can run again.
//!
//! The copy is recorded as the step `data_kept` in the upgrade record, and
//! deliberately so: a release from before the copy cannot put data back,
//! and its own build cannot start on the data a new build has written, so it
//! must refuse the record by that unknown step before it stops or moves
//! anything, leaving the record for this build's installer to finish. The
//! step is recorded only once the copy stands. A file that is neither a
//! regular file nor a directory is refused by name before anything is
//! copied, never skipped.

use std::path::{Path, PathBuf};

use lys_identity::stores_receipt::{self as receipt, Acceptance, Kind};
use lys_log_store::{FileLeafStore, LeafStore};

use super::super::error::{ErrorKind, IdentityError, IdentityResult};
use super::super::install::layout::Layout;
use super::intent::{Intent, Step};

fn io(action: &'static str, path: &Path, error: &std::io::Error) -> IdentityError {
    IdentityError::new(
        ErrorKind::PrivateFileIo,
        action,
        "upgrade",
        error.to_string(),
    )
    .at(path)
}

/// Removes a scratch copy this module made; never a store.
fn remove(path: &Path) -> IdentityResult<()> {
    match std::fs::remove_dir_all(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(io("remove", path, &error)),
    }
}

fn rename(from: &Path, to: &Path, action: &'static str) -> IdentityResult<()> {
    std::fs::rename(from, to).map_err(|error| io(action, from, &error))
}

/// Copies the tree at `from` to `to`, which must not exist, keeping each
/// file's and each directory's permissions.
fn copy_tree(from: &Path, to: &Path) -> IdentityResult<()> {
    let permissions = std::fs::metadata(from)
        .map_err(|error| io("keep data", from, &error))?
        .permissions();
    std::fs::create_dir(to).map_err(|error| io("keep data", to, &error))?;
    for entry in std::fs::read_dir(from).map_err(|error| io("keep data", from, &error))? {
        let entry = entry.map_err(|error| io("keep data", from, &error))?;
        let (source, target) = (entry.path(), to.join(entry.file_name()));
        let kind = entry
            .file_type()
            .map_err(|error| io("keep data", &source, &error))?;
        if kind.is_dir() {
            copy_tree(&source, &target)?;
        } else if kind.is_file() {
            std::fs::copy(&source, &target).map_err(|error| io("keep data", &source, &error))?;
        } else {
            return Err(IdentityError::new(
                ErrorKind::PrivateFileIo,
                "keep data",
                "upgrade",
                "neither a regular file nor a directory, so it cannot be kept for a put-back",
            )
            .at(&source));
        }
    }
    std::fs::set_permissions(to, permissions).map_err(|error| io("keep data", to, &error))
}

/// The join set outside `data/`, by its place under the install root.
fn join_files(layout: &Layout) -> [(PathBuf, PathBuf); 4] {
    let run = layout.run_dir();
    [
        (PathBuf::from("keys/service.key"), layout.service_key()),
        (
            PathBuf::from("keys/runner-machine.key"),
            layout.runner_machine_key(),
        ),
        (
            PathBuf::from("run/runner-join.json"),
            run.join("runner-join.json"),
        ),
        (
            PathBuf::from("run/runner-server.pub"),
            run.join("runner-server.pub"),
        ),
    ]
}

fn join_kept(layout: &Layout) -> PathBuf {
    layout.root.join("join.previous")
}

/// Makes `dir` and its parents owner-only.
fn private_dir(dir: &Path) -> IdentityResult<()> {
    std::fs::create_dir_all(dir).map_err(|error| io("keep data", dir, &error))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))
            .map_err(|error| io("keep data", dir, &error))?;
    }
    Ok(())
}

/// Copies the join set that exists to a fresh `join.previous/`.
fn keep_joins(layout: &Layout) -> IdentityResult<()> {
    let partial = layout.root.join("join.previous.partial");
    remove(&partial)?;
    for (place, live) in join_files(layout) {
        if !live.is_file() {
            continue;
        }
        let kept = partial.join(&place);
        private_dir(kept.parent().unwrap_or(&partial))?;
        std::fs::copy(&live, &kept).map_err(|error| io("keep join set", &live, &error))?;
    }
    private_dir(&partial)?;
    remove(&join_kept(layout))?;
    rename(&partial, &join_kept(layout), "keep join set")
}

/// Copies `data/` to a fresh `data.previous/`, through a temporary name so
/// a crash never leaves a partial copy under the kept name, and the join
/// set to `join.previous/`.
///
/// # Errors
/// Returns the named failure when installed data cannot be retained or restored.
pub fn keep(layout: &Layout, intent: &mut Intent) -> IdentityResult<()> {
    let (data, kept) = (layout.data_dir(), layout.data_previous_dir());
    let partial = layout.data_dir().with_file_name("data.previous.partial");
    remove(&partial)?;
    if data.is_dir() {
        copy_tree(&data, &partial)?;
    } else {
        std::fs::create_dir(&partial).map_err(|error| io("keep data", &partial, &error))?;
    }
    keep_joins(layout)?;
    remove(&kept)?;
    rename(&partial, &kept, "keep data")?;
    intent.done(layout, Step::DataKept)
}

/// Puts the kept data back in place of `data/`, when this upgrade kept it,
/// moving the window's data aside under the instant the put-back began.
///
/// # Errors
/// Returns the named failure when installed data cannot be retained or restored.
pub fn restore(layout: &Layout, intent: &Intent, say: &mut dyn FnMut(&str)) -> IdentityResult<()> {
    restore_at(layout, intent, &receipt::now(), say)
}

/// [`restore`], the put-back having begun at `at`, RFC 3339 UTC.
///
/// # Errors
/// `rolled_back_name_taken` when `data.rolled-back/<at>` is taken, moving
/// nothing; the named failure of any move or copy.
pub fn restore_at(
    layout: &Layout,
    intent: &Intent,
    at: &str,
    say: &mut dyn FnMut(&str),
) -> IdentityResult<()> {
    let kept = layout.data_previous_dir();
    if !intent.has(Step::DataKept) || !kept.is_dir() {
        return Ok(());
    }
    let rolled = layout.root.join("data.rolled-back");
    let aside = rolled.join(at);
    let joins_aside = rolled.join(format!("{at}.join-set"));
    if let Some(taken) = [&aside, &joins_aside]
        .into_iter()
        .find(|path| path.exists())
    {
        return Err(IdentityError::new(
            ErrorKind::RolledBackNameTaken,
            "put back",
            "data.rolled-back",
            format!("{} is taken; nothing was moved", taken.display()),
        )
        .at(taken));
    }
    private_dir(&rolled)?;
    let data = layout.data_dir();
    let partial = layout.data_dir().with_file_name("data.restoring");
    remove(&partial)?;
    copy_tree(&kept, &partial)?;
    let moved = data.exists();
    if moved {
        rename(&data, &aside, "move the window's data aside")?;
    }
    rename(&partial, &data, "restore data")?;
    restore_joins(layout, &joins_aside)?;
    if moved {
        say(&format!(
            "the window's data is kept in {}, none of it removed; its stores:",
            aside.display()
        ));
        let counted = counts(&aside, say);
        record_shrunk(layout, &counted, at)?;
    }
    say("the data the previous build last ran on is back");
    Ok(())
}

/// Puts the kept join set back, the window's copy of each file kept under
/// `aside` first; a file the kept set lacks is moved there whole.
fn restore_joins(layout: &Layout, aside: &Path) -> IdentityResult<()> {
    let kept_set = join_kept(layout);
    if !kept_set.is_dir() {
        return Ok(());
    }
    for (place, live) in join_files(layout) {
        let (kept, window) = (kept_set.join(&place), aside.join(&place));
        if live.is_file() {
            private_dir(window.parent().unwrap_or(aside))?;
            std::fs::copy(&live, &window).map_err(|error| io("keep join set", &live, &error))?;
        }
        if kept.is_file() {
            let folder = live.parent().unwrap_or(&layout.root);
            private_dir(folder)?;
            let placing = live.with_extension("restoring");
            std::fs::copy(&kept, &placing)
                .map_err(|error| io("restore join set", &kept, &error))?;
            rename(&placing, &live, "restore join set")?;
        } else if live.is_file() {
            std::fs::remove_file(&live)
                .map_err(|error| io("move join file aside", &live, &error))?;
        }
    }
    Ok(())
}

/// The records of the log store at `dir`, read from its head, or why not.
fn log_records(dir: &Path) -> Result<u64, String> {
    FileLeafStore::open_read_only(dir)
        .map(|store| store.extent())
        .map_err(|error| error.to_string())
}

/// Says each store under `aside` with its count of records, and answers the
/// counted ones by name.
fn counts(aside: &Path, say: &mut dyn FnMut(&str)) -> Vec<(String, u64)> {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(aside)
        .map(|read| {
            read.filter_map(Result::ok)
                .map(|entry| entry.path())
                .collect()
        })
        .unwrap_or_default();
    entries.sort();
    let mut counted = Vec::new();
    for path in entries {
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        if !path.is_dir() {
            say(&format!(
                "  {name}: not counted here: a one-file store, counted by the service against its receipt at its next start"
            ));
            continue;
        }
        match log_records(&path) {
            Ok(records) => {
                say(&format!("  {name}: {records} records"));
                counted.push((name, records));
            }
            Err(reason) => say(&format!("  {name}: not counted here: {reason}")),
        }
    }
    counted
}

/// Records, beside the receipt, every log store the put-back left with
/// fewer records than the window held: the service refuses each by name
/// until an operator accepts the loss.
fn record_shrunk(layout: &Layout, window: &[(String, u64)], at: &str) -> IdentityResult<()> {
    let shrunk: Vec<Acceptance> = window
        .iter()
        .filter_map(|(name, held)| {
            let found = log_records(&layout.data_dir().join(name)).unwrap_or(0);
            (found < *held).then(|| Acceptance {
                kind: Kind::RolledBack,
                store: name.clone(),
                held: format!("{held} records"),
                found: format!("{found} records"),
                at: at.to_owned(),
                by: "lys identity put-back".to_owned(),
            })
        })
        .collect();
    if shrunk.is_empty() {
        return Ok(());
    }
    let path = layout.stores_accepted();
    receipt::append_acceptances(&path, &shrunk).map_err(|error| {
        IdentityError::new(
            ErrorKind::StoresReceiptUnreadable,
            "record put-back",
            "stores.accepted.json",
            error.to_string(),
        )
        .at(&path)
    })
}

#[path = "data_kept_tests.rs"]
mod tests;
