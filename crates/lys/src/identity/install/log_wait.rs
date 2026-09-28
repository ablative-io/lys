//! Waiting on a detached service's log: a check run once when the wait
//! begins and once on each change to the log, and the reading of what the
//! log gained since a process started.
//!
//! The change is the platform's own file-change notice (kqueue on macOS and
//! the BSDs, inotify on Linux), never a clock and never a question asked
//! again on a schedule. The process exiting first ends the wait, refused by
//! name with where its output is.
//!
//! Invariants: a [`LogCursor`] reads each byte of the log once, from the
//! offset its process opened the log at; a later change reads only what was
//! appended after the last read, never the log again from its first byte.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use super::super::error::{ErrorKind, IdentityError, IdentityResult};
use super::exit_wait::ExitWatch;

fn refuse(action: &'static str, resource: &str, detail: impl Into<String>) -> IdentityError {
    IdentityError::new(ErrorKind::Unready, action, resource, detail)
}

/// What a log has said since an offset, read forward only.
#[derive(Debug)]
pub struct LogCursor {
    path: PathBuf,
    offset: u64,
    tail: Vec<u8>,
    found: bool,
}

impl LogCursor {
    /// A cursor on `path` standing at `offset`, the length the log had when
    /// its process opened it.
    pub fn at(path: &Path, offset: u64) -> Self {
        Self {
            path: path.to_path_buf(),
            offset,
            tail: Vec::new(),
            found: false,
        }
    }

    /// Reads what the log gained since the last read and answers whether
    /// everything read since the offset holds `line`. Only the bytes that
    /// could begin a match across the next read are kept between reads.
    pub fn says(&mut self, line: &str) -> bool {
        if self.found {
            return true;
        }
        let needle = line.as_bytes();
        let Ok(mut file) = File::open(&self.path) else {
            return false;
        };
        if file.seek(SeekFrom::Start(self.offset)).is_err() {
            return false;
        }
        let mut gained = Vec::new();
        if file.read_to_end(&mut gained).is_err() {
            return false;
        }
        let Ok(offset) = file.stream_position() else {
            return false;
        };
        self.offset = offset;
        self.tail.extend_from_slice(&gained);
        self.found = needle.is_empty() || self.tail.windows(needle.len()).any(|at| at == needle);
        let keep = needle.len().saturating_sub(1);
        if self.tail.len() > keep {
            self.tail.drain(..self.tail.len() - keep);
        }
        self.found
    }
}

/// Runs `check` once now and once on each change to `log`, until it
/// passes; the process behind `pid_file` exiting first is refused as
/// `what`, naming the log. The log is watched before the first check, so a
/// change made after it is never missed.
#[cfg(unix)]
pub fn wait_until(
    what: &str,
    log: &Path,
    pid_file: &Path,
    check: &mut dyn FnMut() -> bool,
) -> IdentityResult<()> {
    use mio::unix::SourceFd;
    use mio::{Events, Interest, Poll, Token};
    use std::os::fd::AsRawFd;
    use std::os::unix::net::UnixStream;

    use log_watch::LogWatch;

    const CHANGED: Token = Token(0);
    const EXITED: Token = Token(1);
    let watching = |error: std::io::Error| {
        refuse(
            "wait for service",
            what,
            format!("watching {}: {error}", log.display()),
        )
    };
    let mut poll = Poll::new().map_err(watching)?;
    let registry = poll.registry();
    let mut changes = LogWatch::arm(log, registry, CHANGED).map_err(watching)?;
    if check() {
        return Ok(());
    }
    let exit = ExitWatch::open(pid_file)?;
    let (signal, notice) = UnixStream::pair().map_err(watching)?;
    let signal_fd = signal.as_raw_fd();
    registry
        .register(&mut SourceFd(&signal_fd), EXITED, Interest::READABLE)
        .map_err(watching)?;
    let exited = std::thread::spawn(move || {
        let outcome = exit.wait();
        drop(notice);
        outcome
    });
    let mut events = Events::with_capacity(4);
    loop {
        match poll.poll(&mut events, None) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(watching(error)),
        }
        if events.iter().any(|event| event.token() == EXITED) {
            return match exited.join() {
                Ok(Ok(())) => Err(refuse(
                    "wait for service",
                    what,
                    format!("the process exited; its output is in {}", log.display()),
                )),
                Ok(Err(error)) => Err(error),
                Err(panic) => Err(refuse(
                    "wait for service",
                    what,
                    format!("the exit watch failed: {panic:?}"),
                )),
            };
        }
        changes.drain().map_err(watching)?;
        if check() {
            return Ok(());
        }
    }
}

