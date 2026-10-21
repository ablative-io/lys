#![cfg(test)]
use crate::harness_control::Settings;
use crate::harness_control::events::{Binding, Sessions, Transport};
use crate::input::Input;
use crate::operations::{Operation, OperationRequest, OperationState};
use crate::protocol::Launch;
use crate::tracking_store::Body;
use serde_json::{Value, json};
use std::error::Error;
use std::io::Write;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, mpsc};
type Result<T = ()> = std::result::Result<T, Box<dyn Error>>;

struct Pipe(mpsc::Sender<Vec<u8>>);
impl Write for Pipe {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.send(bytes.to_vec()).map_err(std::io::Error::other)?;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
struct Terminal(Arc<AtomicUsize>);
impl Write for Terminal {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
struct Cleanup(Arc<Sessions>);
impl Drop for Cleanup {
    fn drop(&mut self) {
        if let Err(error) = self.0.stop_all() {
            if std::thread::panicking() {
                crate::error::said(&format!("fixture cleanup failed: {error}"));
            } else {
                panic!("fixture cleanup failed: {error}");
            }
        }
    }
}
fn observe(sessions: &Sessions, source: &Binding, value: &Value) -> Result {
    let mut table = sessions.lock()?;
    let update =
        crate::harness_control::events::runtime(&mut table, &source.session, source.generation)?
            .controller
            .ingest(source, value)?;
    crate::harness_control::events::apply(&mut table, &source.session, source.generation, update)?;
    drop(table);
    sessions.writer.barrier()?;
    sessions.wake();
    Ok(())
}

struct Service {
    sessions: Arc<Sessions>,
    ended: Arc<std::sync::atomic::AtomicBool>,
    worker: Option<std::thread::JoinHandle<std::result::Result<(), crate::error::RunnerError>>>,
}

impl Service {
    fn start(sessions: &Arc<Sessions>, source: &Binding) -> Result<Self> {
        let ended = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let done = Arc::clone(&ended);
        let owner = Arc::clone(sessions);
        let source = source.clone();
        let worker = std::thread::Builder::new()
            .name("fixture-boundary-service".to_owned())
            .spawn(move || {
                let mut answered = None;
                loop {
                    let boundary = match owner.until_any(&done, |table| {
                        table
                            .sessions
                            .get(&source.session)
                            .and_then(|session| session.live.as_ref())
                            .and_then(|live| live.control.as_ref())
                            .and_then(|control| control.controller.control_status().boundary)
                            .filter(|boundary| answered.as_ref() != Some(boundary))
                    }) {
                        Ok(boundary) => boundary,
                        Err(error)
                            if error.name() == "caller_left" && done.load(Ordering::SeqCst) =>
                        {
                            return Ok(());
                        }
                        Err(error) => return Err(error),
                    };
                    owner.apply_boundary_reply(Operation {
                        operation: format!("fixture-{boundary}"),
                        session: source.session.clone(),
                        request: OperationRequest::BoundaryReply {
                            reply: crate::harness_control::BoundaryReply {
                                generation: source.generation,
                                boundary: Some(boundary.clone()),
                                context: crate::harness_control::ContextDecision::Released,
                                reminders: Vec::new(),
                            },
                        },
                    })?;
                    answered = Some(boundary);
                }
            })?;
        Ok(Self {
            sessions: Arc::clone(sessions),
            ended,
            worker: Some(worker),
        })
    }
}

impl Drop for Service {
    fn drop(&mut self) {
        let table = match self.sessions.lock() {
            Ok(table) => table,
            Err(error) => panic!("fixture boundary service lock failed: {error}"),
        };
        self.ended.store(true, Ordering::SeqCst);
        drop(table);
        self.sessions.wake();
        if let Some(worker) = self.worker.take() {
            match worker.join() {
                Ok(Ok(())) => {}
                Ok(Err(error)) => panic!("fixture boundary service failed: {error}"),
                Err(reason) => panic!("fixture boundary service panicked: {reason:?}"),
            }
        }
    }
}
fn fixture(
    transport: Transport,
    human: bool,
    test: impl FnOnce(&Arc<Sessions>, &Binding, &mpsc::Receiver<Vec<u8>>, &AtomicUsize) -> Result,
) -> Result {
    controlled_fixture(transport, human, true, test)
}
fn controlled_fixture(
    transport: Transport,
    human: bool,
    automatic: bool,
    test: impl FnOnce(&Arc<Sessions>, &Binding, &mpsc::Receiver<Vec<u8>>, &AtomicUsize) -> Result,
) -> Result {
    gated_fixture(transport, human, automatic, None, test)
}
fn gated_fixture(
    transport: Transport,
    human: bool,
    automatic: bool,
    gate: Option<mpsc::Receiver<()>>,
    test: impl FnOnce(&Arc<Sessions>, &Binding, &mpsc::Receiver<Vec<u8>>, &AtomicUsize) -> Result,
) -> Result {
    let directory = tempfile::tempdir()?;
    let sessions = Sessions::open(directory.path(), 4096)?;
    let cleanup = Cleanup(Arc::clone(&sessions));
    sessions.start(Launch {
        session: "fixture".to_owned(),
        program: "/bin/sh".to_owned(),
        arguments: vec!["-c".to_owned(), "cat".to_owned()],
        directory: directory.path().display().to_string(),
        environment: std::collections::BTreeMap::new(),
        config: None,
        columns: 80,
        rows: 24,
        rotation: None,
        policy: None,
    })?;
    let executable = crate::harness_control::process::executable(std::path::Path::new("/bin/sh"))?;
    let binding = {
        let table = sessions.lock()?;
        let session = table
            .sessions
            .get("fixture")
            .ok_or("fixture session missing")?;
        Binding {
            session: "fixture".to_owned(),
            generation: session.generation,
            leader: session
                .leader_start
                .clone()
                .ok_or("fixture leader missing")?,
            conversation: "conversation".to_owned(),
            entry: executable.clone(),
            harness: executable,
            harness_version: "9.8.7".to_owned(),
            adapter: match transport {
                Transport::Claude => "claude-stream-json/1",
                Transport::Codex => "codex-app-server/1",
                Transport::Pty => return Err("fixture requires a managed adapter".into()),
            }
            .to_owned(),
        }
    };
    let (sender, frames) = mpsc::channel();
    let terminal = Arc::new(AtomicUsize::new(0));
    let (runtime, bridge) = crate::harness_control::events::live(
        binding.clone(),
        transport,
        match gate {
            Some(release) => Box::new(PausedPipe {
                frames: sender,
                release,
            }),
            None => Box::new(Pipe(sender)),
        },
        Arc::downgrade(&sessions),
    )?;
    let terminal_input = {
        let mut table = sessions.lock()?;
        let session = table
            .sessions
            .get_mut("fixture")
            .ok_or("fixture session missing")?;
        session.managed = Some(Settings {
            transport,
            conversation: "conversation".to_owned(),
            resume: false,
        });
        let live = session.live.as_mut().ok_or("fixture process missing")?;
        live.control = Some(runtime);
        std::mem::replace(
            &mut live.writer,
            if human {
                bridge
            } else {
                Input::new(Box::new(Terminal(Arc::clone(&terminal))))
            },
        )
    };
    initialize_transport(&sessions, &binding, &frames, transport)?;
    let service = if automatic {
        Some(Service::start(&sessions, &binding)?)
    } else {
        None
    };
    let result = test(&sessions, &binding, &frames, &terminal);
    drop(service);
    drop(cleanup);
    drop(terminal_input);
    result
}
fn operation(id: &str, request: OperationRequest) -> Operation {
    Operation {
        operation: id.to_owned(),
        session: "fixture".to_owned(),
        request,
    }
}

struct PausedPipe {
    frames: mpsc::Sender<Vec<u8>>,
    release: mpsc::Receiver<()>,
}
impl Write for PausedPipe {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.frames
            .send(bytes.to_vec())
            .map_err(std::io::Error::other)?;
        self.release.recv().map_err(std::io::Error::other)?;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
struct Release(mpsc::Sender<()>);
impl Drop for Release {
    fn drop(&mut self) {
        if let Err(error) = self.0.send(()) {
            crate::error::said(&format!("fixture_release_failed: {error}"));
        }
    }
}

#[path = "behavior_0.rs"]
mod behavior_0;
#[path = "behavior_1.rs"]
mod behavior_1;
#[path = "notices.rs"]
mod notices;

fn initialize_transport(
    sessions: &Arc<Sessions>,
    binding: &Binding,
    frames: &mpsc::Receiver<Vec<u8>>,
    transport: Transport,
) -> Result {
    match transport {
        Transport::Claude => {
            let initialize = {
                let mut table = sessions.lock()?;
                crate::harness_control::events::runtime(&mut table, "fixture", binding.generation)?
                    .controller
                    .bootstrap()
            };
            let correlation = initialize
                .dispatches
                .first()
                .ok_or("fixture initialize missing")?
                .frame["request_id"]
                .as_str()
                .ok_or("fixture initialize uncorrelated")?;
            observe(
                sessions,
                binding,
                &json!({"type":"control_response",
                "response":{"subtype":"success","request_id":correlation,"response":{}}}),
            )?;
            let uuid = {
                let mut table = sessions.lock()?;
                let controller = &mut crate::harness_control::events::runtime(
                    &mut table,
                    "fixture",
                    binding.generation,
                )?
                .controller;
                let pending = crate::harness_control::events::Pending::new(
                    "fixture-initialization".to_owned(),
                    crate::harness_control::events::Kind::Human,
                    "first turn".to_owned(),
                );
                let uuid = pending.uuid.clone();
                controller.enqueue(pending)?;
                let status = controller.control_status();
                let update = controller.boundary_reply(&crate::harness_control::BoundaryReply {
                    generation: status.generation,
                    boundary: status.boundary,
                    context: crate::harness_control::ContextDecision::Released,
                    reminders: Vec::new(),
                })?;
                assert_eq!(update.dispatches.len(), 1);
                assert_eq!(
                    update.dispatches[0].frame["message"]["content"],
                    "first turn"
                );
                uuid
            };
            observe(
                sessions,
                binding,
                &json!({"type":"system","subtype":"init",
                "session_id":"conversation","claude_code_version":"9.8.7","slash_commands":["compact"]}),
            )?;
            observe(
                sessions,
                binding,
                &json!({"type":"user","session_id":"conversation","uuid":uuid,
                "parent_tool_use_id":null,"message":{"role":"user","content":"first turn"}}),
            )?;
            observe(
                sessions,
                binding,
                &json!({"type":"result","session_id":"conversation",
                "uuid":"fixture-initialization-result","is_error":false}),
            )?;
        }
        Transport::Codex => {
            observe(
                sessions,
                binding,
                &json!({"id":"lys-initialize","result":{"userAgent":"lys-runner/9.8.7 (fixture)"}}),
            )?;
            for expected in ["initialized", "thread/resume"] {
                let frame: Value = serde_json::from_slice(&frames.recv()?)?;
                assert_eq!(frame["method"], expected);
            }
            observe(
                sessions,
                binding,
                &json!({"id":"lys-thread","result":{"thread":{"id":"conversation","status":{"type":"idle"}}}}),
            )?;
        }
        Transport::Pty => return Err("fixture requires a managed adapter".into()),
    }
    Ok(())
}
