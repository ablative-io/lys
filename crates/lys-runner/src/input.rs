//! Each session owns an ordered input path. Only its worker touches the
//! terminal writer; a full terminal never holds the shared session table.

use std::io::Write;
use std::sync::{Arc, Mutex, PoisonError, mpsc};

use crate::error::RunnerError;

type Completion = Box<dyn FnOnce(Result<(), RunnerError>) + Send>;

struct Span {
    bytes: Vec<u8>,
    complete: Completion,
}

struct State {
    writer: Option<Box<dyn Write + Send>>,
    sender: Option<mpsc::Sender<Span>>,
}

/// The session's whole input spans, completed in submission order.
#[derive(Clone)]
pub(crate) struct Input {
    state: Arc<Mutex<State>>,
}

impl Input {
    pub(crate) fn new(writer: Box<dyn Write + Send>) -> Self {
        Self {
            state: Arc::new(Mutex::new(State {
                writer: Some(writer),
                sender: None,
            })),
        }
    }

    /// Enqueue without waiting for the terminal. Completion must not wait
    /// for another span on this same input path.
    pub(crate) fn submit(
        &self,
        bytes: Vec<u8>,
        complete: impl FnOnce(Result<(), RunnerError>) + Send + 'static,
    ) -> Result<(), RunnerError> {
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        if state.sender.is_none() {
            let writer = state.writer.take().ok_or_else(|| {
                RunnerError::refused("input_worker_failed", "the input worker could not start")
            })?;
            let (sender, receiver) = mpsc::channel();
            let worker = std::thread::Builder::new()
                .spawn(move || run(writer, receiver))
                .map_err(|error| RunnerError::refused("input_worker_failed", error.to_string()))?;
            drop(worker);
            state.sender = Some(sender);
        }
        let sender = state.sender.as_ref().ok_or_else(|| {
            RunnerError::refused("input_worker_failed", "the input worker has no queue")
        })?;
        sender
            .send(Span {
                bytes,
                complete: Box::new(complete),
            })
            .map_err(|error| RunnerError::refused("input_worker_ended", error.to_string()))
    }

    /// Wait on this span's completion, with no shared session lock held.
    pub(crate) fn write(&self, bytes: Vec<u8>) -> Result<(), RunnerError> {
        let (complete, completed) = mpsc::channel();
        self.submit(bytes, move |result| {
            if let Err(error) = complete.send(result) {
                crate::error::said(&format!("input completion was not received: {error}"));
            }
        })?;
        completed
            .recv()
            .map_err(|error| RunnerError::refused("input_worker_ended", error.to_string()))?
    }
}

fn run(mut writer: Box<dyn Write + Send>, receiver: mpsc::Receiver<Span>) {
    for span in receiver {
        let result = writer
            .write_all(&span.bytes)
            .and_then(|()| writer.flush())
            .map_err(|error| RunnerError::refused("write_failed", error.to_string()));
        (span.complete)(result);
    }
}
