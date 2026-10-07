//! One dispatcher reserves the next turn before handing input to a pipe.

use std::collections::{BTreeSet, VecDeque};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::RunnerError;
use crate::protocol::Launch;
pub use crate::tracking_store::managed::{Binding, Executable, ManagedEvent};

mod approval;
mod context;
mod initialize;
mod reminders;
pub use context::{
    AppliedContext, BoundaryReply, ContextDecision, ContextReason, ControlPhase, ControlStatus,
};
pub use reminders::{QueuedReminder, ReminderDecision, ReminderReference};
pub mod claude;
pub mod codex;
pub mod events;
pub mod process;

/// The transport selected in the signed launch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case", deny_unknown_fields)]
pub enum Transport {
    /// Existing manual terminal transport.
    Pty,
    /// Claude stream-json pipes.
    Claude,
    /// Codex app-server pipes.
    Codex,
}

/// A transport choice carrying the exact reviewed control requirement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManagedLaunch {
    /// The existing admitted process inputs.
    pub launch: Launch,
    /// The selected transport.
    pub transport: Transport,
    /// An explicit conversation to resume, or empty for a new Codex thread.
    pub conversation: String,
    /// Whether a manual transport must refuse the launch.
    pub requires_controls: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct Settings {
    pub(crate) transport: Transport,
    pub(crate) conversation: String,
}

/// Input's protocol purpose.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Request context compaction.
    Compact,
    /// Deliver saved reminder words.
    Reminder,
    /// Deliver the person's admitted message.
    Human,
}

/// An input's stable correlation and words, held only until dispatch.
#[derive(Debug, Clone)]
pub struct Pending {
    /// The existing operation identity, or the admitted human input identity.
    pub id: String,
    /// Stable envelope correlation.
    pub uuid: String,
    /// The purpose of this input.
    pub kind: Kind,
    /// The admitted text.
    pub text: String,
    pub(crate) reminder: Option<ReminderReference>,
    pub(crate) authorised: Option<String>,
}

/// One frame whose turn was reserved by the dispatcher.
#[derive(Debug, Clone)]
pub struct Dispatch {
    /// Its stable identity.
    pub operation: String,
    /// Its encoded protocol frame.
    pub frame: Value,
}

/// Evidence applied to the existing operation receipt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Receipt {
    /// Its stable identity.
    pub operation: String,
    /// The named outcome.
    pub state: crate::operations::OperationState,
    /// The evidence or refusal name.
    pub reason: String,
}

#[derive(Debug)]
pub(super) enum Observation {
    Other,
    Ready { compact: bool },
    Started { turn: String },
    Accepted,
    Admitted { uuid: String, turn: Option<String> },
    Compacted { turn: Option<String> },
    Completed { turn: Option<String>, failed: bool },
    Refused,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Boundary {
    Unknown,
    Idle,
    Reserved,
    Active(String),
}

/// Results from a frame, applied together before another input is dispatched.
#[derive(Debug, Default)]
pub struct Update {
    /// Evidence retained in the existing feed.
    pub events: Vec<ManagedEvent>,
    /// Operation receipt transitions.
    pub receipts: Vec<Receipt>,
    /// Protocol frames to enqueue after their records are durable.
    pub dispatches: Vec<Dispatch>,
    pub(super) admissions: Vec<(String, Binding, String, Option<String>)>,
}

/// The projection held under the session table, never an additional store.
#[derive(Debug)]
pub struct Controller {
    /// The source's proved identities.
    pub binding: Binding,
    transport: Transport,
    boundary: Boundary,
    compact: bool,
    ready: bool,
    pending: VecDeque<Pending>,
    current: Option<Pending>,
    compacted: Option<String>,
    admitted: bool,
    turn: Option<String>,
    last_result: Option<String>,
    initialized: bool,
    initialize: String,
    refusal: Option<String>,
    replayed: bool,
    closed: bool,
    pending_ids: BTreeSet<String>,
    authority: context::Authority,
}

impl Controller {
    /// Bind a new projection in unknown boundary state.
    ///
    /// # Errors
    /// Refuses a manual transport or invalid source identity.
    pub fn new(binding: Binding, transport: Transport) -> Result<Self, RunnerError> {
        if transport == Transport::Pty {
            return Err(unsupported());
        }
        if binding.session.is_empty() || binding.generation == 0 || binding.leader.pid <= 1 {
            return Err(RunnerError::refused(
                "control_binding_invalid",
                "the source has no proved session generation or leader",
            ));
        }
        Ok(Self {
            binding,
            transport,
            boundary: Boundary::Unknown,
            compact: false,
            ready: false,
            pending: VecDeque::new(),
            current: None,
            compacted: None,
            admitted: false,
            turn: None,
            last_result: None,
            initialized: false,
            initialize: crate::protocol::hex(&rand::random::<[u8; 16]>()),
            refusal: None,
            replayed: false,
            closed: false,
            pending_ids: BTreeSet::new(),
            authority: context::Authority::default(),
        })
    }

