use crate::Sessions;
use crate::operations::{Operation, OperationRequest, OperationState};
use std::error::Error;
use std::io::{self, Write};
use std::sync::{Arc, Mutex};
use std::time::Instant;

#[derive(Default)]
struct Writes {
    bytes: Vec<Vec<u8>>,
    written: Vec<Instant>,
    flushed: Vec<Instant>,
}

struct Recording(Arc<Mutex<Writes>>);
impl Write for Recording {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let mut writes = self
            .0
            .lock()
            .map_err(|error| io::Error::other(error.to_string()))?;
        writes.bytes.push(bytes.to_vec());
        writes.written.push(Instant::now());
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        self.0
            .lock()
            .map_err(|error| io::Error::other(error.to_string()))?
            .flushed
            .push(Instant::now());
        Ok(())
    }
}

#[test]
fn seatfix_text_then_enter_are_separate_writes_with_one_durable_delivery()
-> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let sessions = Sessions::open(dir.path(), 4096)?;
    let launch = serde_json::from_value(serde_json::json!({
        "session":"seatfix", "program":"/bin/cat", "arguments":[], "directory":"/",
        "environment":{}, "columns":80, "rows":24
    }))?;
    sessions.start(launch)?;
    let writes = Arc::new(Mutex::new(Writes::default()));
    let original = {
        let mut table = sessions.lock()?;
        let live = table
            .sessions
            .get_mut("seatfix")
            .ok_or("no session")?
            .live("seatfix")?;
        std::mem::replace(
            &mut live.writer,
            crate::input::Input::new(Box::new(Recording(Arc::clone(&writes)))),
        )
    };
    let operation = Operation {
        operation: "seatfix-notice".to_owned(),
        session: "seatfix".to_owned(),
        request: OperationRequest::Notice {
            text: "seat message".to_owned(),
        },
    };
    let result = (|| {
        let mut table = sessions.lock()?;
        super::accept(&mut table, operation.clone())?;
        drop(table);
        let receipt = sessions.until_any(&std::sync::atomic::AtomicBool::new(false), |table| {
            table
                .operations
                .get("seatfix-notice")
                .filter(|held| held.state == OperationState::Delivered)
                .cloned()
        })?;
        let mut table = sessions.lock()?;
        assert_eq!(
            super::accept(&mut table, operation)?.state,
            OperationState::Delivered
        );
        assert_eq!(receipt.text.as_ref().ok_or("no text digest")?.length, 12);
        drop(table);
        assert_eq!(
            writes.lock().map_err(|error| error.to_string())?.bytes,
            vec![b"seat message".to_vec(), vec![13]]
        );
        let writes = writes.lock().map_err(|error| error.to_string())?;
        assert!(writes.written[1] >= writes.flushed[0]);
        Ok::<_, Box<dyn Error>>(())
    })();
    sessions.stop_all()?;
    drop(original);
    result?;
    drop(sessions);
    let recovered = Sessions::open(dir.path(), 4096)?;
    let table = recovered.lock()?;
    assert_eq!(
        table
            .operations
            .get("seatfix-notice")
            .ok_or("no durable receipt")?
            .state,
        OperationState::Delivered
    );
    Ok(())
}

