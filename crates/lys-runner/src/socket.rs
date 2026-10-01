//! The runner's Unix socket.
//!
//! The runner listens on one Unix socket, made readable and writable by its
//! owner alone (mode 0600), and on no network address. A connection carries
//! one greeting, one request and one reply. Each request is verified
//! against the server's key, this runner's id and the challenge its
//! connection was given before anything in it is acted on; a request that
//! fails any of them is refused by name and does nothing.
//!
//! While an act waits (a read that follows, a wait for a pattern, an end),
//! a second thread reads the connection: when the caller closes it, the act
//! stops waiting. Nothing ends a wait on a clock.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::Shutdown;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;

use crate::admitted::Admitted;
use crate::error::RunnerError;
use crate::protocol::{Act, Answer, Greeting, Output, reply_line, verify_request};
use crate::scrollback::whole_text;
use crate::session::Sessions;

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
}

/// A runner serving on its own thread.
pub struct Serving {
    sessions: Arc<Sessions>,
    socket: PathBuf,
    stop: Arc<AtomicBool>,
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
        let stop = Arc::new(AtomicBool::new(false));
        let sessions = Arc::clone(&self.sessions);
        let socket = self.socket.clone();
        let stopped = Arc::clone(&stop);
        let thread = std::thread::spawn(move || self.serve_until(&stopped));
        Serving {
            sessions,
            socket,
            stop,
            thread,
            #[cfg(test)]
            shutdown_probe: None,
        }
    }

    fn serve_until(self, stop: &AtomicBool) {
        for incoming in self.listener.incoming() {
            if stop.load(Ordering::SeqCst) {
                break;
            }
            match incoming {
                Ok(stream) => {
                    let sessions = Arc::clone(&self.sessions);
                    let server = self.server;
                    std::thread::spawn(move || connection(&sessions, &server, &stream));
                }
                Err(error) => {
                    crate::error::said(&format!(
                        "runner_socket_unavailable: a connection was not accepted: {error}"
                    ));
                }
            }
        }
    }
}

impl Serving {
    /// The sessions it holds.
    pub fn sessions(&self) -> &Arc<Sessions> {
        &self.sessions
    }

    /// End every session, answering once each exit is seen, then stop
    /// serving and remove the socket.
    pub fn stop(self) -> Result<(), RunnerError> {
        self.sessions.stop_all();
        self.stop.store(true, Ordering::SeqCst);
        if let Err(error) = UnixStream::connect(&self.socket) {
            crate::error::said(&format!("the runner's socket was already closed: {error}"));
        }
        #[cfg(test)]
        if let Some(probe) = self.shutdown_probe {
            probe()?;
        }
        self.thread
            .join()
            .map_err(|_panicked| socket_failed("the serving thread ended abnormally"))?;
        std::fs::remove_file(&self.socket)
            .map_err(|error| socket_failed(format!("removing {}: {error}", self.socket.display())))
    }
}

/// Answer the one request `stream` carries, naming in the runner's log a
/// caller that left before its answer was written.
fn connection(sessions: &Arc<Sessions>, server: &[u8; 32], stream: &UnixStream) {
    if let Err(error) = answer_one(sessions, server, stream) {
        crate::error::said(&format!(
            "a request was not answered: the caller left: {error}"
        ));
    }
}

