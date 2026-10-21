#![cfg(test)]
//! A stalled composer may read the whole input burst together.

use crate::Sessions;
use crate::operations::{Operation, OperationRequest, OperationState};
use crate::tracking::Harness;
use std::error::Error;
use std::io::{self, Write};
use std::sync::{Arc, Mutex, mpsc};

struct Composer {
    pending: Vec<u8>,
    submitted: mpsc::Sender<Option<String>>,
    release: mpsc::Receiver<()>,
    enter: bool,
}

impl Write for Composer {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.pending.extend_from_slice(bytes);
        if bytes.ends_with(b"\r") {
            self.enter = true;
            // An explicit paste is one composer event. Raw burst characters
            // followed by Enter are a pasted newline, not a submission.
            let submitted = self
                .pending
                .strip_prefix(b"\x1b[200~")
                .and_then(|text| text.strip_suffix(b"\x1b[201~\r"))
                .map(|text| String::from_utf8_lossy(text).into_owned());
            self.submitted.send(submitted).map_err(io::Error::other)?;
        }
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        if self.enter {
            self.release.recv().map_err(io::Error::other)?;
        }
        Ok(())
    }
}

fn wake_is_submitted(harness: Harness, direct: bool) -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let sessions = Sessions::open(dir.path(), 4096)?;
    sessions.start(serde_json::from_value(serde_json::json!({
        "session":"composer", "program":"/bin/cat", "arguments":[], "directory":"/",
        "environment":{}, "columns":80, "rows":24
    }))?)?;
    let (submitted, received) = mpsc::channel();
    let (release, released) = mpsc::channel();
    let writes = Arc::new(Mutex::new(Composer {
        pending: Vec::new(),
        submitted,
        release: released,
        enter: false,
    }));
    let original = {
        let mut table = sessions.lock()?;
        let session = table.sessions.get_mut("composer").ok_or("no session")?;
        let launch = session.launch.as_mut().ok_or("no launch")?;
        launch.config = Some(serde_json::from_value(serde_json::json!({
            "files":[],"argument_files":{},"environment_paths":{},
            "working_directory":false,"harness":harness
        }))?);
        std::mem::replace(
            &mut session.live("composer")?.writer,
            crate::input::Input::new(Box::new(ComposerWriter(Arc::clone(&writes)))),
        )
    };
    let text = "Wake this agent\nRead the queued work";
    let response = if direct {
        let key = lys_core::Ed25519Identity::load_or_generate(&dir.path().join("server.key"))?;
        let server = key.public_key_bytes();
        let greeting = crate::protocol::Greeting::fresh("21");
        let act = crate::protocol::Act::Input {
            session: "composer".to_owned(),
            text: text.to_owned(),
            enter: true,
        };
        let line = crate::protocol::sign_request(&key, &greeting, &act)?;
        let (answer, answered) = mpsc::channel();
        let owner = Arc::clone(&sessions);
        std::thread::spawn(move || {
            let result = crate::socket::dispatch(
                &owner,
                &server,
                &greeting,
                &line,
                &std::sync::atomic::AtomicBool::new(false),
            );
            if let Err(error) = answer.send(result) {
                panic!("answer receiver ended: {error}");
            }
        });
        Some(answered)
    } else {
        let mut table = sessions.lock()?;
        super::accept(
            &mut table,
            Operation {
                operation: "composer-notice".to_owned(),
                session: "composer".to_owned(),
                request: OperationRequest::Notice {
                    text: text.to_owned(),
                },
            },
        )?;
        None
    };
    let submitted = received.recv()?;
    let pending = match &response {
        Some(response) => matches!(response.try_recv(), Err(mpsc::TryRecvError::Empty)),
        None => sessions.outcome("composer-notice")?.state == OperationState::Delivering,
    };
    release.send(())?;
    let delivered = if let Some(response) = response {
        matches!(response.recv()?, crate::protocol::Answer::Delivered { .. })
    } else {
        sessions
            .until_any(&std::sync::atomic::AtomicBool::new(false), |table| {
                table
                    .operations
                    .get("composer-notice")
                    .filter(|held| held.state != OperationState::Delivering)
                    .cloned()
            })?
            .state
            == OperationState::Delivered
    };
    sessions.stop_all()?;
    drop(original);
    assert!(pending, "delivery was reported before Enter flushed");
    assert!(delivered, "submitted input was not delivered");
    assert_eq!(submitted.as_deref(), Some(text));
    Ok(())
}

struct ComposerWriter(Arc<Mutex<Composer>>);
impl Write for ComposerWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0
            .lock()
            .map_err(|error| io::Error::other(error.to_string()))?
            .write(bytes)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.0
            .lock()
            .map_err(|error| io::Error::other(error.to_string()))?
            .flush()
    }
}

#[test]
fn codex_composer_submits_a_wake_before_delivery_is_recorded() -> Result<(), Box<dyn Error>> {
    wake_is_submitted(Harness::Codex, false)
}

#[test]
fn claude_composer_submits_a_wake_before_delivery_is_recorded() -> Result<(), Box<dyn Error>> {
    wake_is_submitted(Harness::ClaudeCode, false)
}

#[test]
fn codex_direct_wake_submits_before_answering_delivered() -> Result<(), Box<dyn Error>> {
    wake_is_submitted(Harness::Codex, true)
}

#[test]
fn claude_direct_wake_submits_before_answering_delivered() -> Result<(), Box<dyn Error>> {
    wake_is_submitted(Harness::ClaudeCode, true)
}
