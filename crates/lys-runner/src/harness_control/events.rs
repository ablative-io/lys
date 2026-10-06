//! One reader projects managed evidence into the existing feed and receipts.

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Weak, mpsc};

use super::{Binding, Controller, Kind, Pending, Transport, Update};
use crate::error::RunnerError;
use crate::input::Input;
use crate::operations::{OperationRequest, OperationState};
use crate::session::{Sessions, Table, now_ms};
use crate::tracking_store::{Body, Commit};

pub(crate) struct Runtime {
    pub(crate) controller: Controller,
    pipe: Input,
    admissions: BTreeMap<String, mpsc::Sender<Result<(), RunnerError>>>,
    pub(super) left: Arc<AtomicBool>,
    pub(super) approval_active: bool,
    owner: Weak<Sessions>,
}

pub(crate) fn live(
    binding: Binding,
    transport: Transport,
    writer: Box<dyn Write + Send>,
    owner: Weak<Sessions>,
) -> Result<(Runtime, Input), RunnerError> {
    let bridge = Human {
        owner: Weak::clone(&owner),
        session: binding.session.clone(),
        generation: binding.generation,
    };
    let mut controller = Controller::new(binding, transport)?;
    controller.require_boundary_authority()?;
    let runtime = Runtime {
        controller,
        pipe: Input::new(writer),
        admissions: BTreeMap::new(),
        left: Arc::new(AtomicBool::new(false)),
        approval_active: false,
        owner,
    };
    Ok((runtime, Input::new(Box::new(bridge))))
}

impl Drop for Runtime {
    fn drop(&mut self) {
        self.left.store(true, Ordering::SeqCst);
        if let Some(owner) = self.owner.upgrade() {
            owner.wake();
        }
    }
}

struct Human {
    owner: Weak<Sessions>,
    session: String,
    generation: u64,
}

impl Write for Human {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        let sessions = self
            .owner
            .upgrade()
            .ok_or_else(|| std::io::Error::other("runner_stopping"))?;
        sessions
            .managed_input(&self.session, self.generation, bytes)
            .map_err(std::io::Error::other)?;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

pub(super) fn runtime<'a>(
    table: &'a mut Table,
    id: &str,
    generation: u64,
) -> Result<&'a mut Runtime, RunnerError> {
    let session = table
        .sessions
        .get_mut(id)
        .ok_or_else(|| RunnerError::refused("session_unknown", "managed session is not held"))?;
    if session.generation != generation || session.ended.is_some() {
        return Err(RunnerError::refused(
            "control_generation_changed",
            "managed process generation ended or changed",
        ));
    }
    session
        .live
        .as_mut()
        .and_then(|live| live.control.as_mut())
        .ok_or_else(|| {
            RunnerError::refused(
                "control_transport_unsupported",
                "session has no live managed pipe",
            )
        })
}

pub(crate) fn accepts(
    table: &mut Table,
    id: &str,
    request: &OperationRequest,
) -> Result<(), RunnerError> {
    if *request == OperationRequest::Stop {
        return Ok(());
    }
    let session = table
        .sessions
        .get(id)
        .ok_or_else(|| RunnerError::refused("session_unknown", "session is not held"))?;
    if session.managed.is_none() {
        return Ok(());
    }
    let generation = session.generation;
    if session.ending {
        return Err(RunnerError::refused(
            "session_ending",
            "managed session is ending",
        ));
    }
    if !runtime(table, id, generation)?.controller.ready() {
        return Err(RunnerError::refused(
            "control_source_unbound",
            "managed conversation has not been proved",
        ));
    }
    Ok(())
}

pub(crate) fn enqueue_operation(
    table: &mut Table,
    id: &str,
    operation: &str,
    request: &OperationRequest,
) -> Result<(), RunnerError> {
    let generation = table
        .sessions
        .get(id)
        .ok_or_else(|| RunnerError::refused("session_unknown", "session is not held"))?
        .generation;
    let kind = if matches!(
        request,
        OperationRequest::Compact { .. } | OperationRequest::ContextCompact { .. }
    ) {
        Kind::Compact
    } else {
        Kind::Reminder
    };
    let text = crate::operations::managed_take(table, id, operation)?;
    let pending = match request {
        OperationRequest::GoalReminder { reference, .. } => {
            Pending::for_goal(operation.to_owned(), text, reference.clone())
        }
        _ => Pending::new(operation.to_owned(), kind, text),
    };
    let control = &mut runtime(table, id, generation)?.controller;
    let update = if let OperationRequest::ContextCompact { crossing, .. } = request {
        if crossing != operation {
            return Err(RunnerError::refused(
                "control_crossing_changed",
                "the crossing does not name its operation",
            ));
        }
        control.context_compact(pending)?
    } else {
        control.enqueue(pending)?
    };
    match apply(table, id, generation, update) {
        Ok(()) => Ok(()),
        Err(error) => {
            let lost = runtime(table, id, generation)?.controller.disconnected();
            apply(table, id, generation, lost)?;
            Err(error)
        }
    }
}

