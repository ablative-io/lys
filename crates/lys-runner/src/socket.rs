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
        let slots = Arc::new(DispatchSlots::new());
        let open = Arc::new(tokio::sync::Semaphore::new(CONNECTION_MAX));
        let mut connections = tokio::task::JoinSet::new();
        while !*stop.borrow() {
            tokio::select! {
                changed = stop.changed() => {
                    changed.map_err(socket_failed)?;
                }
                incoming = listener.accept() => {
                    let (stream, _) = incoming.map_err(socket_failed)?;
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
        self.sessions.stop_all();
        let sent = self.stop.send(true);
        #[cfg(test)]
        if let Some(probe) = self.shutdown_probe {
            probe()?;
        }
        self.thread.join().map_err(|panic| {
            socket_failed(format!("the serving thread ended abnormally: {panic:?}"))
        })?;
        sent.map_err(socket_failed)?;
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

async fn write_line(stream: &tokio::net::UnixStream, line: String) -> Result<(), RunnerError> {
    let bytes = line.into_bytes();
    let mut offset = 0;
    while offset < bytes.len() {
        stream.writable().await.map_err(socket_failed)?;
        match stream.try_write(&bytes[offset..]) {
            Ok(0) => return Err(socket_failed("the connection closed during its answer")),
            Ok(written) => offset += written,
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {}
            Err(error) => return Err(socket_failed(error)),
        }
    }
    Ok(())
}

async fn read_line(
    stream: &tokio::net::UnixStream,
    pending: &mut Vec<u8>,
) -> Result<Option<String>, RunnerError> {
    let mut scanned = 0;
    loop {
        if let Some(end) = pending[scanned..].iter().position(|byte| *byte == b'\n') {
            let end = scanned + end;
            if end >= LINE_MAX {
                return Err(RunnerError::refused(
                    "request_too_large",
                    "the request exceeds 1048576 bytes",
                ));
            }
            let remainder = pending.split_off(end + 1);
            let line = std::mem::replace(pending, remainder);
            return String::from_utf8(line)
                .map(Some)
                .map_err(|error| RunnerError::refused("request_malformed", error.to_string()));
        }
        if pending.len() > LINE_MAX {
            return Err(RunnerError::refused(
                "request_too_large",
                "the request exceeds 1048576 bytes",
            ));
        }
        scanned = pending.len();
        stream.readable().await.map_err(socket_failed)?;
        let mut bytes = [0; 8192];
        let room = (LINE_MAX + 1 - pending.len()).min(bytes.len());
        match stream.try_read(&mut bytes[..room]) {
            Ok(0) if pending.is_empty() => return Ok(None),
            Ok(0) => {
                return Err(RunnerError::refused(
                    "request_unterminated",
                    "the connection ended before its request newline",
                ));
            }
            Ok(read) => pending.extend_from_slice(&bytes[..read]),
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {}
            Err(error) => return Err(socket_failed(error)),
        }
    }
}

async fn departed(stream: &tokio::net::UnixStream) -> Result<(), RunnerError> {
    let mut byte = [0];
    loop {
        stream.readable().await.map_err(socket_failed)?;
        match stream.try_read(&mut byte) {
            Ok(0) => return Ok(()),
            Ok(_) => {
                return Err(RunnerError::refused(
                    "request_pipelined",
                    "another request arrived before the answer",
                ));
            }
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {}
            Err(error) => return Err(socket_failed(error)),
        }
    }
}

enum Command {
    Peer(String),
    Server(Act),
}

impl Command {
    fn output_session(&self) -> Option<String> {
        match self {
            Self::Server(
                Act::Read { session, .. }
                | Act::ReadBytes { session, .. }
                | Act::Wait { session, .. },
            ) => Some(session.clone()),
            _ => None,
        }
    }

    fn control(&self) -> bool {
        matches!(self, Self::Server(Act::Status { .. }))
            || matches!(self,
            Self::Server(Act::Operate { operation }) if operation.request == crate::operations::OperationRequest::Stop)
    }
}

struct DispatchSlots {
    total: Arc<tokio::sync::Semaphore>,
    ordinary: Arc<tokio::sync::Semaphore>,
}

struct DispatchPermit {
    total: tokio::sync::OwnedSemaphorePermit,
    ordinary: Option<tokio::sync::OwnedSemaphorePermit>,
}

impl DispatchSlots {
    fn new() -> Self {
        Self {
            total: Arc::new(tokio::sync::Semaphore::new(DISPATCH_MAX)),
            ordinary: Arc::new(tokio::sync::Semaphore::new(DISPATCH_MAX - 1)),
        }
    }

    fn acquire(&self, control: bool) -> Result<DispatchPermit, tokio::sync::TryAcquireError> {
        let ordinary = if control {
            None
        } else {
            Some(Arc::clone(&self.ordinary).try_acquire_owned()?)
        };
        let total = Arc::clone(&self.total).try_acquire_owned()?;
        Ok(DispatchPermit { total, ordinary })
    }
}

impl DispatchPermit {
    fn finish(self) {
        drop(self.total);
        drop(self.ordinary);
    }
}

fn command(line: String, server: &[u8; 32], greeting: &Greeting) -> Result<Command, RunnerError> {
    if crate::peer::is_peer(&line) {
        Ok(Command::Peer(line))
    } else {
        verify_request(line.trim_end(), server, greeting).map(Command::Server)
    }
}

struct Cancellation {
    sessions: Arc<Sessions>,
    output_session: Option<String>,
    left: Arc<AtomicBool>,
    proof: Arc<UnixStream>,
    armed: bool,
}

impl Cancellation {
    fn cancel(&self) {
        let table = self.sessions.lock();
        self.left.store(true, Ordering::SeqCst);
        self.sessions.wake();
        drop(table);
        if let Some(session) = &self.output_session {
            if let Err(error) = self.sessions.wake_session(session) {
                crate::error::said(&format!(
                    "session {session}: cancellation_wake_failed: {error}"
                ));
            }
        }
        if let Err(error) = self.proof.shutdown(Shutdown::Both) {
            if error.kind() != io::ErrorKind::NotConnected {
                crate::error::said(&format!("runner_cancellation_failed: {error}"));
            }
        }
    }
}

impl Drop for Cancellation {
    fn drop(&mut self) {
        if self.armed {
            self.cancel();
        }
    }
}

async fn execute(
    sessions: &Arc<Sessions>,
    proof: &Arc<UnixStream>,
    stream: &tokio::net::UnixStream,
    command: Command,
    permit: DispatchPermit,
    stop: &mut tokio::sync::watch::Receiver<bool>,
) -> Result<Option<Answer>, RunnerError> {
    let left = Arc::new(AtomicBool::new(false));
    let mut cancellation = Cancellation {
        sessions: Arc::clone(sessions),
        output_session: command.output_session(),
        left: Arc::clone(&left),
        proof: Arc::clone(proof),
        armed: true,
    };
    let flag = Arc::clone(&left);
    let held = Arc::clone(sessions);
    let proved = Arc::clone(proof);
    let mut task = tokio::task::spawn_blocking(move || {
        let answer = match command {
            Command::Peer(line) => crate::peer::answer(&held, &proved, line.trim_end(), &flag),
            Command::Server(act) => {
                perform(&held, act, &flag).unwrap_or_else(|error| Answer::refusal(&error))
            }
        };
        permit.finish();
        answer
    });
    tokio::select! {
        result = &mut task => {
            cancellation.armed = false;
            Ok(Some(result.map_err(socket_failed)?))
        }
        result = departed(stream) => {
            cancellation.cancel();
            cancellation.armed = false;
            task.await.map_err(socket_failed)?;
            result?;
            Ok(None)
        }
        changed = stop.changed() => {
            cancellation.cancel();
            cancellation.armed = false;
            task.await.map_err(socket_failed)?;
            changed.map_err(socket_failed)?;
            Ok(None)
        }
    }
}

async fn grant_channel(
    sessions: &Arc<Sessions>,
    stream: &tokio::net::UnixStream,
    stop: &mut tokio::sync::watch::Receiver<bool>,
) -> Result<(), RunnerError> {
    let channel = crate::refusals::Channel::new(sessions);
    let ready = channel.ready();
    let mut pending = Vec::new();
    while !*stop.borrow() {
        let notified = ready.notified();
        let Some(question) = channel.next() else {
            tokio::select! {
                () = notified => continue,
                incoming = read_line(stream, &mut pending) => match incoming? {
                    None => break,
                    Some(_) => return Err(RunnerError::refused("grant_answer_unasked", "the authority sent an answer before a question")),
                },
                changed = stop.changed() => { changed.map_err(socket_failed)?; break; }
            }
        };
        let line = serde_json::to_string(&question).map_err(socket_failed)?;
        tokio::select! {
            result = write_line(stream, format!("{line}\n")) => result?,
            changed = stop.changed() => { changed.map_err(socket_failed)?; break; }
        }
        tokio::select! {
            incoming = read_line(stream, &mut pending) => match incoming? {
                None => break,
                Some(line) => channel.answer(&question, &line)?,
            },
            changed = stop.changed() => { changed.map_err(socket_failed)?; break; }
        }
    }
    Ok(())
}

async fn response(
    stream: &tokio::net::UnixStream,
    answer: Answer,
    stop: &mut tokio::sync::watch::Receiver<bool>,
) -> Result<bool, RunnerError> {
    tokio::select! {
        result = write_line(stream, format!("{}\n", reply_line(answer))) => { result?; Ok(true) },
        changed = stop.changed() => { changed.map_err(socket_failed)?; Ok(false) }
    }
}

async fn connection(
    sessions: &Arc<Sessions>,
    server: [u8; 32],
    stream: tokio::net::UnixStream,
    slots: Arc<DispatchSlots>,
    mut stop: tokio::sync::watch::Receiver<bool>,
) -> Result<(), RunnerError> {
    let stream = stream.into_std().map_err(socket_failed)?;
    let proof = Arc::new(stream.try_clone().map_err(socket_failed)?);
    let stream = tokio::net::UnixStream::from_std(stream).map_err(socket_failed)?;
    let mut pending = Vec::new();
    while !*stop.borrow() {
        let greeting = Greeting::fresh(sessions.runner());
        tokio::select! {
            result = write_line(&stream, format!("{}\n", greeting.line())) => result?,
            changed = stop.changed() => { changed.map_err(socket_failed)?; break; }
        }
        let line = tokio::select! {
            result = read_line(&stream, &mut pending) => match result {
                Ok(Some(line)) => line,
                Ok(None) => break,
                Err(error) => { response(&stream, Answer::refusal(&error), &mut stop).await?; break; }
            },
            changed = stop.changed() => { changed.map_err(socket_failed)?; break; }
        };
        let command = match command(line, &server, &greeting) {
            Ok(command) if pending.is_empty() => command,
            Ok(_) => {
                response(
                    &stream,
                    Answer::refusal(&RunnerError::refused(
                        "request_pipelined",
                        "send the next request after its greeting",
                    )),
                    &mut stop,
                )
                .await?;
                break;
            }
            Err(error) => {
                if !response(&stream, Answer::refusal(&error), &mut stop).await? {
                    break;
                }
                continue;
            }
        };
        if matches!(command, Command::Server(Act::GrantChannel)) {
            if response(&stream, Answer::GrantChannel, &mut stop).await? {
                grant_channel(sessions, &stream, &mut stop).await?;
            }
            break;
        }
        let permit = match slots.acquire(command.control()) {
            Ok(permit) => permit,
            Err(error) => {
                if !response(
                    &stream,
                    Answer::refusal(&RunnerError::refused(
                        "runner_dispatch_full",
                        error.to_string(),
                    )),
                    &mut stop,
                )
                .await?
                {
                    break;
                }
                continue;
            }
        };
        let Some(answer) = execute(sessions, &proof, &stream, command, permit, &mut stop).await?
        else {
            break;
        };
        if !response(&stream, answer, &mut stop).await? {
            break;
        }
    }
    match proof.shutdown(Shutdown::Both) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotConnected => Ok(()),
        Err(error) => Err(socket_failed(error)),
    }
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
