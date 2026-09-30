#![cfg(test)]
//! A full input pipe must leave the session table available to another session.

use std::collections::BTreeMap;
use std::error::Error;
use std::io::{self, Read, Write};
use std::os::unix::net::UnixStream;
use std::sync::{Arc, TryLockError, mpsc};

use super::Sessions;
use crate::operations::{Operation, OperationRequest, OperationState};
use crate::protocol::Launch;

struct GatedPipe {
    pipe: UnixStream,
    blocked: mpsc::SyncSender<()>,
    resume: mpsc::Receiver<()>,
    entered: bool,
}

impl Write for GatedPipe {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if !self.entered {
            match self.pipe.write(bytes) {
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {}
                other => {
                    return Err(io::Error::other(format!(
                        "the input pipe was not full: {other:?}"
                    )));
                }
            }
            self.blocked.send(()).map_err(io::Error::other)?;
            self.resume.recv().map_err(io::Error::other)?;
            self.pipe.set_nonblocking(false)?;
            self.entered = true;
        }
        self.pipe.write(bytes)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.pipe.flush()
    }
}

fn launch(session: &str) -> Launch {
    Launch {
        session: session.to_owned(),
        program: "/bin/cat".to_owned(),
        arguments: Vec::new(),
        directory: "/".to_owned(),
        environment: BTreeMap::new(),
        config: None,
        columns: 80,
        rows: 24,
        rotation: None,
        policy: None,
    }
}

#[test]
fn a_full_unwatched_input_pipe_leaves_the_second_session_answering() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let sessions = Sessions::open(dir.path(), 4096)?;
    sessions.start(launch("blocked"))?;
    sessions.start(launch("second"))?;
    let result = exercise(&sessions, None);
    sessions.stop_all();
    result
}

#[test]
fn an_unwatched_operation_leaves_the_second_session_answering() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let sessions = Sessions::open(dir.path(), 4096)?;
    sessions.start(launch("blocked"))?;
    sessions.start(launch("second"))?;
    let result = exercise(
        &sessions,
        Some(OperationRequest::Notice {
            text: "x".to_owned(),
        }),
    );
    sessions.stop_all();
    result
}

#[test]
fn an_unwatched_compaction_keeps_an_early_harness_confirmation() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let sessions = Sessions::open(dir.path(), 4096)?;
    sessions.start(launch("blocked"))?;
    sessions.start(launch("second"))?;
    let result = exercise(
        &sessions,
        Some(OperationRequest::Compact {
            text: "x".to_owned(),
        }),
    );
    sessions.stop_all();
    result
}

#[test]
fn an_unwatched_reminder_leaves_the_second_session_answering() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let sessions = Sessions::open(dir.path(), 4096)?;
    sessions.start(launch("blocked"))?;
    sessions.start(launch("second"))?;
    let result = exercise(
        &sessions,
        Some(OperationRequest::Reminder {
            text: "x".to_owned(),
        }),
    );
    sessions.stop_all();
    result
}