pub(super) fn apply(
    table: &mut Table,
    id: &str,
    generation: u64,
    update: Update,
) -> Result<(), RunnerError> {
    for (operation, binding, uuid, turn) in update.admissions {
        if table.operations.get(&operation).is_some() {
            table
                .operations
                .observed(&operation, &binding, &uuid, turn)?;
        }
    }
    if !update.events.is_empty() {
        table.feed.append(
            id,
            now_ms(),
            update.events.into_iter().map(Body::Managed).collect(),
            Commit::default(),
        )?;
    }
    for receipt in update.receipts {
        if table.operations.get(&receipt.operation).is_some() {
            crate::operations::managed_state(
                table,
                &receipt.operation,
                receipt.state,
                receipt.reason,
            )?;
        } else if let Some(sender) = runtime(table, id, generation)?
            .admissions
            .remove(&receipt.operation)
        {
            let result = if receipt.state == OperationState::Confirmed {
                Ok(())
            } else {
                Err(RunnerError::refused(
                    "control_input_uncertain",
                    receipt.reason,
                ))
            };
            sender.send(result).map_err(|error| {
                RunnerError::refused(
                    "control_input_caller_left",
                    format!("input admission was not received: {error}"),
                )
            })?;
        }
    }
    for dispatch in update.dispatches {
        if table.sessions.get(id).is_some_and(|session| session.ending) {
            if table.operations.get(&dispatch.operation).is_some() {
                crate::operations::managed_state(
                    table,
                    &dispatch.operation,
                    OperationState::Refused,
                    "session_ending: prepared input was not written".to_owned(),
                )?;
            } else if let Some(sender) = runtime(table, id, generation)?
                .admissions
                .remove(&dispatch.operation)
            {
                sender
                    .send(Err(RunnerError::refused(
                        "session_ending",
                        "prepared input was not written",
                    )))
                    .map_err(|error| {
                        RunnerError::refused(
                            "control_input_caller_left",
                            format!("cancelled input was not received: {error}"),
                        )
                    })?;
            }
            let mut lost = runtime(table, id, generation)?.controller.disconnected();
            lost.receipts
                .retain(|receipt| receipt.operation != dispatch.operation);
            apply(table, id, generation, lost)?;
            continue;
        }
        if !dispatch.operation.is_empty() && table.operations.get(&dispatch.operation).is_some() {
            let (prepared, text) = runtime(table, id, generation)?
                .controller
                .preparation(&dispatch)?;
            table
                .operations
                .prepare(&dispatch.operation, prepared, text)?;
            table.operations.arm(&dispatch.operation)?;
        }
        let mut bytes = serde_json::to_vec(&dispatch.frame)
            .map_err(|error| RunnerError::refused("control_frame_invalid", error.to_string()))?;
        bytes.push(b'\n');
        let pipe = runtime(table, id, generation)?.pipe.clone();
        let owner = Weak::clone(&table.owner);
        let sessions = owner.upgrade().ok_or_else(|| {
            RunnerError::refused("runner_stopping", "runner ended before managed delivery")
        })?;
        let durable = sessions.writer.clone();
        let operation = dispatch.operation;
        let session_id = id.to_owned();
        pipe.submit_after(bytes, durable, move |result| {
            let Some(sessions) = owner.upgrade() else {
                crate::error::said("control_transport_lost: runner ended during a pipe write");
                return;
            };
            if let Err(error) =
                sessions.managed_written(&session_id, generation, &operation, result)
            {
                crate::error::said(&format!("control_delivery_uncertain: {error}"));
            }
        })?;
    }
    let idle = runtime(table, id, generation)?.controller.idle();
    if let Some(session) = table.sessions.get_mut(id) {
        session.guard.idle = idle;
        if let (Some(settings), Some(control)) = (
            &mut session.managed,
            session.live.as_ref().and_then(|live| live.control.as_ref()),
        ) {
            if settings.conversation != control.controller.binding.conversation {
                settings
                    .conversation
                    .clone_from(&control.controller.binding.conversation);
            }
        }
    }
    Ok(())
}

