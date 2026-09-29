//! The sole managed writer reserves a flight before touching a pipe. Native
//! admission does not release it: a matching terminal turn is also required.
//! Existing 051 owners supply authorization, context holds and durable writes;
//! this module creates no scheduler, operation store or process lifecycle.

use serde_json::Value;

use super::events::{Boundary, Event, Kind, Projection, Source};
use super::process::{Pipes, Reader, WriteAhead};
use super::{claude, codex};
use crate::error::RunnerError;

/// A request encoded by one of the supported codecs, never arbitrary JSON or
/// terminal text. The dispatcher checks the bound conversation again at send.
pub enum Prepared {
    /// A bound Codex native request.
    Codex(codex::Request),
    /// A bound Claude native envelope.
    Claude(claude::Request),
}

impl Prepared {
    fn is_compaction(&self) -> bool {
        match self {
            Self::Codex(request) => request.is_compaction(),
            Self::Claude(request) => request.is_compaction(),
        }
    }

    fn frame(&self) -> &Value {
        match self {
            Self::Codex(request) => request.frame(),
            Self::Claude(request) => request.frame(),
        }
    }

    fn operation(&self) -> Option<&str> {
        match self {
            Self::Codex(request) => request.frame().get("id")?.as_str(),
            Self::Claude(request) => request.frame().get("uuid")?.as_str(),
        }
    }

    fn conversation(&self) -> Option<&str> {
        match self {
            Self::Codex(request) => request.frame().get("params")?.get("threadId")?.as_str(),
            Self::Claude(request) => request.frame().get("session_id")?.as_str(),
        }
    }
}

/// One reserved operation, including the possible-write gap before admission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Flight {
    /// Existing 051 operation identity, not a newly minted retry identity.
    pub operation: String,
    /// Native turn learned from admission or its start observation.
    pub turn: Option<String>,
    /// Matching native admission observed and durably kept.
    pub admitted: bool,
    /// Matching native terminal observed and durably kept.
    pub terminal: bool,
    /// This flight requires actual compaction evidence, not merely a result.
    pub compact: bool,
    /// Matching compaction evidence was durably retained.
    pub compacted: bool,
}

/// Per-session writer authority. It borrows no PTY or other session's input.
pub struct Dispatcher {
    source: Source,
    projection: Projection,
    pipes: Pipes,
    flight: Option<Flight>,
    pending: Option<Pending>,
}

struct Pending {
    event: Event,
    before: Projection,
    retained: Option<bool>,
}

/// The existing feed and receipt owner, consulted before applying a replay.
/// Retention checks compare the entire fact, not just an event-id string.
pub trait EventJournal {
    /// Whether this exact fact is already durable; conflicting reuse refuses.
    fn retained(&mut self, event: &Event) -> Result<bool, RunnerError>;
    /// Keep the fact and reconcile its receipt, including after partial failure.
    fn keep(&mut self, event: &Event) -> Result<(), RunnerError>;
}

fn refuse(name: &str, reason: &str) -> RunnerError {
    RunnerError::refused(name, reason)
}

impl Dispatcher {
    /// Attach to the launch owner's actual child pipes and proved source.
    /// This validates process binding, not containment or harness capability.
    pub fn new(source: Source, pipes: Pipes) -> Result<Self, RunnerError> {
        if pipes.leader() != &source.leader {
            return Err(refuse(
                "control_source_mismatch",
                "pipe process differs from the bound event source",
            ));
        }
        let projection = Projection::new(source.clone())?;
        Ok(Self {
            source,
            projection,
            pipes,
            flight: None,
            pending: None,
        })
    }

    /// Transfer the one native reader to its event pump exactly once.
    pub fn take_reader(&mut self) -> Result<Reader, RunnerError> {
        self.pipes.take_reader()
    }

    /// Actual observed boundary, never inferred from quiet output.
    pub fn boundary(&self) -> &Boundary {
        self.projection.boundary()
    }

    /// A possible send remains reserved even before the harness admits it.
    pub fn flight(&self) -> Option<&Flight> {
        self.flight.as_ref()
    }

    /// Check current authority/goal revision/context hold through the existing
    /// owner, reserve this flight, then journal before the only pipe write.
    /// The mutable borrow covers the whole check/send, so this writer cannot
    /// accept a person's competing input in the boundary/write gap.
    pub fn dispatch(
        &mut self,
        operation: &str,
        request: &Prepared,
        journal: &mut impl WriteAhead,
        authorize_now: &mut impl FnMut(&str) -> Result<(), RunnerError>,
    ) -> Result<(), RunnerError> {
        if operation.is_empty() || request.conversation() != Some(self.source.conversation.as_str())
        {
            return Err(refuse(
                "control_source_mismatch",
                "request lacks an operation or names another conversation",
            ));
        }
        if request.operation() != Some(operation) {
            return Err(refuse(
                "control_operation_mismatch",
                "native request differs from the operation recorded before its write",
            ));
        }
        if self.flight.is_some() || self.projection.boundary() != &Boundary::Idle {
            return Err(refuse(
                "control_boundary_held",
                "a pending operation or unproved boundary holds the next input",
            ));
        }
        authorize_now(operation)?;
        self.flight = Some(Flight {
            operation: operation.to_owned(),
            turn: None,
            admitted: false,
            terminal: false,
            compact: request.is_compaction(),
            compacted: false,
        });
        // Even a journal failure leaves the flight reserved. The existing
        // receipt owner distinguishes safely unsent from possibly sent; this
        // dispatcher cannot silently retry either result under another id.
        self.pipes.send(operation, request.frame(), journal)
    }

