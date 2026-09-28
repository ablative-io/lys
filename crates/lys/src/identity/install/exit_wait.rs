//! The exit of a service the install started, waited on as an event.
//!
//! A service is started holding an exclusive `flock` on its exit lock, the
//! `.exit` file beside its pid file. The lock is taken on the open file
//! description the service is then given as its standard input, and the
//! starter closes its own copy, so the service is the lock's only holder.
//! The kernel releases a `flock` when the last descriptor of its
//! description closes, which for the service is its exit. Waiting for the
//! exit is asking for the same lock and blocking until the kernel grants
//! it: nothing is asked twice and no clock is read. A lock granted at once
//! is a service already gone.
//!
//! The platforms' own notifications of another process's exit (`kqueue`'s
//! `EVFILT_PROC`, a pidfd) are reachable from this workspace only through
//! `unsafe` code, which it denies, or through an interface this crate does
//! not carry. The lock is the same event on every Unix: the kernel's own
//! release of what the exiting process held.
//!
//! Invariants: a service never closes its standard input while it runs (one
//! that did would read as exited), and a process that inherits that input
//! keeps the lock held until it too has exited. A pid file with no exit lock
//! beside it names a process this watch cannot see; that is refused by name
//! with its pid, never waited on some other way.

use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};

#[cfg(unix)]
use rustix::fs::FlockOperation;

use super::super::error::{ErrorKind, IdentityError, IdentityResult};

fn refuse(action: &'static str, detail: impl Into<String>, path: &Path) -> IdentityError {
    IdentityError::new(ErrorKind::Unready, action, "service", detail).at(path)
}

/// The exit lock beside `pid_file`.
pub fn lock_path(pid_file: &Path) -> PathBuf {
    pid_file.with_extension("exit")
}

/// Takes the exit lock for a service about to be started under `pid_file`.
/// The file returned is handed to the service as its standard input, which
/// carries the lock with it. A lock still held by an earlier process is
/// waited on until that process has exited, so two never run at once.
pub fn hold(pid_file: &Path) -> IdentityResult<File> {
    let path = lock_path(pid_file);
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&path)
        .map_err(|error| refuse("take exit lock", error.to_string(), &path))?;
    let watch = ExitWatch { lock: file, path };
    watch.wait()?;
    Ok(watch.lock)
}

/// A watch on the exit of the service a pid file names.
#[derive(Debug)]
pub struct ExitWatch {
    lock: File,
    path: PathBuf,
}

impl ExitWatch {
    /// Opens the watch on the service `pid_file` names. A pid file without
    /// an exit lock beside it is refused, naming the pid, since no exit of
    /// that process can be seen.
    pub fn open(pid_file: &Path) -> IdentityResult<Self> {
        let path = lock_path(pid_file);
        match File::open(&path) {
            Ok(lock) => Ok(Self { lock, path }),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let pid = std::fs::read_to_string(pid_file)
                    .map(|text| text.trim().to_string())
                    .map_err(|error| refuse("watch for exit", error.to_string(), pid_file))?;
                let detail = format!("process {pid} has no exit lock; its exit cannot be seen");
                Err(refuse("watch for exit", detail, &path))
            }
            Err(error) => Err(refuse("watch for exit", error.to_string(), &path)),
        }
    }
}

#[cfg(unix)]
impl ExitWatch {
    /// Whether the service has exited, answered at once without waiting.
    pub fn exited(&self) -> IdentityResult<bool> {
        self.lock(FlockOperation::NonBlockingLockExclusive)
    }

    /// Blocks until the service has exited: the kernel grants the lock the
    /// moment the service's last descriptor of it closes.
    pub fn wait(&self) -> IdentityResult<()> {
        self.lock(FlockOperation::LockExclusive).map(drop)
    }

    fn lock(&self, operation: FlockOperation) -> IdentityResult<bool> {
        use rustix::io::Errno;
        loop {
            match rustix::fs::flock(&self.lock, operation) {
                Ok(()) => return Ok(true),
                Err(Errno::AGAIN) => return Ok(false),
                Err(Errno::INTR) => {}
                Err(error) => {
                    return Err(refuse("watch for exit", error.to_string(), &self.path));
                }
            }
        }
    }
}

#[cfg(not(unix))]
impl ExitWatch {
    /// Whether the service has exited; an exit lock needs a Unix host.
    pub fn exited(&self) -> IdentityResult<bool> {
        Err(refuse("watch for exit", "an exit lock needs a Unix host", &self.path))
    }

    /// Blocks until the service has exited; an exit lock needs a Unix host.
    pub fn wait(&self) -> IdentityResult<()> {
        self.exited().map(drop)
    }
}

#[cfg(test)]
#[path = "exit_wait_tests.rs"]
mod tests;