    pub(crate) fn active_turn(&self) -> Option<&str> {
        self.turn.as_deref()
    }

    /// Whether authoritative state currently permits a new turn.
    #[must_use]
    pub fn idle(&self) -> bool {
        self.boundary == Boundary::Idle && (!self.authority.required || self.authority.normal)
    }

    /// Whether the channel has been bound by harness evidence.
    #[must_use]
    pub fn ready(&self) -> bool {
        self.ready
    }

    /// Reconnecting never invents an idle boundary or repeats a possible write.
    pub fn disconnected(&mut self) -> Update {
        self.ready = false;
        self.closed = true;
        self.boundary = Boundary::Unknown;
        let mut update = Update::default();
        if let Some(current) = self.current.take() {
            if let Some(reason) = &self.refusal {
                update.receipts.push(receipt(
                    &current.id,
                    crate::operations::OperationState::Refused,
                    reason,
                ));
            } else if current.kind == Kind::Compact || !self.admitted {
                update.receipts.push(receipt(
                    &current.id,
                    crate::operations::OperationState::Uncertain,
                    "control_transport_lost",
                ));
            }
        }
        for pending in self.pending.drain(..) {
            update.receipts.push(receipt(
                &pending.id,
                crate::operations::OperationState::Refused,
                "control_transport_lost: input was not written",
            ));
        }
        self.pending_ids.clear();
        self.turn = None;
        self.compacted = None;
        self.admitted = false;
        update
            .events
            .push(self.event("control_transport_lost", None, None));
        update
    }

    /// Queue an admitted input and reserve its turn before returning a frame.
    ///
    /// # Errors
    /// Refuses unbound sources, repeated identities and unavailable compaction.
    pub fn enqueue(&mut self, pending: Pending) -> Result<Update, RunnerError> {
        if pending.id.starts_with("lys-") {
            return Err(RunnerError::refused(
                "control_identity_reserved",
                "operation identity belongs to the protocol handshake",
            ));
        }
        if !self.ready {
            return Err(RunnerError::refused(
                "control_source_unbound",
                "the harness has not proved its conversation",
            ));
        }
        if pending.kind == Kind::Compact && !self.compact {
            return Err(RunnerError::refused(
                "control_capability_unsupported",
                "the harness did not advertise compaction",
            ));
        }
        if self
            .current
            .as_ref()
            .is_some_and(|held| held.id == pending.id)
            || self.pending_ids.contains(&pending.id)
        {
            return Err(RunnerError::refused(
                "operation_reused",
                "this input is already held",
            ));
        }
        self.pending_ids.insert(pending.id.clone());
        if pending.kind == Kind::Compact {
            let before_input = self
                .pending
                .iter()
                .position(|held| held.kind != Kind::Compact)
                .unwrap_or(self.pending.len());
            self.pending.insert(before_input, pending);
        } else {
            self.pending.push_back(pending);
        }
        let mut update = Update::default();
        if self.boundary == Boundary::Idle {
            self.request_boundary(&mut update)?;
        }
        self.dispatch(&mut update)?;
        Ok(update)
    }

