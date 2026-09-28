//! The wait for the container engine to answer, on the kernel's notice and
//! the engine's own answer, never a clock or a question asked again on a
//! schedule.
//!
//! An engine can be in three states when Lys asks: nothing listens on its
//! socket; something listens but the engine behind it is not ready; or it
//! answers. Docker Desktop passes through the second on every start: it
//! makes its socket before the engine inside it answers, so a wait that
//! wakes only on the socket being made would miss the moment it is ready.
//!
//! So each round of the wait:
//!
//! 1. arms a kqueue vnode watch (an entry made, removed or renamed) on each
//!    socket's folder (or, before it exists, its nearest folder that does)
//!    and on each of the engine's own state folders that exists;
//! 2. then asks every socket for its `/_ping` and *holds* each request a
//!    listener accepted, registering the connection for kqueue's read
//!    notice, so the engine's answer, when it comes, wakes the wait;
//! 3. then sleeps in the kernel until one of those notices arrives.
//!
//! A held answer of `200` ends the wait. Any other answer, or a connection
//! closed without one, means the engine is there but not ready; that
//! socket is dropped from the round and asked again only on the next folder
//! notice, so an engine that answers "not yet" at once is never asked in a
//! loop. A folder notice starts a new round. The watch is armed before the
//! sockets are asked, so a socket made between the two is never missed.
//!
//! A socket that exists but refuses the connection is taken as not there:
//! it is left from an engine that stopped, or it is in the instant between
//! the engine's bind and its listen, which makes no notice of its own. The
//! next change to a watched folder asks it again; Docker Desktop makes its
//! other sockets in its state folder as it starts, which gives that change.
//!
//! Invariants: no wait here reads a clock or sleeps; every round is started
//! by a kernel notice; a watch that cannot be armed is refused by name.

use std::path::PathBuf;

use crate::refusal::Refusal;

/// What reading a held request's answer found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Heard {
    /// The engine answered `200`.
    Ready,
    /// The engine answered something else, or closed without an answer.
    NotYet,
    /// Not all of the answer's status line has arrived.
    More,
}

/// What an answer's first bytes say.
pub fn heard(answer: &[u8], closed: bool) -> Heard {
    let status = b"HTTP/1.1 200".len();
    if answer.len() >= status {
        let ok = answer.starts_with(b"HTTP/1.1 200") || answer.starts_with(b"HTTP/1.0 200");
        return if ok { Heard::Ready } else { Heard::NotYet };
    }
    if closed { Heard::NotYet } else { Heard::More }
}

/// A `/_ping` sent to an engine whose answer is read when it arrives.
#[cfg(unix)]
#[derive(Debug)]
pub struct Held {
    /// The socket the request was sent on.
    pub socket: PathBuf,
    stream: std::os::unix::net::UnixStream,
    answer: Vec<u8>,
}

#[cfg(unix)]
impl Held {
    /// Sends `/_ping` on `socket`, answering the held request, or `None`
    /// when nothing accepts a connection there.
    pub fn ask(socket: &std::path::Path) -> Option<Self> {
        use std::io::Write;
        let mut stream = std::os::unix::net::UnixStream::connect(socket).ok()?;
        stream
            .write_all(b"GET /_ping HTTP/1.0\r\nHost: docker\r\n\r\n")
            .ok()?;
        stream.set_nonblocking(true).ok()?;
        Some(Self {
            socket: socket.to_path_buf(),
            stream,
            answer: Vec::new(),
        })
    }

    /// The connection's descriptor, for the read notice.
    pub fn fd(&self) -> std::os::fd::RawFd {
        use std::os::fd::AsRawFd;
        self.stream.as_raw_fd()
    }