fn answer_one(
    sessions: &Arc<Sessions>,
    server: &[u8; 32],
    stream: &UnixStream,
) -> std::io::Result<()> {
    let greeting = Greeting::fresh(sessions.runner());
    let mut writer = stream;
    writer.write_all(greeting.line().as_bytes())?;
    writer.write_all(b"\n")?;
    writer.flush()?;
    let mut line = String::new();
    BufReader::new(stream.try_clone()?).read_line(&mut line)?;
    if line.is_empty() {
        return Ok(());
    }
    if !crate::peer::is_peer(&line)
        && matches!(
            verify_request(line.trim_end(), server, &greeting),
            Ok(Act::GrantChannel)
        )
    {
        writer.write_all(reply_line(Answer::GrantChannel).as_bytes())?;
        writer.write_all(b"\n")?;
        writer.flush()?;
        crate::refusals::serve(sessions, stream);
        return stream.shutdown(Shutdown::Both);
    }
    let left = Arc::new(AtomicBool::new(false));
    let mut watched = stream.try_clone()?;
    let (flag, woken) = (Arc::clone(&left), Arc::clone(sessions));
    std::thread::spawn(move || {
        let mut byte = [0_u8; 1];
        let seen = watched.read(&mut byte);
        if let Ok(read @ 1..) = seen {
            crate::error::said(&format!("a caller wrote {read} bytes past its one request"));
        }
        flag.store(true, Ordering::SeqCst);
        woken.wake();
    });
    let answer = if crate::peer::is_peer(&line) {
        crate::peer::answer(sessions, stream, line.trim_end(), &left)
    } else {
        dispatch(sessions, server, &greeting, &line, &left)
    };
    writer.write_all(reply_line(answer).as_bytes())?;
    writer.write_all(b"\n")?;
    writer.flush()?;
    stream.shutdown(Shutdown::Both)
}

/// The answer to request `line`, made on the connection given `greeting`.
pub fn dispatch(
    sessions: &Arc<Sessions>,
    server: &[u8; 32],
    greeting: &Greeting,
    line: &str,
    left: &AtomicBool,
) -> Answer {
    let act = match verify_request(line.trim_end(), server, greeting) {
        Ok(act) => act,
        Err(error) => return Answer::refusal(&error),
    };
    perform(sessions, act, left).unwrap_or_else(|error| Answer::refusal(&error))
}

fn perform(sessions: &Arc<Sessions>, act: Act, left: &AtomicBool) -> Result<Answer, RunnerError> {
    match act {
        Act::ReadBytes {
            session,
            cursor,
            follow,
        } => crate::terminal_bytes::read(sessions, &session, cursor, follow, left),
        Act::InputBytes { session, data } => sessions
            .write(&session, &data)
            .map(|()| Answer::Delivered { session }),
        Act::Start { launch } => {
            let session = launch.session.clone();
            let policy = launch
                .policy
                .clone()
                .map(|admitted| Admitted::verified(*admitted))
                .transpose()?;
            let (pid, started_at) = sessions.begin(*launch, policy, None)?;
            Ok(Answer::Started {
                session,
                pid,
                started_at,
            })
        }
        Act::Input {
            session,
            text,
            enter,
        } => sessions
            .input(&session, &text, enter)
            .map(|()| Answer::Delivered { session }),
        Act::Keys { session, keys } => sessions
            .keys(&session, &keys)
            .map(|()| Answer::Delivered { session }),
        Act::Read {
            session,
            cursor,
            lines,
            bytes,
            follow,
        } => read(sessions, &session, (cursor, lines, bytes), follow, left),
        Act::Wait {
            session,
            cursor,
            pattern,
            regex,
        } => wait(sessions, &session, cursor, &pattern, regex, left),
        Act::Resize {
            session,
            columns,
            rows,
        } => sessions
            .resize(&session, columns, rows)
            .map(|()| Answer::Delivered { session }),
        Act::End { session } => {
            let ended = sessions.end(&session, left)?;
            Ok(Answer::Ended { session, ended })
        }
        Act::Status { session } => Ok(Answer::Status {
            status: sessions.status(session.as_deref())?,
        }),
        Act::Operate { operation } => sessions
            .operate(operation)
            .map(|outcome| Answer::Operation { outcome }),
        Act::Feed { cursor, follow } => {
            if follow {
                sessions.until_any(left, |table| {
                    table
                        .feed
                        .after(cursor.as_deref())
                        .map_or(Some(()), |more| more.then_some(()))
                })?;
            }
            let reader = sessions.lock().feed.reader();
            let page = reader.page(cursor.as_deref())?;
            Ok(Answer::Feed { page })
        }
        Act::GrantChannel => Err(RunnerError::refused(
            "grant_channel_unheld",
            "a grant channel is held only as the whole of its connection",
        )),
        Act::Outcome { operation } => sessions
            .outcome(&operation)
            .map(|outcome| Answer::Operation { outcome }),
    }
}