impl Sessions {
    fn managed_input(&self, id: &str, generation: u64, bytes: &[u8]) -> Result<(), RunnerError> {
        let text = std::str::from_utf8(bytes).map_err(|error| {
            RunnerError::refused(
                "control_input_invalid",
                format!("managed input must be UTF-8 text: {error}"),
            )
        })?;
        let text = text.strip_suffix('\r').unwrap_or(text);
        if text
            .chars()
            .any(|ch| ch.is_control() && ch != '\n' && ch != '\t')
        {
            return Err(RunnerError::refused(
                "control_input_invalid",
                "terminal control bytes are not a managed message",
            ));
        }
        let operation = format!(
            "human-{}",
            crate::protocol::hex(&rand::random::<[u8; 16]>())
        );
        let (sender, admission) = mpsc::channel();
        let mut table = self.lock()?;
        if table.sessions.get(id).is_some_and(|session| session.ending) {
            return Err(RunnerError::refused(
                "session_ending",
                "managed session is ending",
            ));
        }
        let held = runtime(&mut table, id, generation)?;
        let update = held.controller.enqueue(Pending::new(
            operation.clone(),
            Kind::Human,
            text.to_owned(),
        ))?;
        held.admissions.insert(operation, sender);
        let applied = apply(&mut table, id, generation, update);
        drop(table);
        if let Err(error) = applied {
            self.managed_lost(id, generation, &error.to_string())?;
            return Err(error);
        }
        self.writer.barrier()?;
        self.wake();
        admission.recv().map_err(|error| {
            RunnerError::refused(
                "control_input_uncertain",
                format!("managed admission channel ended: {error}"),
            )
        })?
    }

    fn managed_written(
        &self,
        id: &str,
        generation: u64,
        operation: &str,
        result: Result<(), RunnerError>,
    ) -> Result<(), RunnerError> {
        if let Err(error) = result {
            return self.managed_lost(id, generation, &error.to_string());
        }
        let mut table = self.lock()?;
        runtime(&mut table, id, generation)?;
        if table
            .operations
            .get(operation)
            .is_some_and(|held| held.state == OperationState::Delivering)
        {
            crate::operations::managed_state(
                &mut table,
                operation,
                OperationState::Accepted,
                "managed request written; harness evidence pending".to_owned(),
            )?;
        }
        drop(table);
        self.writer.barrier()?;
        self.wake();
        Ok(())
    }

    pub(crate) fn managed_lost(
        &self,
        id: &str,
        generation: u64,
        reason: &str,
    ) -> Result<(), RunnerError> {
        let mut table = self.lock()?;
        let held = runtime(&mut table, id, generation)?;
        let leader = held.controller.binding.leader.clone();
        held.left.store(true, Ordering::SeqCst);
        let update = held.controller.disconnected();
        let admissions = std::mem::take(&mut held.admissions);
        let applied = apply(&mut table, id, generation, update);
        if let Some(session) = table.sessions.get_mut(id) {
            session.ending = true;
        }
        drop(table);
        for sender in admissions.into_values() {
            if sender
                .send(Err(RunnerError::refused("control_transport_lost", reason)))
                .is_err()
            {
                crate::error::said("control_input_caller_left: transport loss was not received");
            }
        }
        let ended = match crate::pty::end_left_group(&leader) {
            Ok(crate::pty::Left::Gone | crate::pty::Left::Ended { reason: None }) => Ok(()),
            Ok(left) => Err(RunnerError::refused(
                "control_process_end_unconfirmed",
                format!("lost managed process group could not be ended: {left:?}"),
            )),
            Err(error) => Err(error),
        };
        let durable = self.writer.barrier();
        self.wake();
        if let Err(error) = &applied {
            crate::error::said(&format!("control_loss_record_failed: {error}"));
        }
        if let Err(error) = &durable {
            crate::error::said(&format!("control_loss_fence_failed: {error}"));
        }
        ended?;
        applied?;
        durable?;
        Ok(())
    }

