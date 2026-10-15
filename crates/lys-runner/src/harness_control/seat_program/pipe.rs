//! ACP writes share one pipe, which remains owned until the run ends.
use crate::RunnerError;
use serde_json::{Value, json};
use std::io::Write;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

pub(crate) struct Pipe {
    writer: Mutex<Option<Box<dyn Write + Send>>>,
    session: Mutex<Option<String>>,
    next: AtomicU64,
    pub(crate) directory: String,
    pub(crate) proof: Value,
    pub(crate) answer: std::sync::OnceLock<Value>,
}

impl Pipe {
    pub(crate) fn new(writer: Box<dyn Write + Send>, directory: String, proof: Value) -> Arc<Self> {
        Arc::new(Self {
            writer: Mutex::new(Some(writer)),
            session: Mutex::new(None),
            next: AtomicU64::new(1),
            directory,
            proof,
            answer: std::sync::OnceLock::new(),
        })
    }
    pub(crate) fn send(&self, frame: &Value) -> Result<(), RunnerError> {
        let mut held = self
            .writer
            .lock()
            .map_err(|error| RunnerError::refused("acp_input_poisoned", error.to_string()))?;
        let writer = held.as_mut().ok_or_else(|| {
            RunnerError::refused("acp_stdin_closed", "the seat's stdin is closed")
        })?;
        serde_json::to_writer(&mut **writer, frame)
            .map_err(|error| RunnerError::refused("acp_write_failed", error.to_string()))?;
        writer
            .write_all(b"\n")
            .and_then(|()| writer.flush())
            .map_err(|error| RunnerError::refused("acp_write_failed", error.to_string()))
    }
    pub(crate) fn close(&self) -> Result<(), RunnerError> {
        drop(
            self.writer
                .lock()
                .map_err(|error| RunnerError::refused("acp_input_poisoned", error.to_string()))?
                .take(),
        );
        Ok(())
    }
    pub(crate) fn bind(&self, session: &str) -> Result<(), RunnerError> {
        if session.is_empty() {
            return Err(RunnerError::refused(
                "acp_session_unbound",
                "session/new answered no sessionId",
            ));
        }
        *self
            .session
            .lock()
            .map_err(|error| RunnerError::refused("acp_input_poisoned", error.to_string()))? =
            Some(session.to_owned());
        Ok(())
    }
    fn prompt(&self, bytes: &[u8]) -> Result<(), RunnerError> {
        let session = self
            .session
            .lock()
            .map_err(|error| RunnerError::refused("acp_input_poisoned", error.to_string()))?
            .clone()
            .ok_or_else(|| {
                RunnerError::refused(
                    "acp_session_unbound",
                    "the adapter has not answered session/new",
                )
            })?;
        let text = std::str::from_utf8(bytes)
            .map_err(|error| RunnerError::refused("acp_input_invalid", error.to_string()))?
            .trim_end_matches(['\r', '\n']);
        let next = self
            .next
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |next| {
                next.checked_add(1)
            })
            .map_err(|current| {
                RunnerError::refused(
                    "acp_sequence_exhausted",
                    format!("the request sequence is exhausted at {current}"),
                )
            })?;
        self.send(&json!({"jsonrpc":"2.0", "id":format!("lys-prompt-{next}"), "method":"session/prompt", "params":{"sessionId":session, "prompt":[{"type":"text","text":text}]}}))
    }
}

pub(crate) struct Writer {
    pipe: Arc<Pipe>,
    bytes: Vec<u8>,
}
impl Writer {
    pub(crate) fn new(pipe: Arc<Pipe>) -> Self {
        Self {
            pipe,
            bytes: Vec::new(),
        }
    }
}
impl Write for Writer {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.pipe
            .prompt(&self.bytes)
            .map_err(std::io::Error::other)?;
        self.bytes.clear();
        Ok(())
    }
}
impl Drop for Writer {
    fn drop(&mut self) {
        if let Err(error) = self.pipe.close() {
            crate::error::said(&format!("acp_close_failed: {error}"));
        }
    }
}
