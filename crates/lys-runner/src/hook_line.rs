//! Claude hook intake uses a bounded peer exchange without holding up the harness.

use std::io::{BufRead, BufReader, Read, Write};
use std::os::fd::{AsFd, AsRawFd};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::{Duration, Instant};

use nix::errno::Errno;
use nix::fcntl::{FcntlArg, FdFlag, fcntl};
use nix::poll::{PollFd, PollFlags, PollTimeout, poll};
use nix::sys::socket::{AddressFamily, SockFlag, SockType, UnixAddr, connect, socket};
use serde_json::Value;

use crate::error::RunnerError;
use crate::peer::{Collected, PeerAct, PeerRequest};
use crate::protocol::{Answer, PROTOCOL_VERSION, read_greeting, read_reply};

// Readiness can resume after its timeout; leave time to return within three seconds.
const EXCHANGE_BUDGET: Duration = Duration::from_secs(2);

/// The hook command's inputs and captured process streams.
pub struct Args<'a> {
    /// The runner's Unix socket.
    pub socket: &'a Path,
    /// The settings entry's event, overriding the payload when present.
    pub event: Option<&'a str>,
    /// The hook's complete JSON input.
    pub stdin: &'a mut dyn Read,
    /// Standard output, kept empty so the harness proceeds.
    pub stdout: &'a mut dyn Write,
    /// Standard error, receiving one named diagnostic on failure.
    pub stderr: &'a mut dyn Write,
}

fn request(stdin: &str, event: Option<&str>) -> Result<PeerRequest, RunnerError> {
    let input: Value = serde_json::from_str(stdin).map_err(|error| {
        RunnerError::refused(
            "hook_input_malformed",
            format!("hook stdin is not JSON ({:?})", error.classify()),
        )
    })?;
    if !input.is_object() {
        return Err(RunnerError::refused(
            "hook_input_malformed",
            "hook stdin must be a JSON object",
        ));
    }
    let event = event
        .or_else(|| input.get("hook_event_name").and_then(Value::as_str))
        .filter(|event| !event.trim().is_empty())
        .ok_or_else(|| {
            RunnerError::refused(
                "hook_input_malformed",
                "hook_event_name must be a nonempty string, or --event must name the event",
            )
        })?;
    Ok(PeerRequest {
        version: PROTOCOL_VERSION,
        peer: PeerAct::Collect(Collected::Hook {
            event: event.to_owned(),
            input,
        }),
    })
}

/// Hand the whole hook input to the runner, preserving its answer.
///
/// # Errors
/// Names malformed input, an unreachable runner, a refused request or
/// an exhausted socket wait allowance within the three-second exchange bound.
pub fn hand(socket: &Path, stdin: &str, event: Option<&str>) -> Result<Answer, RunnerError> {
    exchange(socket, &request(stdin, event)?)
}

/// Read and hand one hook to its runner. Every outcome exits zero; a failure
/// is named on standard error, and standard output remains empty.
pub fn run(
    Args {
        socket,
        event,
        stdin: input,
        stdout: _,
        stderr,
    }: Args<'_>,
) -> i32 {
    let mut stdin = String::new();
    let handed = input
        .read_to_string(&mut stdin)
        .map_err(|error| {
            RunnerError::refused("hook_input_unread", format!("reading hook stdin: {error}"))
        })
        .and_then(|_| hand(socket, &stdin, event));
    if let Err(error) = handed {
        report(stderr, "hook", &error);
    }
    0
}

pub(crate) fn report(stderr: &mut dyn Write, command: &str, error: &RunnerError) {
    let line = format!("lys runner {command}: {error}").replace(['\r', '\n'], " ");
    if let Err(error) = writeln!(stderr, "{line}").and_then(|()| stderr.flush()) {
        crate::error::said(&format!(
            "collector_diagnostic_unwritten: {line}; writing stderr failed: {error}"
        ));
    }
}

fn unreachable(stage: &str, error: impl std::fmt::Display) -> RunnerError {
    RunnerError::Unreachable {
        reason: format!("{stage}: {error}"),
    }
}

fn remaining(deadline: Instant, stage: &str) -> Result<Duration, RunnerError> {
    deadline
        .checked_duration_since(Instant::now())
        .filter(|left| !left.is_zero())
        .ok_or_else(|| {
            RunnerError::refused(
                "collector_timeout",
                format!("{stage} exceeded the two-second connect and exchange wait allowance"),
            )
        })
}

