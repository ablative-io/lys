//! One bounded writer batches ready work and acknowledges only durable writes.

use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, mpsc};
use std::thread::JoinHandle;

use crate::error::RunnerError;

type ResultLine = Result<(), String>;

enum WriteKind {
    Append(Vec<u8>),
    Replace(Vec<u8>),
}

struct Command {
    write: Option<(PathBuf, WriteKind)>,
    answer: Option<mpsc::Sender<ResultLine>>,
}

struct Inner {
    sender: Option<mpsc::SyncSender<Command>>,
    fault: Arc<Mutex<Option<String>>>,
    worker: Option<JoinHandle<()>>,
}

#[derive(Clone)]
pub(crate) struct Writer(Arc<Inner>);

fn refused(reason: impl std::fmt::Display) -> RunnerError {
    RunnerError::refused("durable_writer_unavailable", reason.to_string())
}

impl Writer {
    pub(crate) fn new() -> Result<Self, RunnerError> {
        let (sender, receiver) = mpsc::sync_channel(32);
        let fault = Arc::new(Mutex::new(None));
        let faults = Arc::clone(&fault);
        let worker = std::thread::Builder::new()
            .name("runner-writer".to_owned())
            .spawn(move || run(&receiver, &faults))
            .map_err(refused)?;
        Ok(Self(Arc::new(Inner {
            sender: Some(sender),
            fault,
            worker: Some(worker),
        })))
    }

    fn sender(&self) -> Result<&mpsc::SyncSender<Command>, RunnerError> {
        let fault = self.0.fault.lock().map_err(refused)?;
        if let Some(reason) = fault.as_ref() {
            return Err(refused(reason));
        }
        self.0
            .sender
            .as_ref()
            .ok_or_else(|| refused("the writer has ended"))
    }

    pub(crate) fn append(&self, path: &Path, bytes: Vec<u8>) -> Result<(), RunnerError> {
        self.enqueue(path, WriteKind::Append(bytes))
    }

    pub(crate) fn replace(&self, path: &Path, bytes: Vec<u8>) -> Result<(), RunnerError> {
        self.enqueue(path, WriteKind::Replace(bytes))
    }

    fn enqueue(&self, path: &Path, write: WriteKind) -> Result<(), RunnerError> {
        self.sender()?
            .try_send(Command {
                write: Some((path.to_owned(), write)),
                answer: None,
            })
            .map_err(|error| match error {
                mpsc::TrySendError::Full(_) => RunnerError::refused(
                    "durable_writer_busy",
                    "the durable writer's bounded queue is full",
                ),
                mpsc::TrySendError::Disconnected(_) => refused("the writer has ended"),
            })
    }

    /// Called after releasing the table; a fence also observes an earlier write failure.
    pub(crate) fn barrier(&self) -> Result<(), RunnerError> {
        let (answer, answered) = mpsc::channel();
        self.sender()?
            .send(Command {
                write: None,
                answer: Some(answer),
            })
            .map_err(refused)?;
        answered.recv().map_err(refused)?.map_err(refused)
    }
}

impl Drop for Inner {
    fn drop(&mut self) {
        drop(self.sender.take());
        if let Some(worker) = self.worker.take() {
            if let Err(panic) = worker.join() {
                crate::error::said(&format!("durable_writer_panicked: {panic:?}"));
            }
        }
    }
}

fn run(receiver: &mpsc::Receiver<Command>, fault: &Mutex<Option<String>>) {
    while let Ok(first) = receiver.recv() {
        // A batch of at most 32 is one flush; the rest wait for the next pass.
        let mut commands = std::iter::once(first)
            .chain(receiver.try_iter().take(31))
            .collect::<Vec<_>>();
        let previous = match fault.lock() {
            Ok(held) => held.clone(),
            Err(error) => Some(format!("writer failure state is poisoned: {error}")),
        };
        let result = match previous {
            Some(reason) => Err(reason),
            None => batch(&mut commands),
        };
        if let Err(reason) = &result {
            match fault.lock() {
                Ok(mut held) => {
                    held.get_or_insert_with(|| reason.clone());
                }
                Err(error) => {
                    crate::error::said(&format!("durable_writer_state_failed: {error}"));
                }
            }
        }
        for command in commands {
            if let Some(answer) = command.answer {
                if let Err(error) = answer.send(result.clone()) {
                    crate::error::said(&format!("durable_writer_answer_lost: {error}"));
                }
            }
        }
    }
}

fn batch(commands: &mut [Command]) -> ResultLine {
    let mut writes: BTreeMap<PathBuf, WriteKind> = BTreeMap::new();
    for command in commands {
        let Some((path, write)) = command.write.take() else {
            continue;
        };
        match write {
            WriteKind::Append(bytes) => match writes.entry(path) {
                std::collections::btree_map::Entry::Vacant(entry) => {
                    entry.insert(WriteKind::Append(bytes));
                }
                std::collections::btree_map::Entry::Occupied(mut entry) => {
                    let WriteKind::Append(held) = entry.get_mut() else {
                        return Err("one file has mixed append and replace writes".to_owned());
                    };
                    held.extend_from_slice(&bytes);
                }
            },
            WriteKind::Replace(bytes) => {
                if matches!(writes.get(&path), Some(WriteKind::Append(_))) {
                    return Err("one file has mixed append and replace writes".to_owned());
                }
                writes.insert(path, WriteKind::Replace(bytes));
            }
        }
    }
    for (path, write) in writes {
        let result = match write {
            WriteKind::Append(bytes) => std::fs::OpenOptions::new()
                .append(true)
                .open(&path)
                .and_then(|mut file| file.write_all(&bytes).and_then(|()| file.sync_data())),
            WriteKind::Replace(bytes) => crate::state::replace(&path, &bytes),
        };
        result.map_err(|error| format!("{}: {error}", path.display()))?;
    }
    Ok(())
}