    pub(crate) fn managed_read(
        self: &Arc<Self>,
        id: &str,
        generation: u64,
        reader: Box<dyn Read + Send>,
    ) {
        let mut reader = std::io::BufReader::new(reader);
        let (source, transport) = match self.lock().and_then(|mut table| {
            let control = &runtime(&mut table, id, generation)?.controller;
            Ok((control.binding.clone(), control.transport))
        }) {
            Ok(source) => source,
            Err(error) => {
                crate::error::said(&format!("control_source_unbound: {error}"));
                return;
            }
        };
        loop {
            let observed = super::process::frame(&mut reader).and_then(|value| {
                let passive = match transport {
                    Transport::Claude => super::claude::passive_frame(&value),
                    Transport::Codex => super::codex::passive_frame(&value),
                    Transport::Pty => return Err(super::unsupported()),
                };
                if passive {
                    return Ok(false);
                }
                if value.get("id").is_some() && value.get("method").is_some() {
                    self.managed_approval(id, generation, &source, value)?;
                    return Ok(true);
                }
                let mut table = self.lock()?;
                let held = runtime(&mut table, id, generation)?;
                let update = held.controller.ingest(&source, &value)?;
                let closed = held.controller.closed;
                let changed = !update.events.is_empty()
                    || !update.receipts.is_empty()
                    || !update.dispatches.is_empty();
                apply(&mut table, id, generation, update)?;
                drop(table);
                if changed {
                    self.writer.barrier()?;
                }
                if closed {
                    return Err(RunnerError::refused(
                        "harness_refused",
                        "correlated request was refused; this managed generation ended",
                    ));
                }
                Ok(changed)
            });
            if let Err(error) = observed {
                if let Err(lost) = self.managed_lost(id, generation, &error.to_string()) {
                    crate::error::said(&format!("control_transport_loss_unrecorded: {lost}"));
                }
                crate::error::said(&format!("managed reader ended: {error}"));
                break;
            }
            if matches!(observed, Ok(true)) {
                self.wake();
            }
        }
    }

    pub(crate) fn managed_bootstrap(&self, id: &str, generation: u64) -> Result<(), RunnerError> {
        let mut table = self.lock()?;
        let held = runtime(&mut table, id, generation)?;
        let update = held.controller.bootstrap();
        apply(&mut table, id, generation, update)
    }
}