    /// Reads what has arrived of the answer.
    pub fn read(&mut self) -> Heard {
        use std::io::Read;
        let mut chunk = [0_u8; 64];
        loop {
            match self.stream.read(&mut chunk) {
                Ok(0) => return heard(&self.answer, true),
                Ok(read) => {
                    self.answer.extend_from_slice(&chunk[..read]);
                    let now = heard(&self.answer, false);
                    if now != Heard::More {
                        return now;
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    return heard(&self.answer, false);
                }
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
                Err(_) => return heard(&self.answer, true),
            }
        }
    }
}

/// Waits until one of `sockets` answers, watching their folders and the
/// engine's `state` folders, and answers the socket that does.
pub fn wait(sockets: &[PathBuf], state: &[PathBuf]) -> Result<PathBuf, Refusal> {
    wait_with(sockets, state, &mut |_| {})
}

fn refused(detail: String) -> Refusal {
    Refusal::app(
        "engine_watch_failed",
        "Lys could not watch for the container engine to start.",
        detail,
    )
}

/// The folders a round watches: each socket's watched folder and each
/// state folder that exists, each once.
pub fn round_folders(sockets: &[PathBuf], state: &[PathBuf]) -> Vec<PathBuf> {
    let mut folders = crate::engine::watched_folders(sockets);
    for folder in state.iter().filter(|folder| folder.is_dir()) {
        if !folders.contains(folder) {
            folders.push(folder.clone());
        }
    }
    folders
}

/// [`wait`], telling `round` how many requests each round holds once it is
/// armed and before it sleeps.
#[cfg(target_os = "macos")]
pub fn wait_with(
    sockets: &[PathBuf],
    state: &[PathBuf],
    round: &mut dyn FnMut(usize),
) -> Result<PathBuf, Refusal> {
    use kqueue::{EventData, EventFilter, FilterFlag, Ident, Watcher};

    loop {
        let mut watcher = Watcher::new().map_err(|error| refused(error.to_string()))?;
        let folders = round_folders(sockets, state);
        if folders.is_empty() {
            return Err(refused("no socket folder or ancestor exists".to_string()));
        }
        let changes = FilterFlag::NOTE_WRITE | FilterFlag::NOTE_DELETE | FilterFlag::NOTE_RENAME;
        for folder in &folders {
            watcher
                .add_filename(folder, EventFilter::EVFILT_VNODE, changes)
                .map_err(|error| refused(format!("{}: {error}", folder.display())))?;
        }
        watcher
            .watch()
            .map_err(|error| refused(error.to_string()))?;
        let mut held: Vec<Held> = sockets
            .iter()
            .filter_map(|socket| Held::ask(socket))
            .collect();
        for request in &held {
            watcher
                .add_fd(request.fd(), EventFilter::EVFILT_READ, FilterFlag::empty())
                .map_err(|error| refused(format!("{}: {error}", request.socket.display())))?;
        }
        watcher
            .watch()
            .map_err(|error| refused(error.to_string()))?;
        round(held.len());
        loop {
            let Some(event) = watcher.poll_forever(None) else {
                return Err(refused("the watch ended without a notice".to_string()));
            };
            if let EventData::Error(error) = &event.data {
                return Err(refused(error.to_string()));
            }
            let Ident::Fd(fd) = event.ident else {
                break;
            };
            let Some(at) = held.iter().position(|request| request.fd() == fd) else {
                continue;
            };
            match held[at].read() {
                Heard::Ready => return Ok(held.swap_remove(at).socket),
                Heard::More => {}
                Heard::NotYet => {
                    let dropped = held.swap_remove(at);
                    watcher
                        .remove_fd(dropped.fd(), EventFilter::EVFILT_READ)
                        .map_err(|error| refused(error.to_string()))?;
                }
            }
        }
    }
}

/// [`wait`] needs kqueue's notices, which only a Mac here has: anywhere
/// else a socket not answering now is refused by name.
#[cfg(not(target_os = "macos"))]
pub fn wait_with(
    sockets: &[PathBuf],
    state: &[PathBuf],
    round: &mut dyn FnMut(usize),
) -> Result<PathBuf, Refusal> {
    round(0);
    crate::engine::answering(sockets).ok_or_else(|| {
        Refusal::app(
            "engine_watch_needs_macos",
            "Lys can wait for the container engine only on a Mac.",
            format!(
                "{} sockets asked once; {} state folders not watched",
                sockets.len(),
                state.len()
            ),
        )
    })
}

#[cfg(test)]
#[path = "engine_wait_tests.rs"]
mod tests;
