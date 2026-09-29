//! Take the managed child's pipes from its existing launch owner. This module
//! spawns nothing and never changes containment, environment or credentials.
//! There is one stdout reader and one serialized writer. A successful write
//! is not harness admission; the correlated native event establishes that.

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout};

use serde_json::Value;

use crate::error::RunnerError;
use crate::peer::{Leader, Processes, System};

/// A journal backed by 051's existing operation owner. Implementations must
/// durably mark the stable operation possibly sent before returning success;
/// an already possibly-sent operation must refuse, never authorize a replay.
pub trait WriteAhead {
    /// Make the request identity/digest and possibly-sent state durable.
    fn before_write(&mut self, operation: &str, encoded: &[u8]) -> Result<(), RunnerError>;
}

/// Exclusive access to an existing managed child's stdin and stdout.
pub struct Pipes {
    writer: ChildStdin,
    reader: Option<Reader>,
    leader: Leader,
}

/// The one native JSON reader. No clone or second observer of stdout exists.
pub struct Reader(BufReader<ChildStdout>);

fn refused(name: &str, reason: impl Into<String>) -> RunnerError {
    RunnerError::refused(name, reason)
}

impl Pipes {
    /// Attach only to the process whose start identity the launch owner proved.
    /// The caller retains Child and its existing stop/wait lifecycle authority.
    pub fn attach(child: &mut Child, expected: &Leader) -> Result<Self, RunnerError> {
        if child.id() != expected.pid || System.start(child.id())? != expected.start {
            return Err(refused(
                "control_process_mismatch",
                "child differs from the proved process start",
            ));
        }
        if child.stdin.is_none() || child.stdout.is_none() {
            return Err(refused(
                "control_transport_unsupported",
                "managed control requires exclusive stdin and stdout pipes",
            ));
        }
        let writer = child
            .stdin
            .take()
            .ok_or_else(|| refused("control_transport_unsupported", "stdin is not owned"))?;
        let reader = child
            .stdout
            .take()
            .ok_or_else(|| refused("control_transport_unsupported", "stdout is not owned"))?;
        Ok(Self {
            writer,
            reader: Some(Reader(BufReader::new(reader))),
            leader: expected.clone(),
        })
    }

    /// The actual process this exclusive transport was attached to.
    pub(crate) fn leader(&self) -> &Leader {
        &self.leader
    }

    /// Transfer the sole reader to the transport's event pump.
    pub fn take_reader(&mut self) -> Result<Reader, RunnerError> {
        self.reader.take().ok_or_else(|| {
            refused(
                "control_reader_already_taken",
                "this process already has its one native event reader",
            )
        })
    }

    /// Encode a frame as JSON data and durably mark the operation possibly sent
    /// before writing anything. No automatic retry follows any write failure.
    pub fn send(
        &mut self,
        operation: &str,
        frame: &Value,
        journal: &mut impl WriteAhead,
    ) -> Result<(), RunnerError> {
        if System.start(self.leader.pid)? != self.leader.start {
            return Err(refused(
                "control_process_mismatch",
                "the managed process start identity changed",
            ));
        }
        let mut bytes = serde_json::to_vec(frame).map_err(|error| {
            refused(
                "control_frame_invalid",
                format!("frame could not be encoded: {:?}", error.classify()),
            )
        })?;
        bytes.push(b'\n');
        journal.before_write(operation, &bytes)?;
        self.writer.write_all(&bytes).and_then(|()| self.writer.flush())
            .map_err(|error| refused("control_delivery_uncertain", format!("operation {operation} may have reached its pipe ({:?}); reconcile the existing receipt before any new send", error.kind())))
    }
}

impl Reader {
    /// One native message. EOF means transport loss, never an idle boundary.
    /// Errors carry no native payload, prompt text or credential bytes.
    pub fn next_frame(&mut self) -> Result<Option<Value>, RunnerError> {
        let mut line = String::new();
        if self.0.read_line(&mut line).map_err(|error| {
            refused(
                "control_transport_lost",
                format!("the native pipe could not be read: {:?}", error.kind()),
            )
        })? == 0
        {
            return Ok(None);
        }
        serde_json::from_str(&line).map(Some).map_err(|error| {
            refused(
                "control_protocol_unsupported",
                format!(
                    "the native pipe produced an unsupported JSON frame: {:?}",
                    error.classify()
                ),
            )
        })
    }
}
