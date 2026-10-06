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

struct Reserved {
    path: PathBuf,
    bytes: mpsc::Receiver<Result<Vec<u8>, String>>,
}

struct Command {
    write: Option<(PathBuf, WriteKind)>,
    answer: Option<mpsc::Sender<ResultLine>>,
    reserved: Option<Reserved>,
}

#[cfg(test)]
#[derive(Default, Clone, Copy)]
struct Count {
    syncs: u64,
    replacement: Option<std::time::Duration>,
}

struct Inner {
    sender: Option<mpsc::SyncSender<Command>>,
    fault: Arc<Mutex<Option<String>>>,
    worker: Option<JoinHandle<()>>,
    #[cfg(test)]
    counts: Arc<Mutex<BTreeMap<PathBuf, Count>>>,
    #[cfg(test)]
    serial: std::sync::atomic::AtomicBool,
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
        #[cfg(test)]
        let counts = Arc::new(Mutex::new(BTreeMap::new()));
        #[cfg(test)]
        let counter = Arc::clone(&counts);
        let worker = std::thread::Builder::new()
            .name("runner-writer".to_owned())
            .spawn(move || {
                #[cfg(test)]
                run(&receiver, &faults, &counter);
                #[cfg(not(test))]
                run(&receiver, &faults);
            })
            .map_err(refused)?;
        Ok(Self(Arc::new(Inner {
            sender: Some(sender),
            fault,
            worker: Some(worker),
            #[cfg(test)]
            counts,
            #[cfg(test)]
            serial: std::sync::atomic::AtomicBool::new(false),
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
        self.enqueue(path, WriteKind::Append(bytes))?;
        #[cfg(test)]
        if self.0.serial.load(std::sync::atomic::Ordering::SeqCst)
            && path
                .file_name()
                .is_some_and(|name| name == "operations.v2.journal")
        {
            self.barrier()?;
        }
        Ok(())
    }

    pub(crate) fn replace(&self, path: &Path, bytes: Vec<u8>) -> Result<(), RunnerError> {
        self.enqueue(path, WriteKind::Replace(bytes))
    }

    fn enqueue(&self, path: &Path, write: WriteKind) -> Result<(), RunnerError> {
        self.sender()?
            .try_send(Command {
                write: Some((path.to_owned(), write)),
                reserved: None,
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

    pub(crate) fn reserve(&self, path: &Path) -> Result<Reservation, RunnerError> {
        let (complete, bytes) = mpsc::channel();
        self.sender()?
            .try_send(Command {
                write: None,
                answer: None,
                reserved: Some(Reserved {
                    path: path.to_owned(),
                    bytes,
                }),
            })
            .map_err(|error| match error {
                mpsc::TrySendError::Full(_) => RunnerError::refused(
                    "durable_writer_busy",
                    "the durable writer's bounded queue is full",
                ),
                mpsc::TrySendError::Disconnected(_) => refused("the writer has ended"),
            })?;
        Ok(Reservation {
            complete: Some(complete),
        })
    }

    /// Called after releasing the table; a fence also observes an earlier write failure.
    pub(crate) fn barrier(&self) -> Result<(), RunnerError> {
        let (answer, answered) = mpsc::channel();
        self.sender()?
            .send(Command {
                write: None,
                reserved: None,
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

fn run(
    receiver: &mpsc::Receiver<Command>,
    fault: &Mutex<Option<String>>,
    #[cfg(test)] counts: &Mutex<BTreeMap<PathBuf, Count>>,
) {
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
            None => {
                #[cfg(test)]
                {
                    batch(&mut commands, counts)
                }
                #[cfg(not(test))]
                {
                    batch(&mut commands)
                }
            }
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

fn batch(
    commands: &mut [Command],
    #[cfg(test)] counts: &Mutex<BTreeMap<PathBuf, Count>>,
) -> ResultLine {
    let mut writes: BTreeMap<PathBuf, WriteKind> = BTreeMap::new();
    for command in commands {
        if let Some(Reserved { path, bytes }) = command.reserved.take() {
            let bytes = bytes
                .recv()
                .map_err(|error| format!("journal_batch_dropped: {error}"))??;
            command.write = Some((path, WriteKind::Append(bytes)));
        }
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
                .and_then(|mut file| {
                    file.write_all(&bytes)?;
                    #[cfg(test)]
                    {
                        let mut counts = counts
                            .lock()
                            .map_err(|error| std::io::Error::other(error.to_string()))?;
                        let count = counts.entry(path.clone()).or_default();
                        count.syncs = count
                            .syncs
                            .checked_add(1)
                            .ok_or_else(|| std::io::Error::other("sync count overflows"))?;
                    }
                    file.sync_data()
                }),
            WriteKind::Replace(bytes) => {
                #[cfg(test)]
                let started = std::time::Instant::now();
                let result = crate::state::replace(&path, &bytes);
                #[cfg(test)]
                {
                    let mut counts = counts.lock().map_err(|error| error.to_string())?;
                    counts.entry(path.clone()).or_default().replacement = Some(started.elapsed());
                }
                result
            }
        };
        result.map_err(|error| format!("{}: {error}", path.display()))?;
    }
    Ok(())
}

#[cfg(test)]
impl Writer {
    pub(crate) fn sync_count(&self, path: &Path) -> Result<u64, RunnerError> {
        Ok(self
            .0
            .counts
            .lock()
            .map_err(refused)?
            .get(path)
            .map_or(0, |count| count.syncs))
    }
    pub(crate) fn replacement_cost(&self, path: &Path) -> Result<std::time::Duration, RunnerError> {
        self.0
            .counts
            .lock()
            .map_err(refused)?
            .get(path)
            .and_then(|count| count.replacement)
            .ok_or_else(|| refused("replacement was not measured"))
    }
    pub(crate) fn serial_appends(&self) {
        self.0
            .serial
            .store(true, std::sync::atomic::Ordering::SeqCst);
    }
}

pub(crate) struct Reservation {
    complete: Option<mpsc::Sender<Result<Vec<u8>, String>>>,
}
impl Reservation {
    pub(crate) fn commit(mut self, bytes: Vec<u8>) -> Result<(), RunnerError> {
        self.complete
            .take()
            .ok_or_else(|| refused("journal batch was already completed"))?
            .send(Ok(bytes))
            .map_err(refused)
    }
}
impl Drop for Reservation {
    fn drop(&mut self) {
        if let Some(complete) = self.complete.take()
            && let Err(error) = complete.send(Err(
                "journal_batch_dropped: a reserved decision was not committed".to_owned(),
            ))
        {
            crate::error::said(&format!("journal_batch_drop_failed: {error}"));
        }
    }
}