#[cfg(test)]
mod tests {
    use super::{Binding, Sessions, Transport};
    use crate::harness_control::Settings;
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
        let update = super::runtime(&mut table, &source.session, source.generation)?
            .controller
            .ingest(source, value)?;
        super::apply(&mut table, &source.session, source.generation, update)?;
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
            self.ended.store(true, Ordering::SeqCst);
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
        let executable =
            crate::harness_control::process::executable(std::path::Path::new("/bin/sh"))?;
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
                adapter: "fixture/1".to_owned(),
            }
        };
        let (sender, frames) = mpsc::channel();
        let terminal = Arc::new(AtomicUsize::new(0));
        let (runtime, bridge) = super::live(
            binding.clone(),
            transport,
            Box::new(Pipe(sender)),
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
        match transport {
            Transport::Claude => observe(
                &sessions,
                &binding,
                &json!({"type":"system","subtype":"init",
                "session_id":"conversation","claude_code_version":"9.8.7","slash_commands":["compact"]}),
            )?,
            Transport::Codex => {
                observe(
                    &sessions,
                    &binding,
                    &json!({"id":"lys-initialize","result":{"userAgent":"lys-runner/9.8.7 (fixture)"}}),
                )?;
                for expected in ["initialized", "thread/resume"] {
                    let frame: Value = serde_json::from_slice(&frames.recv()?)?;
                    assert_eq!(frame["method"], expected);
                }
                observe(
                    &sessions,
                    &binding,
                    &json!({"id":"lys-thread","result":{"thread":{"id":"conversation","status":{"type":"idle"}}}}),
                )?;
            }
            Transport::Pty => return Err("fixture requires a managed adapter".into()),
        }
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

    #[test]
    fn a_boundary_journal_refusal_keeps_authority_and_queued_words_unchanged() -> Result {
        controlled_fixture(
            Transport::Claude,
            false,
            false,
            |sessions, source, frames, terminal| {
                let reference = crate::harness_control::ReminderReference {
                    goal: "goal".to_owned(),
                    occurrence: "occurrence".to_owned(),
                    version: "old".to_owned(),
                    prior: None,
                };
                sessions.operate(operation(
                    "delivery",
                    OperationRequest::GoalReminder {
                        text: "old words".to_owned(),
                        reference: reference.clone(),
                    },
                ))?;
                let (before, offset) = {
                    let mut table = sessions.lock()?;
                    let status = super::runtime(&mut table, "fixture", source.generation)?
                        .controller
                        .control_status();
                    let offset = table.operations.test_journal_offset(u64::MAX);
                    (status, offset)
                };
                let mut reference = reference;
                reference.version = "current".to_owned();
                let reply = operation(
                    "decision",
                    OperationRequest::BoundaryReply {
                        reply: crate::harness_control::BoundaryReply {
                            generation: source.generation,
                            boundary: before.boundary.clone(),
                            context: crate::harness_control::ContextDecision::Released,
                            reminders: vec![crate::harness_control::ReminderDecision::Deliver {
                                operation: "delivery".to_owned(),
                                reference,
                                text: "current words".to_owned(),
                            }],
                        },
                    },
                );
                let refused = sessions.apply_boundary_reply(reply.clone());
                let after = {
                    let mut table = sessions.lock()?;
                    table.operations.test_journal_offset(offset);
                    assert!(table.operations.get("decision").is_none());
                    super::runtime(&mut table, "fixture", source.generation)?
                        .controller
                        .control_status()
                };
                assert!(
                    refused
                        .err()
                        .ok_or("journal refusal was accepted")?
                        .to_string()
                        .contains("journal offset overflows")
                );
                assert_eq!(after, before);
                assert!(matches!(frames.try_recv(), Err(mpsc::TryRecvError::Empty)));
                assert_eq!(terminal.load(Ordering::SeqCst), 0);
                assert_eq!(
                    sessions.apply_boundary_reply(reply)?.state,
                    OperationState::Confirmed
                );
                let frame: Value = serde_json::from_slice(&frames.recv()?)?;
                assert_eq!(frame["message"]["content"], "Lys reminder\ncurrent words");
                Ok(())
            },
        )
    }

    #[test]
    fn channel_loss_releases_human_waiters_refuses_more_input_and_ends_its_child() -> Result {
        fixture(
            Transport::Codex,
            true,
            |sessions, source, frames, terminal| {
                sessions.operate(operation(
                    "current",
                    OperationRequest::Compact {
                        text: String::new(),
                    },
                ))?;
                let frame: Value = serde_json::from_slice(&frames.recv()?)?;
                assert_eq!(frame["method"], "thread/compact/start");
                let human = Arc::clone(sessions);
                let waiter =
                    std::thread::spawn(move || human.input("fixture", "human words", true));
                sessions.until_any(&std::sync::atomic::AtomicBool::new(false), |table| {
                    match super::runtime(table, "fixture", source.generation) {
                        Ok(runtime) => (!runtime.admissions.is_empty()).then_some(Ok(())),
                        Err(error) => Some(Err(error)),
                    }
                })??;
                assert_eq!(terminal.load(Ordering::SeqCst), 0);
                sessions.managed_lost("fixture", source.generation, "reader lost")?;
                let error = waiter
                    .join()
                    .map_err(|error| format!("human caller panicked: {error:?}"))?
                    .err()
                    .ok_or("lost channel admitted human input")?;
                assert!(error.to_string().contains("control_transport_lost"));
                let ending = {
                    let table = sessions.lock()?;
                    let session = table.sessions.get("fixture").ok_or("fixture missing")?;
                    session.ending
                };
                assert!(ending);
                let error = sessions
                    .managed_input("fixture", source.generation, b"later")
                    .err()
                    .ok_or("lost generation accepted more human input")?;
                assert_eq!(error.name(), "session_ending");
                sessions.until(
                    "fixture",
                    &std::sync::atomic::AtomicBool::new(false),
                    |state, _| state.ended().map(Ok),
                )?;
                let outcome = sessions.operate(operation(
                    "later",
                    OperationRequest::Reminder {
                        text: "later".to_owned(),
                    },
                ))?;
                assert_eq!(outcome.state, OperationState::Refused);
                assert!(outcome.words.starts_with("session_ended:"));
                Ok(())
            },
        )
    }

    #[test]
    fn the_terminal_writer_spy_records_zero_automated_writes_during_claude_compaction() -> Result {
        fixture(
            Transport::Claude,
            false,
            |sessions, source, frames, terminal| {
                let outcome = sessions.operate(operation(
                    "compact",
                    OperationRequest::Compact {
                        text: "untrusted terminal command".to_owned(),
                    },
                ))?;
                assert_eq!(outcome.state, OperationState::Accepted);
                let frame: Value = serde_json::from_slice(&frames.recv()?)?;
                assert_eq!(frame["message"]["content"], "/compact");
                assert_eq!(terminal.load(Ordering::SeqCst), 0);
                observe(
                    sessions,
                    source,
                    &json!({"type":"user","session_id":"conversation","uuid":frame["uuid"],
                "parent_tool_use_id":null,"message":{"role":"user","content":"/compact"}}),
                )?;
                observe(
                    sessions,
                    source,
                    &json!({"type":"system","subtype":"compact_boundary","session_id":"conversation"}),
                )?;
                observe(
                    sessions,
                    source,
                    &json!({"type":"result","session_id":"conversation","uuid":"compact-result","is_error":false}),
                )?;
                assert_eq!(
                    sessions.outcome("compact")?.state,
                    OperationState::Confirmed
                );
                assert_eq!(terminal.load(Ordering::SeqCst), 0);
                Ok(())
            },
        )
    }

    #[test]
    fn a_hook_callback_returns_while_its_turn_remains_active() -> Result {
        fixture(
            Transport::Codex,
            false,
            |sessions, source, frames, terminal| {
                observe(
                    sessions,
                    source,
                    &json!({"method":"turn/started","params":{"threadId":"conversation","turn":{"id":"active"}}}),
                )?;
                let outcome = sessions.operate(operation(
                    "reminder",
                    OperationRequest::Reminder {
                        text: "saved reminder".to_owned(),
                    },
                ))?;
                assert_eq!(outcome.state, OperationState::Accepted);
                sessions.collect(
                    "fixture",
                    &crate::peer::Collected::Hook {
                        event: "Stop".to_owned(),
                        input: json!({}),
                    },
                )?;
                let mut table = sessions.lock()?;
                assert!(
                    !super::runtime(&mut table, "fixture", source.generation)?
                        .controller
                        .idle()
                );
                drop(table);
                assert!(matches!(frames.try_recv(), Err(mpsc::TryRecvError::Empty)));
                assert_eq!(terminal.load(Ordering::SeqCst), 0);
                Ok(())
            },
        )
    }

    #[test]
    fn a_concurrent_human_message_cannot_overtake_the_boundary_dispatcher() -> Result {
        fixture(
            Transport::Claude,
            true,
            |sessions, source, frames, terminal| {
                sessions.operate(operation(
                    "compact",
                    OperationRequest::Compact {
                        text: "/compact".to_owned(),
                    },
                ))?;
                let first: Value = serde_json::from_slice(&frames.recv()?)?;
                let human = Arc::clone(sessions);
                let sender =
                    std::thread::spawn(move || human.input("fixture", "human words", true));
                sessions.until_any(&std::sync::atomic::AtomicBool::new(false), |table| {
                    match super::runtime(table, "fixture", source.generation) {
                        Ok(runtime) => (!runtime.admissions.is_empty()).then_some(Ok(())),
                        Err(error) => Some(Err(error)),
                    }
                })??;
                assert!(matches!(frames.try_recv(), Err(mpsc::TryRecvError::Empty)));
                observe(
                    sessions,
                    source,
                    &json!({"type":"user","session_id":"conversation","uuid":first["uuid"],
                "parent_tool_use_id":null,"message":{"role":"user","content":"/compact"}}),
                )?;
                observe(
                    sessions,
                    source,
                    &json!({"type":"system","subtype":"compact_boundary","session_id":"conversation"}),
                )?;
                observe(
                    sessions,
                    source,
                    &json!({"type":"result","session_id":"conversation","uuid":"compact-result","is_error":false}),
                )?;
                let second: Value = serde_json::from_slice(&frames.recv()?)?;
                assert_eq!(second["message"]["content"], "human words");
                assert_eq!(first["message"]["content"], "/compact");
                observe(
                    sessions,
                    source,
                    &json!({"type":"user","session_id":"conversation","uuid":second["uuid"],
                "parent_tool_use_id":null,"message":{"role":"user","content":"human words"}}),
                )?;
                sender
                    .join()
                    .map_err(|cause| format!("human input worker panicked: {cause:?}"))??;
                assert_eq!(terminal.load(Ordering::SeqCst), 0);
                Ok(())
            },
        )
    }

    #[test]
    fn the_managed_identity_record_keeps_the_selected_executable_fingerprint() -> Result {
        fixture(
            Transport::Claude,
            false,
            |sessions, source, frames, terminal| {
                assert!(matches!(frames.try_recv(), Err(mpsc::TryRecvError::Empty)));
                let page = sessions.lock()?.feed.page(None)?;
                let identity = page
                    .entries
                    .into_iter()
                    .find_map(|entry| match entry.body {
                        Body::Managed(event) if event.event == "control_bound" => {
                            Some(event.binding.harness)
                        }
                        _ => None,
                    })
                    .ok_or("managed binding was not recorded")?;
                assert_eq!(identity, source.harness);
                assert_eq!(
                    identity.path,
                    std::path::Path::new("/bin/sh")
                        .canonicalize()?
                        .display()
                        .to_string()
                );
                assert_eq!(identity.sha256.len(), 64);
                assert_eq!(terminal.load(Ordering::SeqCst), 0);
                Ok(())
            },
        )
    }

    fn control_answer(
        sessions: &Arc<Sessions>,
        session: &str,
    ) -> std::result::Result<crate::protocol::Answer, Box<dyn std::error::Error>> {
        let act: crate::protocol::Act =
            serde_json::from_value(serde_json::json!({"act":"control_status","session":session}))?;
        let dir = tempfile::tempdir()?;
        let key = lys_core::Ed25519Identity::load_or_generate(&dir.path().join("key"))?;
        let greeting = crate::protocol::Greeting::fresh("21");
        let line = crate::protocol::sign_request(&key, &greeting, &act)?;
        Ok(crate::socket::dispatch(
            sessions,
            &key.public_key_bytes(),
            &greeting,
            &line,
            &std::sync::atomic::AtomicBool::new(false),
        ))
    }

    #[test]
    fn current_control_status_matches_the_same_sessions_status_fields() -> Result {
        fixture(Transport::Claude, false, |sessions, _, _, _| {
            sessions.until_any(&std::sync::atomic::AtomicBool::new(false), |table| {
                table
                    .sessions
                    .get("fixture")
                    .and_then(|session| session.live.as_ref())
                    .and_then(|live| live.control.as_ref())
                    .and_then(|runtime| runtime.controller.control_status().context)
            })?;
            let ordinary = sessions.status(Some("fixture"))?;
            let answer = serde_json::to_value(control_answer(sessions, "fixture")?)?;
            assert_eq!(answer["kind"], "control_status");
            assert_eq!(answer["session"], "fixture");
            assert_eq!(
                answer["control"],
                serde_json::to_value(&ordinary.sessions[0].control)?
            );
            assert!(!answer["control"].is_null());
            Ok(())
        })
    }

    #[test]
    fn current_control_status_refuses_an_unknown_session_by_name() -> Result {
        fixture(Transport::Claude, false, |sessions, _, _, _| {
            let answer = serde_json::to_value(control_answer(sessions, "unknown")?)?;
            assert_eq!(answer["kind"], "refused");
            assert_eq!(answer["refusal"], "session_unknown");
            Ok(())
        })
    }

    #[test]
    fn current_control_status_does_not_visit_long_account_move_history() -> Result {
        fixture(Transport::Claude, false, |sessions, _, _, _| {
            let mut rotation = crate::rotation::RotationState::new(crate::rotation::Rotation {
                accounts: (0..10001)
                    .map(|number| format!("handle-{number}"))
                    .collect(),
                variable: "LYS_ACCOUNT".to_owned(),
                resume_arguments: Vec::new(),
                limit: crate::rotation::Limit::PlanWindow,
            })?;
            for number in 0..10000 {
                rotation.advance(number).ok_or("rotation ended early")?;
            }
            sessions
                .lock()?
                .sessions
                .get_mut("fixture")
                .ok_or("session absent")?
                .rotation = Some(rotation);
            crate::session::control_history::READS.with(|reads| reads.set(0));
            assert_eq!(
                sessions.status(Some("fixture"))?.sessions[0].moves.len(),
                10000
            );
            assert_eq!(
                crate::session::control_history::READS.with(std::cell::Cell::get),
                10000
            );
            crate::session::control_history::READS.with(|reads| reads.set(0));
            let answer = serde_json::to_value(control_answer(sessions, "fixture")?)?;
            assert_eq!(answer["kind"], "control_status");
            assert_eq!(
                crate::session::control_history::READS.with(std::cell::Cell::get),
                0
            );
            Ok(())
        })
    }
}
