//! The runner's Unix socket.
//!
//! The runner listens on one Unix socket, made readable and writable by its
//! owner alone (mode 0600), and on no network address. A connection carries
//! a fresh greeting, one request and one reply at each request boundary. Each request is verified
//! against the server's key, this runner's id and the challenge its
//! connection was given before anything in it is acted on; a request that
//! fails any of them is refused by name and does nothing.
//!
//! While an act waits (a read that follows, a wait for a pattern, an end),
//! readiness watches the connection: when the caller closes it, the act
//! stops waiting. Nothing ends a wait on a clock.

use std::io;
use std::net::Shutdown;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;

use crate::admitted::Admitted;
use crate::error::RunnerError;
use crate::protocol::{Act, Answer, Greeting, Output, reply_line, verify_parsed, verify_request};
use crate::scrollback::whole_text;
use crate::session::Sessions;

mod acts;
use acts::*;

mod connection;
use connection::*;

#[cfg(test)]
#[path = "../tests/socket_bounds/cases.rs"]
mod bounds_tests;

#[cfg(test)]
type ShutdownProbe = Box<dyn FnOnce() -> Result<(), RunnerError> + Send>;

/// What a runner is started with.
#[derive(Debug, Clone)]
pub struct Options {
    /// The socket it listens on.
    pub socket: PathBuf,
    /// The directory its record of sessions is kept in.
    pub state: PathBuf,
    /// The server's public key, the only key whose requests it answers.
    pub server_key: [u8; 32],
    /// Each session's scrollback, in bytes.
    pub scrollback: usize,
}

/// A runner, its sessions opened and its socket bound.
pub struct Runner {
    sessions: Arc<Sessions>,
    listener: UnixListener,
    socket: PathBuf,
    server: [u8; 32],
    #[cfg(test)]
    accept_error: Option<(usize, io::Error)>,
}

/// A runner serving on its own thread.
pub struct Serving {
    sessions: Arc<Sessions>,
    socket: PathBuf,
    stop: tokio::sync::watch::Sender<bool>,
    thread: JoinHandle<()>,
    #[cfg(test)]
    shutdown_probe: Option<ShutdownProbe>,
}

fn socket_failed(what: impl std::fmt::Display) -> RunnerError {
    RunnerError::Socket {
        reason: what.to_string(),
    }
}

/// Refuse `runner_already_running` when a runner answers on `path`.
fn refuse_live(path: &Path) -> Result<(), RunnerError> {
    if UnixStream::connect(path).is_ok() {
        return Err(RunnerError::AlreadyRunning {
            socket: path.display().to_string(),
        });
    }
    Ok(())
}

/// Bind `path`, owner-only. A socket file no runner answers on is a runner
/// that ended without removing it, and is replaced; one a runner answers on
/// is refused `runner_already_running`.
fn bind(path: &Path) -> Result<UnixListener, RunnerError> {
    if path.exists() {
        refuse_live(path)?;
        std::fs::remove_file(path)
            .map_err(|error| socket_failed(format!("removing {}: {error}", path.display())))?;
    }
    let listener = UnixListener::bind(path)
        .map_err(|error| socket_failed(format!("binding {}: {error}", path.display())))?;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
        .map_err(|error| socket_failed(format!("closing {}: {error}", path.display())))?;
    Ok(listener)
}

impl Runner {
    /// Open the sessions `options` records and bind its socket.
    pub fn open(options: &Options) -> Result<Self, RunnerError> {
        refuse_live(&options.socket)?;
        let sessions = Sessions::open(&options.state, options.scrollback)?;
        let listener = bind(&options.socket)?;
        Ok(Self {
            sessions,
            listener,
            socket: options.socket.clone(),
            server: options.server_key,
            #[cfg(test)]
            accept_error: None,
        })
    }

    /// The sessions it holds.
    pub fn sessions(&self) -> &Arc<Sessions> {
        &self.sessions
    }

