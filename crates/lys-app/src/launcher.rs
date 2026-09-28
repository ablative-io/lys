//! Opening the app: one app's work at a time, and a browser always shown
//! the page of the one running.
//!
//! The app a person opens is short-lived: it starts the app's work as its
//! own detached process (`lys-app --serve`), reads the loopback port that
//! process serves its page on, opens the default browser to that page, and
//! exits, so opening the app again always runs again. The work claims the
//! install's one-instance lock; when another work already holds it, the
//! new one names that one's port and exits, so a second opening shows the
//! first one's page and never starts a second install.
//!
//! The claim is two locks, each a `flock` the kernel releases when its
//! holder exits: `app-start.lock`, held only while a work claims and
//! publishes its port, and `app.lock`, held for as long as the work runs.
//! A work that finds `app.lock` held has waited on `app-start.lock` first,
//! so the holder's port is already written when it is read. Every wait is
//! the kernel's grant of a lock or the arrival of a line on a pipe, never a
//! clock.
//!
//! Invariants: at most one work holds `app.lock`; a port is read only from
//! a work that has published it; and a work that exits before naming its
//! port is refused by name, pointing to its log.

use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use lys_install::install::layout::Layout;
use lys_install::private_files;

use crate::refusal::Refusal;
use crate::server::PAGE;

/// The argument the app's work runs with.
pub const SERVE: &str = "--serve";

/// The app's log, in the install's logs.
pub fn log_path(layout: &Layout) -> PathBuf {
    layout.logs_dir().join("lys-app.log")
}

fn cannot(what: &str, path: &Path, error: impl std::fmt::Display) -> Refusal {
    Refusal::app(
        "app_start_failed",
        "Lys could not start.",
        format!("{what} {}: {error}", path.display()),
    )
}

/// Opens `path` for appending, making its folder owner-only first.
pub fn append(path: &Path) -> Result<File, Refusal> {
    if let Some(folder) = path.parent() {
        private_files::ensure_dir(folder).map_err(|error| cannot("make", folder, error))?;
    }
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|error| cannot("open", path, error))
}

fn lock_file(path: &Path) -> Result<File, Refusal> {
    OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
        .map_err(|error| cannot("open", path, error))
}

/// Takes `operation` on `file`: `true` when granted, `false` when another
/// holds it and the operation does not wait.
#[cfg(unix)]
fn flock(file: &File, operation: rustix::fs::FlockOperation, path: &Path) -> Result<bool, Refusal> {
    use rustix::io::Errno;
    loop {
        match rustix::fs::flock(file, operation) {
            Ok(()) => return Ok(true),
            Err(Errno::AGAIN) => return Ok(false),
            Err(Errno::INTR) => {}
            Err(error) => return Err(cannot("lock", path, error)),
        }
    }
}

/// The work that holds the install's one-instance lock.
#[derive(Debug)]
pub struct Instance {
    /// `app.lock`, held for as long as the work runs and released by its
    /// exit; it holds the port the work's page is served on.
    running: File,
    /// `app-start.lock`, held until the port is written.
    starting: Option<File>,
    path: PathBuf,
}

impl Instance {
    /// Writes the port the page is served on into the lock the work holds,
    /// and lets the next opening in.
    pub fn publish(&mut self, port: u16) -> Result<(), Refusal> {
        self.running
            .set_len(0)
            .and_then(|()| self.running.write_all(port.to_string().as_bytes()))
            .and_then(|()| self.running.sync_all())
            .map_err(|error| cannot("write", &self.path, error))?;
        drop(self.starting.take());
        Ok(())
    }
}

/// Who serves the page.
#[derive(Debug)]
pub enum Claim {
    /// This work: it runs, serving its page.
    Ours(Instance),
    /// Another work already running, serving its page on this port.
    Theirs(u16),
}

/// Claims the install's one-instance lock under `layout`.
#[cfg(unix)]
pub fn claim(layout: &Layout) -> Result<Claim, Refusal> {
    use rustix::fs::FlockOperation;

    let dir = layout.install_dir();
    private_files::ensure_dir(&dir).map_err(|error| cannot("make", &dir, error))?;
    let start_path = dir.join("app-start.lock");
    let starting = lock_file(&start_path)?;
    flock(&starting, FlockOperation::LockExclusive, &start_path)?;
    let path = dir.join("app.lock");
    let running = lock_file(&path)?;
    if flock(&running, FlockOperation::NonBlockingLockExclusive, &path)? {
        return Ok(Claim::Ours(Instance {
            running,
            starting: Some(starting),
            path,
        }));
    }
    let text = std::fs::read_to_string(&path).map_err(|error| cannot("read", &path, error))?;
    let port = text
        .trim()
        .parse()
        .map_err(|error| cannot("read a port from", &path, error))?;
    Ok(Claim::Theirs(port))
}

/// The one-instance lock is a `flock`, which a host without Unix has none
/// of.
#[cfg(not(unix))]
pub fn claim(layout: &Layout) -> Result<Claim, Refusal> {
    Err(cannot(
        "lock",
        &layout.install_dir(),
        "the one-instance lock needs a Unix host",
    ))
}

/// Reads the port a starting work names on its first line of `output`.
pub fn read_port(output: &mut dyn BufRead) -> Option<u16> {
    let mut line = String::new();
    output.read_line(&mut line).ok()?;
    line.trim().parse().ok()
}

/// The page's address on `port`.
pub fn page_url(port: u16) -> String {
    format!("http://127.0.0.1:{port}{PAGE}")
}

/// Opens `url` in the person's default browser.
pub fn open_browser(url: &str) -> Result<(), Refusal> {
    let status = Command::new("/usr/bin/open")
        .arg(url)
        .stdin(Stdio::null())
        .status()
        .map_err(|error| cannot("open the browser at", Path::new(url), error))?;
    if !status.success() {
        return Err(cannot("open the browser at", Path::new(url), status));
    }
    Ok(())
}

/// The app as a person opens it: starts the work, reads its port, opens the
/// browser to its page, and exits.
pub fn open(root: Option<&Path>) -> Result<(), Refusal> {
    let layout = crate::flow::layout(root)?;
    let log = log_path(&layout);
    let own =
        std::env::current_exe().map_err(|error| cannot("find", Path::new("lys-app"), error))?;
    let mut command = Command::new(&own);
    command.arg(SERVE);
    if let Some(root) = root {
        command.arg("--root").arg(root);
    }
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(append(&log)?);
    detach(&mut command);
    let mut work = command
        .spawn()
        .map_err(|error| cannot("start", &own, error))?;
    let port = work
        .stdout
        .take()
        .and_then(|output| read_port(&mut BufReader::new(output)));
    let Some(port) = port else {
        let status = work
            .wait()
            .map_err(|error| cannot("wait for", &own, error))?;
        let said = format!(
            "it exited {status} before naming its page; its log is {}",
            log.display()
        );
        return Err(cannot("start", &own, said));
    };
    open_browser(&page_url(port))
}

#[cfg(unix)]
fn detach(command: &mut Command) {
    use std::os::unix::process::CommandExt;
    command.process_group(0);
}

#[cfg(not(unix))]
fn detach(_command: &mut Command) {}

#[cfg(test)]
#[path = "launcher_tests.rs"]
mod tests;
