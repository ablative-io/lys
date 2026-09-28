//! Where the container engine's command-line tools are, for an install
//! started by a person's app rather than from a terminal.
//!
//! A program opened from the Finder is given only the system's own `PATH`
//! (`/usr/bin:/bin:/usr/sbin:/sbin`), which holds no container engine's
//! tools. Every `docker` the install runs is therefore found on the `PATH`
//! it was given followed by the places the engines this install supports
//! put their tools, and is run with that same search path, so the engine's
//! own helpers (its credential helper, its compose plugin) are found by it
//! too. The search is over the file system as it stands; nothing is waited
//! on and nothing is asked again.
//!
//! Invariant: a `PATH` that cannot be joined is refused by name, never
//! replaced by a guess; a `docker` found nowhere is left as the bare name,
//! so the start that follows refuses it by name as not installed.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::super::error::{ErrorKind, IdentityError, IdentityResult};

/// The program every engine this install supports answers to.
pub const DOCKER: &str = "docker";

/// Where the engines this install supports put their tools on a Mac, in the
/// order they are searched after `PATH`: Docker Desktop's link, Homebrew's
/// two prefixes, Docker Desktop's own folder, and the per-user folders of
/// Docker Desktop, `OrbStack` and Rancher Desktop.
pub fn engine_dirs(home: Option<&Path>) -> Vec<PathBuf> {
    let mut dirs = vec![
        PathBuf::from("/usr/local/bin"),
        PathBuf::from("/opt/homebrew/bin"),
        PathBuf::from("/Applications/Docker.app/Contents/Resources/bin"),
    ];
    if let Some(home) = home {
        dirs.push(home.join(".docker").join("bin"));
        dirs.push(home.join(".orbstack").join("bin"));
        dirs.push(home.join(".rd").join("bin"));
    }
    dirs
}

/// The search path: `given` (the `PATH` this process has) followed by every
/// directory of [`engine_dirs`] it does not already hold.
pub fn search_dirs(given: Option<&OsString>, home: Option<&Path>) -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = given
        .map(|path| std::env::split_paths(path).collect())
        .unwrap_or_default();
    for dir in engine_dirs(home) {
        if !dirs.contains(&dir) {
            dirs.push(dir);
        }
    }
    dirs
}

/// The first `name` that is a file in `dirs`, or the bare name when none is.
pub fn find(name: &str, dirs: &[PathBuf]) -> PathBuf {
    dirs.iter()
        .map(|dir| dir.join(name))
        .find(|candidate| candidate.is_file())
        .unwrap_or_else(|| PathBuf::from(name))
}

/// This process's search path, joined for a child's `PATH`.
pub fn search_path() -> IdentityResult<(Vec<PathBuf>, OsString)> {
    let given = std::env::var_os("PATH");
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let dirs = search_dirs(given.as_ref(), home.as_deref());
    let joined = std::env::join_paths(&dirs).map_err(|error| {
        IdentityError::new(
            ErrorKind::ConfigInvalid,
            "find the container engine",
            "PATH",
            format!("the search path cannot be joined: {error}"),
        )
    })?;
    Ok((dirs, joined))
}

/// A `docker` command found on the search path and run with it.
pub fn docker_command() -> IdentityResult<Command> {
    let (dirs, joined) = search_path()?;
    let mut command = Command::new(find(DOCKER, &dirs));
    command.env("PATH", joined);
    Ok(command)
}

/// `program` as a command run with the search path, so a `docker` named
/// outright still finds the engine's helpers.
pub fn command_for(program: &Path) -> IdentityResult<Command> {
    let (_, joined) = search_path()?;
    let mut command = Command::new(program);
    command.env("PATH", joined);
    Ok(command)
}

#[cfg(test)]
#[path = "engine_path_tests.rs"]
mod tests;