    /// Serve every connection until the process is asked to stop, by
    /// SIGTERM, SIGINT or SIGHUP; then end every session, answering once
    /// each exit is seen, and remove the socket. `ready` is called once the
    /// signals are held and the runner serves, so a stop asked for after it
    /// is never missed.
    pub fn serve_until_stopped(self, ready: impl FnOnce()) -> Result<(), RunnerError> {
        use tokio::signal::unix::{SignalKind, signal};
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|error| socket_failed(format!("the signal watch could not begin: {error}")))?;
        let asked = runtime.block_on(async {
            let held = |kind: SignalKind| {
                signal(kind).map_err(|error| {
                    socket_failed(format!("a stop signal could not be held: {error}"))
                })
            };
            let (mut term, mut interrupt, mut hangup) = (
                held(SignalKind::terminate())?,
                held(SignalKind::interrupt())?,
                held(SignalKind::hangup())?,
            );
            let serving = self.spawn();
            ready();
            let name = tokio::select! {
                _ = term.recv() => "SIGTERM",
                _ = interrupt.recv() => "SIGINT",
                _ = hangup.recv() => "SIGHUP",
            };
            Ok::<_, RunnerError>((serving, name))
        });
        let (serving, name) = asked?;
        crate::error::said(&format!("asked to stop by {name}: ending every session"));
        serving.stop()
    }

    /// Serve on a thread of its own until [`Serving::stop`].
    pub fn spawn(self) -> Serving {
        let (stop, stopped) = tokio::sync::watch::channel(false);
        let sessions = Arc::clone(&self.sessions);
        let socket = self.socket.clone();
        let thread = std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .max_blocking_threads(DISPATCH_MAX)
                .build();
            match runtime {
                Ok(runtime) => {
                    if let Err(error) = runtime.block_on(self.serve_until(stopped)) {
                        crate::error::said(&format!("runner_socket_unavailable: {error}"));
                    }
                }
                Err(error) => {
                    crate::error::said(&format!("runner_socket_runtime_failed: {error}"));
                }
            }
        });
        Serving {
            sessions,
            socket,
            stop,
            thread,
            #[cfg(test)]
            shutdown_probe: None,
        }
    }

    async fn serve_until(
        self,
        mut stop: tokio::sync::watch::Receiver<bool>,
    ) -> Result<(), RunnerError> {
        self.listener.set_nonblocking(true).map_err(socket_failed)?;
        let listener = tokio::net::UnixListener::from_std(self.listener).map_err(socket_failed)?;
        #[cfg(test)]
        let mut accept_error = self.accept_error;
        let slots = Arc::new(DispatchSlots::new());
        let open = Arc::new(tokio::sync::Semaphore::new(CONNECTION_MAX));
        let mut connections = tokio::task::JoinSet::new();
        while !*stop.borrow() {
            tokio::select! {
                changed = stop.changed() => {
                    changed.map_err(socket_failed)?;
                }
                incoming = listener.accept() => {
                    #[cfg(test)]
                    let incoming = if let Some((remaining, _)) = accept_error.as_mut()
                        && *remaining > 0
                    {
                        *remaining -= 1;
                        incoming
                    } else if let Some((_, error)) = accept_error.take() {
                        drop(incoming);
                        Err(error)
                    } else {
                        incoming
                    };
                    let (stream, _) = match incoming {
                        Ok(accepted) => accepted,
                        Err(error) => {
                            crate::error::said(&format!("runner_socket_accept_failed: {error}"));
                            if matches!(error.raw_os_error(), Some(nix::libc::EMFILE | nix::libc::ENFILE)) {
                                tokio::select! {
                                    changed = stop.changed() => { changed.map_err(socket_failed)?; }
                                    completed = connections.join_next(), if !connections.is_empty() => {
                                        connection_result(completed);
                                    }
                                }
                            }
                            continue;
                        }
                    };
                    match Arc::clone(&open).try_acquire_owned() {
                        Ok(permit) => {
                            let sessions = Arc::clone(&self.sessions);
                            let slots = Arc::clone(&slots);
                            let stopped = stop.clone();
                            let server = self.server;
                            connections.spawn(async move {
                                let result = connection(&sessions, server, stream, slots, stopped).await;
                                drop(permit);
                                result
                            });
                        }
                        Err(error) => {
                            let greeting = Greeting::fresh(self.sessions.runner());
                            let refused = Answer::refusal(&RunnerError::refused("runner_connections_full", error.to_string()));
                            let text = format!("{}\n{}\n", greeting.line(), reply_line(refused));
                            match stream.try_write(text.as_bytes()) {
                                Ok(written) if written == text.len() => {},
                                Ok(written) => crate::error::said(&format!("runner_capacity_answer_incomplete: wrote {written} of {} bytes", text.len())),
                                Err(error) => crate::error::said(&format!("runner_capacity_answer_failed: {error}")),
                            }
                        }
                    }
                }
                completed = connections.join_next(), if !connections.is_empty() => {
                    connection_result(completed);
                }
            }
        }
        drop(listener);
        while let Some(completed) = connections.join_next().await {
            connection_result(Some(completed));
        }
        Ok(())
    }
}

const CONNECTION_MAX: usize = 64;
const DISPATCH_MAX: usize = 32;
const LINE_MAX: usize = 1_048_576;

fn connection_result(result: Option<Result<Result<(), RunnerError>, tokio::task::JoinError>>) {
    match result {
        Some(Ok(Err(error))) => crate::error::said(&format!("runner_connection_closed: {error}")),
        Some(Err(error)) => crate::error::said(&format!("runner_connection_failed: {error}")),
        Some(Ok(Ok(()))) | None => {}
    }
}

impl Serving {
    /// The sessions it holds.
    pub fn sessions(&self) -> &Arc<Sessions> {
        &self.sessions
    }

    /// Stop uses its held signal channel even after the listener path disappears.
    pub fn stop(self) -> Result<(), RunnerError> {
        let stopped = self.sessions.stop_all();
        let sent = self.stop.send(true);
        #[cfg(test)]
        if let Some(probe) = self.shutdown_probe {
            probe()?;
        }
        self.thread.join().map_err(|panic| {
            socket_failed(format!("the serving thread ended abnormally: {panic:?}"))
        })?;
        sent.map_err(socket_failed)?;
        stopped?;
        self.sessions.writer.barrier()?;
        match std::fs::remove_file(&self.socket) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(socket_failed(format!(
                "removing {}: {error}",
                self.socket.display()
            ))),
        }
    }
}