#[test]
fn session_ending_in_enter_gap_refuses_delivery_without_writing_enter() -> Result<(), Box<dyn Error>>
{
    let dir = tempfile::tempdir()?;
    let sessions = Sessions::open(dir.path(), 4096)?;
    let launch = serde_json::from_value(serde_json::json!({
        "session":"enter-gap", "program":"/bin/cat", "arguments":[], "directory":"/",
        "environment":{}, "columns":80, "rows":24
    }))?;
    sessions.start(launch)?;
    let writes = Arc::new(Mutex::new(Writes::default()));
    let original = {
        let mut table = sessions.lock()?;
        let live = table
            .sessions
            .get_mut("enter-gap")
            .ok_or("no session")?
            .live("enter-gap")?;
        std::mem::replace(
            &mut live.writer,
            crate::input::Input::new(Box::new(Recording(Arc::clone(&writes)))),
        )
    };
    let (gap_reached, resume) = super::enter::pause_gap("gap-notice")?;
    let result = (|| {
        {
            let mut table = sessions.lock()?;
            super::accept(
                &mut table,
                Operation {
                    operation: "gap-notice".to_owned(),
                    session: "enter-gap".to_owned(),
                    request: OperationRequest::Notice {
                        text: "long seat message ".repeat(14),
                    },
                },
            )?;
        }
        let phase = sessions.until_any(&std::sync::atomic::AtomicBool::new(false), |table| {
            table
                .operations
                .get("gap-notice")
                .filter(|held| {
                    super::enter::waiting(held) || held.state != OperationState::Delivering
                })
                .cloned()
        })?;
        if !super::enter::waiting(&phase) {
            return Err(format!("enter_gap_missing: operation reached {:?}", phase.state).into());
        }
        gap_reached.recv()?;
        // The callback holds its gap until the session's real exit is recorded.
        sessions.end("enter-gap", &std::sync::atomic::AtomicBool::new(false))?;
        resume.send(())?;
        let ended = sessions.until_any(&std::sync::atomic::AtomicBool::new(false), |table| {
            table
                .operations
                .get("gap-notice")
                .filter(|held| held.state != OperationState::Delivering)
                .cloned()
        })?;
        assert_eq!(ended.state, OperationState::Refused);
        assert!(
            (ended.words.contains("session_unknown") || ended.words.contains("runner_stopping")),
            "{}",
            ended.words
        );
        assert_eq!(
            writes
                .lock()
                .map_err(|error| error.to_string())?
                .bytes
                .len(),
            1
        );
        Ok::<_, Box<dyn Error>>(())
    })();
    sessions.stop_all()?;
    drop(original);
    result?;
    drop(sessions);
    let recovered = Sessions::open(dir.path(), 4096)?;
    let table = recovered.lock()?;
    let held = table
        .operations
        .get("gap-notice")
        .ok_or("no durable refusal")?;
    assert_eq!(held.state, OperationState::Refused);
    assert!(held.words.contains("session_unknown") || held.words.contains("runner_stopping"));
    Ok(())
}

#[test]
fn empty_text_submits_enter_on_its_flush_event() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let sessions = Sessions::open(dir.path(), 4096)?;
    let launch = serde_json::from_value(serde_json::json!({
        "session":"empty-text", "program":"/bin/cat", "arguments":[], "directory":"/",
        "environment":{}, "columns":80, "rows":24
    }))?;
    sessions.start(launch)?;
    let writes = Arc::new(Mutex::new(Writes::default()));
    let original = {
        let mut table = sessions.lock()?;
        let live = table
            .sessions
            .get_mut("empty-text")
            .ok_or("no session")?
            .live("empty-text")?;
        std::mem::replace(
            &mut live.writer,
            crate::input::Input::new(Box::new(Recording(Arc::clone(&writes)))),
        )
    };
    let result = (|| {
        {
            let mut table = sessions.lock()?;
            super::accept(
                &mut table,
                Operation {
                    operation: "empty-notice".to_owned(),
                    session: "empty-text".to_owned(),
                    request: OperationRequest::Notice {
                        text: String::new(),
                    },
                },
            )?;
        }
        let receipt = sessions.until_any(&std::sync::atomic::AtomicBool::new(false), |table| {
            table
                .operations
                .get("empty-notice")
                .filter(|held| {
                    held.state != OperationState::Delivering || super::enter::waiting(held)
                })
                .cloned()
        })?;
        assert_eq!(receipt.state, OperationState::Delivered);
        assert_eq!(
            writes.lock().map_err(|error| error.to_string())?.bytes,
            vec![vec![13]]
        );
        Ok::<_, Box<dyn Error>>(())
    })();
    sessions.stop_all()?;
    drop(original);
    result
}
