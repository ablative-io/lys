//! One connection: its greeting, its framed lines, its requests dispatched
//! within the bounded slots with one held for control, and its acts cancelled
//! when the caller leaves.

use super::{DISPATCH_MAX, LINE_MAX, perform, socket_failed};
use crate::error::RunnerError;
use crate::protocol::{Act, Answer, Greeting, reply_line, verify_parsed};
use crate::session::Sessions;
use std::io;
use std::net::Shutdown;
use std::os::unix::net::UnixStream;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

pub(super) async fn write_line(
    stream: &tokio::net::UnixStream,
    line: String,
) -> Result<(), RunnerError> {
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

pub(super) async fn read_line(
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

pub(super) async fn departed(stream: &tokio::net::UnixStream) -> Result<(), RunnerError> {
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

pub(super) enum Command {
    Peer(crate::peer::PeerRequest),
    Server(Act),
}

impl Command {
    pub(super) fn output_session(&self) -> Option<String> {
        match self {
            Self::Server(
                Act::Read { session, .. }
                | Act::ReadBytes { session, .. }
                | Act::Wait { session, .. },
            ) => Some(session.clone()),
            _ => None,
        }
    }

    pub(super) fn control(&self) -> bool {
        matches!(self, Self::Server(Act::Status { .. }))
            || matches!(self,
            Self::Server(Act::Operate { operation }) if operation.request == crate::operations::OperationRequest::Stop)
    }
}

pub(super) struct DispatchSlots {
    total: Arc<tokio::sync::Semaphore>,
    ordinary: Arc<tokio::sync::Semaphore>,
}

pub(super) struct DispatchPermit {
    total: tokio::sync::OwnedSemaphorePermit,
    ordinary: Option<tokio::sync::OwnedSemaphorePermit>,
}

impl DispatchSlots {
    pub(super) fn new() -> Self {
        Self {
            total: Arc::new(tokio::sync::Semaphore::new(DISPATCH_MAX)),
            ordinary: Arc::new(tokio::sync::Semaphore::new(DISPATCH_MAX - 1)),
        }
    }

    pub(super) fn acquire(
        &self,
        control: bool,
    ) -> Result<DispatchPermit, tokio::sync::TryAcquireError> {
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
    pub(super) fn finish(self) {
        drop(self.total);
        drop(self.ordinary);
    }
}

pub(super) fn command(
    line: &str,
    server: &[u8; 32],
    greeting: &Greeting,
) -> Result<Command, RunnerError> {
    match crate::peer::parse(line.trim_end())? {
        crate::peer::ParsedRequest::Peer(request) => Ok(Command::Peer(request)),
        crate::peer::ParsedRequest::Server(request) => {
            verify_parsed(&request, server, greeting).map(Command::Server)
        }
        crate::peer::ParsedRequest::Versioned { .. } => Err(RunnerError::Malformed {
            reason: "the request shape was not decoded".to_owned(),
        }),
    }
}

pub(super) struct Cancellation {
    sessions: Arc<Sessions>,
    output_session: Option<String>,
    left: Arc<AtomicBool>,
    proof: Arc<UnixStream>,
    armed: bool,
}

impl Cancellation {
    pub(super) fn cancel(&self) {
        let table = self.sessions.lock();
        self.left.store(true, Ordering::SeqCst);
        self.sessions.wake();
        match table {
            Ok(table) => drop(table),
            Err(error) => crate::error::said(&error.to_string()),
        }
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

pub(super) async fn execute(
    sessions: &Arc<Sessions>,
    proof: &Arc<UnixStream>,
    stream: &tokio::net::UnixStream,
    command: Command,
    permit: DispatchPermit,
    stop: &mut tokio::sync::watch::Receiver<bool>,
    peer: &mut crate::peer::Connection,
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
    let mut cached = std::mem::take(peer);
    let mut task = tokio::task::spawn_blocking(move || {
        let answer = match command {
            Command::Peer(request) => cached.answer(&held, &proved, request, &flag),
            Command::Server(act) => {
                perform(&held, act, &flag).unwrap_or_else(|error| Answer::refusal(&error))
            }
        };
        permit.finish();
        (answer, cached)
    });
    tokio::select! {
        result = &mut task => {
            cancellation.armed = false;
            let (answer, cached) = result.map_err(socket_failed)?;
            *peer = cached;
            Ok(Some(answer))
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

pub(super) async fn grant_channel(
    sessions: &Arc<Sessions>,
    stream: &tokio::net::UnixStream,
    stop: &mut tokio::sync::watch::Receiver<bool>,
) -> Result<(), RunnerError> {
    let channel = crate::refusals::Channel::new(sessions)?;
    let ready = channel.ready()?;
    let mut pending = Vec::new();
    while !*stop.borrow() {
        let notified = ready.notified();
        let Some(question) = channel.next()? else {
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

pub(super) async fn response(
    stream: &tokio::net::UnixStream,
    answer: Answer,
    stop: &mut tokio::sync::watch::Receiver<bool>,
) -> Result<bool, RunnerError> {
    tokio::select! {
        result = write_line(stream, format!("{}\n", reply_line(answer))) => { result?; Ok(true) },
        changed = stop.changed() => { changed.map_err(socket_failed)?; Ok(false) }
    }
}

pub(super) async fn connection(
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
    let mut peer = crate::peer::Connection::default();
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
        let command = match command(&line, &server, &greeting) {
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
        let Some(answer) = execute(
            sessions, &proof, &stream, command, permit, &mut stop, &mut peer,
        )
        .await?
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