    fn dispatch(&mut self, update: &mut Update) -> Result<(), RunnerError> {
        if self.boundary != Boundary::Idle || !self.permitted() {
            return Ok(());
        }
        let Some(pending) = self.pending.pop_front() else {
            return Ok(());
        };
        self.pending_ids.remove(&pending.id);
        self.boundary = Boundary::Reserved;
        self.authority.waiting = None;
        self.authority.answered = false;
        self.compacted = None;
        self.admitted = false;
        self.turn = None;
        let frame = match self.transport {
            Transport::Claude => claude::request(&self.binding.conversation, &pending),
            Transport::Codex => codex::request(&self.binding.conversation, &pending),
            Transport::Pty => return Err(unsupported()),
        };
        update
            .events
            .push(self.event("control_prepared", None, Some(pending.id.clone())));
        update.dispatches.push(Dispatch {
            operation: pending.id.clone(),
            frame,
        });
        self.current = Some(pending);
        Ok(())
    }

    /// Read one frame from the single bound reader.
    ///
    /// # Errors
    /// Refuses an unsupported protocol or missing required correlation.
    pub fn ingest(&mut self, source: &Binding, value: &Value) -> Result<Update, RunnerError> {
        if self.closed {
            return Err(RunnerError::refused(
                "control_transport_lost",
                "this managed controller generation has ended",
            ));
        }
        if source.session != self.binding.session
            || source.generation != self.binding.generation
            || source.leader != self.binding.leader
            || source.entry != self.binding.entry
            || source.harness != self.binding.harness
            || source.adapter != self.binding.adapter
            || source.harness_version != self.binding.harness_version
            || (!source.conversation.is_empty() && source.conversation != self.binding.conversation)
        {
            return Err(RunnerError::refused(
                "control_source_mismatch",
                "event belongs to another source",
            ));
        }
        if self.transport == Transport::Claude
            && let Some(update) = self.bind_claude(value)?
        {
            return Ok(update);
        }
        if self.transport == Transport::Codex
            && let Some(update) = self.bind_codex(value)?
        {
            return Ok(update);
        }
        if self.transport == Transport::Claude
            && value.get("type").and_then(Value::as_str) == Some("result")
        {
            if value.get("session_id").and_then(Value::as_str)
                != Some(self.binding.conversation.as_str())
            {
                return Ok(Update::default());
            }
            let id = value
                .get("uuid")
                .and_then(Value::as_str)
                .filter(|id| !id.is_empty())
                .ok_or_else(|| {
                    RunnerError::refused(
                        "control_correlation_unsupported",
                        "Claude result has no event identity",
                    )
                })?;
            if self.last_result.as_deref() == Some(id) {
                return Ok(Update::default());
            }
            if !self.admitted {
                return Err(RunnerError::refused(
                    "control_correlation_unsupported",
                    "Claude result has no matching input replay",
                ));
            }
            self.last_result = Some(id.to_owned());
        }
        let observation = match self.transport {
            Transport::Claude => self.observe_claude(value)?,
            Transport::Codex => {
                codex::observe(&self.binding.conversation, value, self.current.as_ref())?
            }
            Transport::Pty => return Err(unsupported()),
        };
        self.apply(observation)
    }