fn ready(
    stream: &UnixStream,
    interest: PollFlags,
    deadline: Instant,
    stage: &str,
) -> Result<(), RunnerError> {
    loop {
        let timeout = PollTimeout::try_from(remaining(deadline, stage)?)
            .map_err(|error| unreachable(stage, error))?;
        let mut descriptors = [PollFd::new(stream.as_fd(), interest)];
        match poll(&mut descriptors, timeout) {
            Ok(0) => {
                return Err(RunnerError::refused(
                    "collector_timeout",
                    format!("{stage} exhausted the two-second connect and exchange wait allowance"),
                ));
            }
            Ok(_) => return Ok(()),
            Err(Errno::EINTR) => {}
            Err(error) => return Err(unreachable(stage, error)),
        }
    }
}

fn connection(path: &Path, deadline: Instant) -> Result<UnixStream, RunnerError> {
    let stage = format!("connecting to {}", path.display());
    let address = UnixAddr::new(path).map_err(|error| unreachable(&stage, error))?;
    let descriptor = socket(
        AddressFamily::Unix,
        SockType::Stream,
        SockFlag::empty(),
        None,
    )
    .map_err(|error| unreachable(&stage, error))?;
    let stream = UnixStream::from(descriptor);
    fcntl(stream.as_raw_fd(), FcntlArg::F_SETFD(FdFlag::FD_CLOEXEC))
        .map_err(|error| unreachable(&stage, error))?;
    stream
        .set_nonblocking(true)
        .map_err(|error| unreachable(&stage, error))?;
    remaining(deadline, &stage)?;
    match connect(stream.as_raw_fd(), &address) {
        Ok(()) => {}
        Err(Errno::EINPROGRESS) => {
            ready(&stream, PollFlags::POLLOUT, deadline, &stage)?;
            if let Some(error) = stream
                .take_error()
                .map_err(|error| unreachable(&stage, error))?
            {
                return Err(unreachable(&stage, error));
            }
        }
        Err(error) => return Err(unreachable(&stage, error)),
    }
    Ok(stream)
}

fn line(
    reader: &mut BufReader<UnixStream>,
    deadline: Instant,
    stage: &str,
) -> Result<String, RunnerError> {
    let mut bytes = Vec::new();
    loop {
        remaining(deadline, stage)?;
        let (count, ended) = match reader.fill_buf() {
            Ok([]) => return Err(unreachable(stage, "runner closed before the line ended")),
            Ok(buffer) => {
                let end = buffer.iter().position(|byte| *byte == b'\n');
                let count = end.map_or(buffer.len(), |end| end + 1);
                bytes.extend_from_slice(&buffer[..count]);
                (count, end.is_some())
            }
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                ready(reader.get_ref(), PollFlags::POLLIN, deadline, stage)?;
                continue;
            }
            Err(error) => return Err(unreachable(stage, error)),
        };
        reader.consume(count);
        if ended {
            return String::from_utf8(bytes).map_err(|error| RunnerError::ReplyMalformed {
                reason: format!("{stage} is not UTF-8: {}", error.utf8_error()),
            });
        }
    }
}

fn send(stream: &mut UnixStream, bytes: &[u8], deadline: Instant) -> Result<(), RunnerError> {
    let mut pending = bytes;
    while !pending.is_empty() {
        remaining(deadline, "writing collector request")?;
        match stream.write(pending) {
            Ok(0) => {
                return Err(unreachable(
                    "writing collector request",
                    "runner closed the connection",
                ));
            }
            Ok(count) => pending = &pending[count..],
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                ready(
                    stream,
                    PollFlags::POLLOUT,
                    deadline,
                    "writing collector request",
                )?;
            }
            Err(error) => return Err(unreachable("writing collector request", error)),
        }
    }
    Ok(())
}

pub(crate) fn exchange(socket: &Path, request: &PeerRequest) -> Result<Answer, RunnerError> {
    let mut request = serde_json::to_vec(request).map_err(|error| RunnerError::Malformed {
        reason: format!("collector request could not be encoded: {error}"),
    })?;
    request.push(b'\n');
    let deadline = Instant::now() + EXCHANGE_BUDGET;
    let mut reader = BufReader::new(connection(socket, deadline)?);
    read_greeting(&line(&mut reader, deadline, "reading runner greeting")?)?;
    send(reader.get_mut(), &request, deadline)?;
    read_reply(&line(&mut reader, deadline, "reading collector answer")?)
}
