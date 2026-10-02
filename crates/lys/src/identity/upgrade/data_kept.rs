//! The data an upgrade starts from, kept so a put-back has it to return to.
//!
//! A new build may write its stores in a shape the previous build cannot
//! read, as an issuer move does. Once the service and the broker are
//! stopped, and before anything is placed, `data/` is copied whole to
//! `data.previous/`; a put-back replaces `data/` with that copy before it
//! starts the previous build, so the previous build starts on the data it
//! last ran on. That the copy was kept is marked by a file beside the
//! upgrade record, never by a step in it, so a release that predates the
//! copy still reads the record when it finishes or puts back an upgrade
//! this build began. Such a release puts back no data, since it knows no
//! mark, and its clear leaves the mark behind; the next upgrade forgets it
//! before writing its record, so it is never trusted. A file that is neither a regular file nor a directory is
//! refused by name before anything is copied, never skipped.

use std::path::Path;

use super::super::error::{ErrorKind, IdentityError, IdentityResult};
use super::super::install::layout::Layout;
use super::super::private_files;
use super::intent::{self, Intent, Step};

fn io(action: &'static str, path: &Path, error: &std::io::Error) -> IdentityError {
    IdentityError::new(
        ErrorKind::PrivateFileIo,
        action,
        "upgrade",
        error.to_string(),
    )
    .at(path)
}

fn remove(path: &Path) -> IdentityResult<()> {
    match std::fs::remove_dir_all(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(io("remove", path, &error)),
    }
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

/// Forgets any copy an earlier upgrade kept, so a put-back of the upgrade
/// about to begin never returns data from before it.
pub fn forget(layout: &Layout) -> IdentityResult<()> {
    intent::remove(&layout.upgrade_data_kept())
}

/// Whether the upgrade under way kept `data/`.
fn was_kept(layout: &Layout, intent: &Intent) -> bool {
    intent.has(Step::DataKept) || layout.upgrade_data_kept().is_file()
}

/// Copies `data/` to a fresh `data.previous/`, through a temporary name so
/// a crash never leaves a partial copy under the kept name, then marks it
/// kept.
pub fn keep(layout: &Layout) -> IdentityResult<()> {
    forget(layout)?;
    let (data, kept) = (layout.data_dir(), layout.data_previous_dir());
    let partial = layout.data_dir().with_file_name("data.previous.partial");
    remove(&partial)?;
    if data.is_dir() {
        copy_tree(&data, &partial)?;
    } else {
        std::fs::create_dir(&partial).map_err(|error| io("keep data", &partial, &error))?;
    }
    remove(&kept)?;
    std::fs::rename(&partial, &kept).map_err(|error| io("keep data", &partial, &error))?;
    private_files::ensure_dir(&layout.install_dir())?;
    private_files::write(&layout.upgrade_data_kept(), b"data.previous\n")?;
    Ok(())
}

/// Puts the kept data back in place of `data/`, when this upgrade kept it.
/// The kept copy stays, so a put-back that stops part-way can run again.
pub fn restore(layout: &Layout, intent: &Intent, say: &mut dyn FnMut(&str)) -> IdentityResult<()> {
    let kept = layout.data_previous_dir();
    if !was_kept(layout, intent) || !kept.is_dir() {
        return Ok(());
    }
    let data = layout.data_dir();
    let partial = layout.data_dir().with_file_name("data.restoring");
    remove(&partial)?;
    copy_tree(&kept, &partial)?;
    remove(&data)?;
    std::fs::rename(&partial, &data).map_err(|error| io("restore data", &partial, &error))?;
    say("the data the previous build last ran on is back");
    Ok(())
}

#[path = "data_kept_tests.rs"]
mod tests;