    fn apply(&mut self, observation: Observation) -> Result<Update, RunnerError> {
        let mut update = Update::default();
        match observation {
            Observation::Other | Observation::Accepted => {}
            Observation::Ready { compact } => {
                if self.transport == Transport::Claude {
                    return self.claude_version(compact);
                }
                if self.ready {
                    return Ok(update);
                }
                self.ready = true;
                self.compact = compact;
                self.boundary = Boundary::Idle;
                update.events.push(self.event("control_bound", None, None));
                self.request_boundary(&mut update)?;
            }
            Observation::Started { turn } => {
                if self.boundary == Boundary::Reserved || self.boundary == Boundary::Idle {
                    self.turn = Some(turn.clone());
                    self.boundary = Boundary::Active(turn.clone());
                    update
                        .events
                        .push(self.event("turn_start", Some(turn), None));
                }
            }
            Observation::Admitted { uuid, turn } => {
                if self.transport == Transport::Claude && self.defer_claude_admission(&uuid) {
                    return Ok(update);
                }
                if let Some(current) = &self.current {
                    if current.uuid == uuid && !self.admitted {
                        update.admissions.push((
                            current.id.clone(),
                            self.binding.clone(),
                            uuid,
                            turn.clone(),
                        ));
                        self.admitted = true;
                        if let Some(turn) = turn {
                            self.turn = Some(turn.clone());
                            self.boundary = Boundary::Active(turn);
                        }
                        if current.kind != Kind::Compact {
                            update.receipts.push(receipt(
                                &current.id,
                                crate::operations::OperationState::Confirmed,
                                "harness_admitted",
                            ));
                        }
                        update.events.push(self.event(
                            "harness_admitted",
                            self.turn.clone(),
                            Some(current.id.clone()),
                        ));
                    }
                }
            }
            Observation::Compacted { turn } => {
                if self.compaction_matches(turn.as_deref()) {
                    self.compacted = Some(turn.unwrap_or_default());
                }
            }
            Observation::Completed { turn, failed } => {
                if self.transport == Transport::Claude && !self.initialized {
                    return Err(self.claude_unproved());
                }
                let matches = if self.transport == Transport::Claude {
                    self.current.is_some() && self.boundary != Boundary::Unknown
                } else {
                    turn.is_some()
                        && turn == self.turn
                        && matches!(self.boundary, Boundary::Active(_))
                };
                if !matches {
                    return Ok(update);
                }
                if let Some(current) = self.current.take() {
                    if current.kind == Kind::Compact {
                        let (state, reason) = if self.compacted.is_some() && !failed {
                            (
                                crate::operations::OperationState::Confirmed,
                                "harness_compacted",
                            )
                        } else {
                            (crate::operations::OperationState::Refused, "not_compacted")
                        };
                        update.receipts.push(receipt(&current.id, state, reason));
                    } else if !self.admitted {
                        update.receipts.push(receipt(
                            &current.id,
                            crate::operations::OperationState::Uncertain,
                            "admission_unconfirmed",
                        ));
                    }
                    update
                        .events
                        .push(self.event("turn_end", self.turn.clone(), Some(current.id)));
                } else {
                    update
                        .events
                        .push(self.event("turn_end", self.turn.clone(), None));
                }
                self.boundary = Boundary::Idle;
                self.turn = None;
                self.compacted = None;
                self.request_boundary(&mut update)?;
                self.dispatch(&mut update)?;
            }
            Observation::Refused => {
                if let Some(current) = self.current.take() {
                    update.receipts.push(receipt(
                        &current.id,
                        crate::operations::OperationState::Refused,
                        "harness_refused",
                    ));
                }
                for pending in self.pending.drain(..) {
                    update.receipts.push(receipt(
                        &pending.id,
                        crate::operations::OperationState::Refused,
                        "harness_refused: queued input was not written",
                    ));
                }
                self.pending_ids.clear();
                self.ready = false;
                self.closed = true;
                self.boundary = Boundary::Unknown;
                self.turn = None;
                self.compacted = None;
                self.admitted = false;
                update
                    .events
                    .push(self.event("control_request_refused", None, None));
            }
        }
        Ok(update)
    }

    fn event(&self, event: &str, turn: Option<String>, operation: Option<String>) -> ManagedEvent {
        ManagedEvent {
            binding: self.binding.clone(),
            event: event.to_owned(),
            turn,
            operation,
        }
    }
}

fn receipt(operation: &str, state: crate::operations::OperationState, reason: &str) -> Receipt {
    Receipt {
        operation: operation.to_owned(),
        state,
        reason: reason.to_owned(),
    }
}

pub(crate) fn unsupported() -> RunnerError {
    RunnerError::refused(
        "control_transport_unsupported",
        "automatic controls require a managed pipe transport",
    )
}