fn exercise(
    sessions: &Arc<Sessions>,
    operation: Option<OperationRequest>,
) -> Result<(), Box<dyn Error>> {
    let (mut pipe, mut reader) = UnixStream::pair()?;
    pipe.set_nonblocking(true)?;
    let buffer = [0_u8; 8192];
    let mut filled = 0;
    loop {
        match pipe.write(&buffer) {
            Ok(0) => return Err("input pipe closed before becoming full".into()),
            Ok(written) => filled += written,
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => break,
            Err(error) => return Err(error.into()),
        }
    }
    assert!(filled > 0);
    let (blocked, pipe_full) = mpsc::sync_channel(1);
    let (release, resume) = mpsc::sync_channel(1);
    let terminal_input = {
        let mut table = sessions.lock();
        let live = table
            .sessions
            .get_mut("blocked")
            .ok_or("blocked session missing")?
            .live("blocked")?;
        // Dropping the terminal input sends EOF; keep it alive through delivery.
        let terminal_input = std::mem::replace(
            &mut live.writer,
            crate::input::Input::new(Box::new(GatedPipe {
                pipe,
                blocked,
                resume,
                entered: false,
            })),
        );
        if operation.is_some() {
            let tracking = crate::tracking::Tracking {
                harness: crate::tracking::Harness::ClaudeCode,
                adapter: crate::tracking::CLAUDE_ADAPTER.to_owned(),
                version: "2.1.281".to_owned(),
                config_home: "/".to_owned(),
                context_window: 200_000,
                profile_version: 1,
                account: None,
                requires_pre_tool: false,
            };
            tracking.checked()?;
            let session = table.sessions.get_mut("blocked").ok_or("session missing")?;
            session.guard.tracking = Some(tracking);
            session.guard.idle = false;
        }
        terminal_input
    };
    let compact = matches!(operation, Some(OperationRequest::Compact { .. }));
    let writer = if let Some(request) = operation {
        let mut table = sessions.lock();
        let accepted = crate::operations::accept(
            &mut table,
            Operation {
                operation: "notice".to_owned(),
                session: "blocked".to_owned(),
                request,
            },
        )?;
        assert_eq!(accepted.state, OperationState::Accepted);
        drop(table);
        sessions.collect(
            "blocked",
            &crate::peer::Collected::Hook {
                event: "Stop".to_owned(),
                input: serde_json::json!({}),
            },
        )?;
        None
    } else {
        let input_owner = Arc::clone(sessions);
        Some(std::thread::spawn(move || {
            input_owner.write("blocked", b"x")
        }))
    };
    pipe_full.recv()?;
    if compact {
        let mut table = sessions.lock();
        crate::operations::compacting(&mut table, "blocked");
        assert_eq!(
            table
                .operations
                .get("notice")
                .ok_or("operation missing")?
                .state,
            OperationState::Delivering
        );
    }
    let available = match sessions.table.try_lock() {
        Ok(table) => {
            drop(table);
            true
        }
        Err(TryLockError::WouldBlock) => false,
        Err(TryLockError::Poisoned(error)) => return Err(error.to_string().into()),
    };
    let second = if available {
        Some(sessions.status(Some("second")))
    } else {
        None
    };
    let mut drained = vec![0; filled];
    reader.read_exact(&mut drained)?;
    release.send(())?;
    if let Some(writer) = writer {
        writer.join().expect("input writer panicked")?;
    } else {
        let outcome = sessions.until_any(&std::sync::atomic::AtomicBool::new(false), |table| {
            table
                .operations
                .get("notice")
                .filter(|outcome| outcome.state != OperationState::Delivering)
                .cloned()
        })?;
        assert_eq!(
            outcome.state,
            if compact {
                OperationState::Confirmed
            } else {
                OperationState::Delivered
            }
        );
    }
    let answer = second.unwrap_or_else(|| sessions.status(Some("second")))?;
    assert_eq!(answer.sessions.len(), 1);
    assert_eq!(answer.sessions[0].session, "second");
    assert!(
        available,
        "the full input pipe held the shared table and stopped the second session answering"
    );
    drop(terminal_input);
    Ok(())
}

struct PartialWriter {
    entered: mpsc::Sender<()>,
    resume: mpsc::Receiver<()>,
    collected: mpsc::Sender<Vec<u8>>,
    bytes: Vec<u8>,
}

impl Write for PartialWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self.bytes.is_empty() {
            self.entered.send(()).map_err(io::Error::other)?;
            self.resume.recv().map_err(io::Error::other)?;
        }
        self.bytes.push(bytes[0]);
        Ok(1)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.collected
            .send(self.bytes.clone())
            .map_err(io::Error::other)
    }
}

#[test]
fn input_spans_finish_in_order_without_interleaving_partial_writes() -> Result<(), Box<dyn Error>> {
    let (entered, waiting) = mpsc::channel();
    let (release, resume) = mpsc::channel();
    let (collected, written) = mpsc::channel();
    let input = crate::input::Input::new(Box::new(PartialWriter {
        entered,
        resume,
        collected,
        bytes: Vec::new(),
    }));
    let (complete, completed) = mpsc::channel();
    let first = complete.clone();
    input.submit(b"ab".to_vec(), move |result| {
        first
            .send((1, result))
            .expect("first completion receiver left");
    })?;
    waiting.recv()?;
    input.submit(b"cd".to_vec(), move |result| {
        complete
            .send((2, result))
            .expect("second completion receiver left");
    })?;
    release.send(())?;
    let (first, result) = completed.recv()?;
    result?;
    assert_eq!(first, 1);
    let (second, result) = completed.recv()?;
    result?;
    assert_eq!(second, 2);
    assert_eq!(written.recv()?, b"ab");
    assert_eq!(written.recv()?, b"abcd");
    Ok(())
}

struct BrokenWriter;

impl Write for BrokenWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        Err(io::Error::new(
            io::ErrorKind::BrokenPipe,
            format!("cannot write {} bytes", bytes.len()),
        ))
    }

    fn flush(&mut self) -> io::Result<()> {
        Err(io::Error::other("cannot flush a broken writer"))
    }
}

#[test]
fn input_failure_is_named_and_answers_the_waiter() {
    let input = crate::input::Input::new(Box::new(BrokenWriter));
    match input.write(b"x".to_vec()) {
        Err(crate::RunnerError::Refused { refusal, words, .. }) => {
            assert_eq!(refusal, "write_failed");
            assert!(words.contains("cannot write 1 bytes"));
        }
        other => panic!("expected a named write failure, got {other:?}"),
    }
}