/// Without Unix there is no event to wait on: `check` is asked once and a
/// service not ready then is refused as `what`, naming the log.
#[cfg(not(unix))]
pub fn wait_until(
    what: &str,
    log: &Path,
    pid_file: &Path,
    check: &mut dyn FnMut() -> bool,
) -> IdentityResult<()> {
    if check() {
        return Ok(());
    }
    Err(refuse(
        "wait for service",
        what,
        format!(
            "waiting on {} and {} needs a Unix host",
            log.display(),
            pid_file.display()
        ),
    ))
}

/// The notice of a change to a log file.
#[cfg(any(target_os = "linux", target_os = "android"))]
mod log_watch {
    use std::fs::File;
    use std::io::{ErrorKind, Read, Result};
    use std::os::fd::AsRawFd;
    use std::path::Path;

    use mio::unix::SourceFd;
    use mio::{Interest, Registry, Token};
    use rustix::fs::inotify::{self, CreateFlags, WatchFlags};

    /// An inotify watch on one log file, read without blocking.
    pub struct LogWatch(File);

    impl LogWatch {
        /// Watches `log` for writes, the notice reaching `registry` as `token`.
        pub fn arm(log: &Path, registry: &Registry, token: Token) -> Result<Self> {
            let notices = inotify::init(CreateFlags::NONBLOCK | CreateFlags::CLOEXEC)?;
            inotify::add_watch(&notices, log, WatchFlags::MODIFY)?;
            let fd = notices.as_raw_fd();
            let source = &mut SourceFd(&fd);
            registry.register(source, token, Interest::READABLE)?;
            Ok(Self(File::from(notices)))
        }

        /// Takes every notice waiting, so the next change is a new event.
        pub fn drain(&mut self) -> Result<()> {
            let mut notices = [0_u8; 4096];
            loop {
                match self.0.read(&mut notices) {
                    Ok(0) => return Ok(()),
                    Ok(_) => {}
                    Err(error) if error.kind() == ErrorKind::WouldBlock => return Ok(()),
                    Err(error) if error.kind() == ErrorKind::Interrupted => {}
                    Err(error) => return Err(error),
                }
            }
        }
    }
}

/// The notice of a change to a log file.
#[cfg(all(unix, not(any(target_os = "linux", target_os = "android"))))]
mod log_watch {
    use std::fs::File;
    use std::io::{Result, Seek, SeekFrom};
    use std::os::fd::AsRawFd;
    use std::path::Path;

    use mio::unix::SourceFd;
    use mio::{Interest, Registry, Token};

    /// The log file itself, registered for reading: kqueue reports it when
    /// the file grows past where this reader stands.
    pub struct LogWatch(File);

    impl LogWatch {
        /// Watches `log` for what is appended from now, the notice reaching
        /// `registry` as `token`.
        pub fn arm(log: &Path, registry: &Registry, token: Token) -> Result<Self> {
            let mut file = File::open(log)?;
            file.seek(SeekFrom::End(0))?;
            let fd = file.as_raw_fd();
            let source = &mut SourceFd(&fd);
            registry.register(source, token, Interest::READABLE)?;
            Ok(Self(file))
        }

        /// Stands at the end again, so the next append is a new event.
        pub fn drain(&mut self) -> Result<()> {
            self.0.seek(SeekFrom::End(0)).map(drop)
        }
    }
}

#[cfg(test)]
#[path = "log_wait_tests.rs"]
mod tests;