    /// Validate and durably keep a native fact before releasing any input.
    /// `keep` appends to the existing tracking feed/operation owner; a failure
    /// invalidates the boundary and retains the unresolved flight.
    pub fn observe(
        &mut self,
        event: &Event,
        journal: &mut impl EventJournal,
    ) -> Result<(), RunnerError> {
        if event.source != self.source || event.source_id.is_empty() {
            return Err(refuse(
                "control_source_mismatch",
                "event differs from the owned source",
            ));
        }
        if let Some(pending) = &self.pending
            && pending.event != *event
        {
            return Err(refuse(
                "control_event_unresolved",
                "retain or reconcile the failed observation before advancing the reader",
            ));
        }
        let before = self
            .pending
            .as_ref()
            .map_or_else(|| self.projection.clone(), |pending| pending.before.clone());
        let retained = match self.pending.as_ref().and_then(|pending| pending.retained) {
            Some(retained) => retained,
            None => match journal.retained(event) {
                Ok(retained) => retained,
                Err(error) => {
                    self.pending = Some(Pending {
                        event: event.clone(),
                        before,
                        retained: None,
                    });
                    self.projection.gap();
                    return Err(error);
                }
            },
        };
        if retained {
            // This dispatcher already applied a retained fact, or restarted
            // without its live projection. Neither case permits applying that
            // old fact to today's flight. A partial keep in THIS dispatcher is
            // pending with retained=false and still completes its original keep.
            // Historical receipt repair uses explicit operation reconciliation.
            self.pending = None;
            self.projection = before;
            return Ok(());
        }
        let mut next = before.clone();
        self.validate_flight(event)?;
        if let Err(error) = next.apply(event) {
            self.projection = next;
            return Err(error);
        }
        if let Err(error) = journal.keep(event) {
            self.pending = Some(Pending {
                event: event.clone(),
                before,
                retained: Some(retained),
            });
            self.projection.gap();
            return Err(error);
        }
        self.pending = None;
        self.projection = next;
        if let Some(flight) = &mut self.flight {
            match &event.kind {
                Kind::TurnStarted { turn } => flight.turn = Some(turn.clone()),
                Kind::Admitted { turn, .. } => {
                    flight.admitted = true;
                    if let Some(turn) = turn {
                        flight.turn = Some(turn.clone());
                    }
                }
                Kind::TurnCompleted { .. } => flight.terminal = true,
                Kind::Compacted { .. } => flight.compacted = true,
                Kind::Exited => {
                    self.flight = None;
                    return Ok(());
                }
                _ => {}
            }
            if flight.admitted && flight.terminal && (!flight.compact || flight.compacted) {
                self.flight = None;
            }
        }
        Ok(())
    }

    fn validate_flight(&self, event: &Event) -> Result<(), RunnerError> {
        let Some(flight) = &self.flight else {
            if matches!(event.kind, Kind::Admitted { .. } | Kind::Compacted { .. }) {
                return Err(refuse(
                    "control_operation_mismatch",
                    "operation evidence has no reserved flight",
                ));
            }
            return Ok(());
        };
        match &event.kind {
            Kind::Admitted { operation, turn } => {
                if operation != &flight.operation
                    || turn
                        .as_ref()
                        .zip(flight.turn.as_ref())
                        .is_some_and(|(a, b)| a != b)
                {
                    return Err(refuse(
                        "control_operation_mismatch",
                        "admission names another operation or turn",
                    ));
                }
            }
            Kind::Compacted {
                operation, turn, ..
            } => {
                if !flight.compact
                    || operation != &flight.operation
                    || flight.turn.as_ref() != Some(turn)
                {
                    return Err(refuse(
                        "control_operation_mismatch",
                        "compaction evidence does not name the reserved operation and turn",
                    ));
                }
            }
            Kind::TurnStarted { turn } | Kind::TurnCompleted { turn }
                if flight.turn.as_ref().is_some_and(|held| held != turn) =>
            {
                return Err(refuse(
                    "control_operation_mismatch",
                    "native event names another turn",
                ));
            }
            _ => {}
        }
        Ok(())
    }
}
