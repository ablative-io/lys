//! The client of any runner that speaks the protocol on a Unix socket.
//!
//! A connection carries the runner's greeting, then one request and one
//! reply; the request is signed over the greeting's runner and challenge,
//! so it is good on that connection alone. A caller that gives up
//! closes the connection, from any thread, through its [`Closer`]; the
//! runner then stops waiting on the request. Nothing here waits on a clock.

use std::io::{BufRead, BufReader, Write};
use std::net::Shutdown;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use lys_core::Ed25519Identity;

use crate::error::RunnerError;
use crate::protocol::{Act, Answer, Greeting, read_greeting, read_reply, sign_request};

/// A client of any runner speaking this protocol on a Unix socket, signing
/// each request with `key`.
pub struct Client {
    socket: PathBuf,
    key: Arc<Ed25519Identity>,
}

/// One connection to a runner, carrying one request.
pub struct Connection {
    stream: UnixStream,
    reader: BufReader<UnixStream>,
}

/// What closes a connection from another thread, so the runner stops
/// waiting on the request it carries.
pub struct Closer {
    stream: UnixStream,
}

impl Client {
    /// The client of the runner on `socket`, signing with `key`.
    pub fn new(socket: impl Into<PathBuf>, key: Arc<Ed25519Identity>) -> Self {
        Self {
            socket: socket.into(),
            key,
        }
    }

    /// Open a connection.
    pub fn connect(&self) -> Result<Connection, RunnerError> {
        connect(&self.socket)
    }

    /// Ask `act` on a connection of its own, and answer the runner's answer.
    pub fn ask(&self, act: &Act) -> Result<Answer, RunnerError> {
        self.connect()?.ask(&self.key, act)
    }
}

fn unreachable(error: &std::io::Error) -> RunnerError {
    RunnerError::Unreachable {
        reason: error.to_string(),
    }
}

/// Open a connection to the runner on `socket`. Its greeting is read by
/// [`Connection::greeting_line`], so a caller can hold its [`Closer`] first.
pub fn connect(socket: &Path) -> Result<Connection, RunnerError> {
    let stream = UnixStream::connect(socket).map_err(|error| RunnerError::Unreachable {
        reason: format!("{}: {error}", socket.display()),
    })?;
    let reader = BufReader::new(stream.try_clone().map_err(|error| unreachable(&error))?);
    Ok(Connection { stream, reader })
}

impl Connection {
    /// What closes this connection from another thread.
    pub fn closer(&self) -> Result<Closer, RunnerError> {
        self.stream
            .try_clone()
            .map(|stream| Closer { stream })
            .map_err(|error| unreachable(&error))
    }

    /// The line the runner says first on this connection, as it came.
    pub fn greeting_line(&mut self) -> Result<String, RunnerError> {
        let mut line = String::new();
        self.reader
            .read_line(&mut line)
            .map_err(|error| unreachable(&error))?;
        if line.is_empty() {
            return Err(RunnerError::Unreachable {
                reason: "the runner closed the connection before it greeted".to_owned(),
            });
        }
        Ok(line.trim_end().to_owned())
    }

    /// The runner's greeting on this connection, refused
    /// `runner_protocol_mismatch` when it speaks another version.
    pub fn greeting(&mut self) -> Result<Greeting, RunnerError> {
        read_greeting(&self.greeting_line()?)
    }

    /// Ask `act`, signed by `key` over this connection's greeting.
    pub fn ask(mut self, key: &Ed25519Identity, act: &Act) -> Result<Answer, RunnerError> {
        let line = sign_request(key, &self.greeting()?, act)?;
        read_reply(&self.exchange(&line)?)
    }

    /// Send the request `line`, once the greeting is read, and answer the
    /// reply line as it came.
    pub fn exchange(mut self, line: &str) -> Result<String, RunnerError> {
        let mut writer = &self.stream;
        writer
            .write_all(line.trim_end().as_bytes())
            .and_then(|()| writer.write_all(b"\n"))
            .and_then(|()| writer.flush())
            .map_err(|error| unreachable(&error))?;
        let mut reply = String::new();
        self.reader
            .read_line(&mut reply)
            .map_err(|error| unreachable(&error))?;
        if reply.is_empty() {
            return Err(RunnerError::Unreachable {
                reason: "the runner closed the connection before it answered".to_owned(),
            });
        }
        Ok(reply)
    }
}

impl Closer {
    /// Close the connection: the caller has left.
    pub fn close(&self) {
        if let Err(error) = self.stream.shutdown(Shutdown::Both) {
            crate::error::said(&format!("the connection was already closed: {error}"));
        }
    }
}