/// Where a read begins: a cursor, the last lines, or the last bytes.
type Start = (Option<u64>, Option<u32>, Option<u64>);

fn read(
    sessions: &Sessions,
    id: &str,
    (cursor, lines, bytes): Start,
    follow: bool,
    left: &AtomicBool,
) -> Result<Answer, RunnerError> {
    sessions.until(id, left, |session, id| {
        let scrollback = session.scrollback();
        let from = match (cursor, lines, bytes) {
            (Some(cursor), _, _) => cursor,
            (None, Some(lines), _) => scrollback.last_lines(lines),
            (None, None, Some(bytes)) => scrollback.last_bytes(bytes),
            (None, None, None) => scrollback.oldest(),
        };
        if from > scrollback.end() {
            return Some(Err(RunnerError::refused(
                "cursor_ahead",
                format!(
                    "cursor {from} is past the end of the output, {}",
                    scrollback.end()
                ),
            )));
        }
        let kept = match scrollback.from(from) {
            Ok(kept) => kept,
            Err(expired) => return Some(Err(expired)),
        };
        let (text, given) = whole_text(&kept);
        let ended = session.ended();
        if follow && given == 0 && ended.is_none() {
            return None;
        }
        Some(Ok(Answer::Output {
            output: Output {
                session: id.to_owned(),
                from,
                cursor: from + given as u64,
                oldest: scrollback.oldest(),
                text,
                ended,
            },
        }))
    })
}

/// The offset in `bytes` that offset `at` of their lossy text stands for:
/// each replacement character stands for the whole invalid sequence it
/// replaced, so a cursor after a match counts the bytes the session gave.
fn byte_at(bytes: &[u8], at: usize) -> usize {
    let (mut text, mut byte) = (0, 0);
    for chunk in bytes.utf8_chunks() {
        let valid = chunk.valid().len();
        if at <= text + valid {
            return byte + (at - text);
        }
        text += valid;
        byte += valid;
        if !chunk.invalid().is_empty() {
            text += char::REPLACEMENT_CHARACTER.len_utf8();
            byte += chunk.invalid().len();
            if at <= text {
                return byte;
            }
        }
    }
    byte
}

/// Where `needle` first begins in `haystack`.
fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn wait(
    sessions: &Sessions,
    id: &str,
    cursor: Option<u64>,
    pattern: &str,
    regex: bool,
    left: &AtomicBool,
) -> Result<Answer, RunnerError> {
    if pattern.is_empty() {
        return Err(RunnerError::refused(
            "pattern_invalid",
            "the pattern is empty",
        ));
    }
    let expression = if regex {
        Some(
            regex_lite::Regex::new(pattern)
                .map_err(|error| RunnerError::refused("pattern_invalid", error.to_string()))?,
        )
    } else {
        None
    };
    let mut start = cursor;
    sessions.until(id, left, |session, id| {
        let scrollback = session.scrollback();
        let from = *start.get_or_insert(scrollback.end());
        let kept = match scrollback.from(from.min(scrollback.end())) {
            Ok(kept) => kept,
            Err(expired) => return Some(Err(expired)),
        };
        let found = match &expression {
            Some(expression) => {
                let text = String::from_utf8_lossy(&kept);
                expression
                    .find(&text)
                    .map(|matched| (matched.as_str().to_owned(), byte_at(&kept, matched.end())))
            }
            None => {
                find(&kept, pattern.as_bytes()).map(|at| (pattern.to_owned(), at + pattern.len()))
            }
        };
        if let Some((matched, end)) = found {
            return Some(Ok(Answer::Matched {
                session: id.to_owned(),
                matched,
                cursor: from.min(scrollback.end()) + end as u64,
            }));
        }
        session.ended().map(|ended| {
            Err(RunnerError::refused(
                "session_ended",
                format!(
                    "session {id} ended ({:?}) before the pattern appeared",
                    ended.how
                ),
            ))
        })
    })
}
