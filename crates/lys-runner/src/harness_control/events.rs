//! Typed observations from the one session-owned pipe reader. These values
//! contain identities and lifecycle facts, never prompt or credential text.
//! Consumers use `tracking_store::FeedPage`'s existing cursor for readback.

use serde::{Deserialize, Serialize};

use crate::containment_policy::Binding;
use crate::error::RunnerError;
use crate::peer::Leader;

/// The audience bound by the authenticated launch and actual process owner.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    /// The existing containment authority, not a second policy binding.
    pub binding: Binding,
    /// The process identity observed at spawn.
    pub leader: Leader,
    /// The runner's current process generation for this session.
    pub generation: u64,
    /// The actual harness conversation/thread established by its protocol.
    pub conversation: String,
}

/// A lifecycle fact projected from a supported native message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case", deny_unknown_fields)]
pub enum Kind {
    /// Authoritative readback established no active turn. Never silence.
    IdleReconciled,
    /// A specific native turn began.
    TurnStarted {
        /// Native turn identity.
        turn: String,
    },
    /// That native turn terminated; text or tool completion is insufficient.
    TurnCompleted {
        /// The native turn whose terminal state was observed.
        turn: String,
    },
    /// Admission of the existing operation, not completion or obedience.
    Admitted {
        /// Existing 051 operation identity.
        operation: String,
        /// The admitted native turn, when supplied by the harness.
        turn: Option<String>,
    },
    /// Actual native compaction evidence, apart from turn termination.
    Compacted {
        /// Existing compaction operation identity.
        operation: String,
        /// The compaction turn.
        turn: String,
        /// Native completed compaction item.
        item: String,
    },
    /// A native rejection with unknown cause, projected by the 065 adapter.
    Rejected {
        /// Checked native evidence, including its explicitly unknown cause.
        rejection: crate::codex_refusals::Rejection,
    },
    /// Transport was lost, including a gap in retained observations.
    Lost {
        /// A safe reason code without raw native payload.
        reason: String,
    },
    /// The owned process actually exited.
    Exited,
}

/// A safe event published into the existing tracking feed after it is durable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Event {
    /// Full owned source, including generation and conversation.
    pub source: Source,
    /// Stable native identity, or the reader's durably retained source identity.
    pub source_id: String,
    /// The lifecycle fact, with its native turn and operation when known.
    pub kind: Kind,
}

/// What is actually known about the next turn boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Boundary {
    /// A reconnect, gap or lost transport requires authoritative reconciliation.
    Unknown,
    /// Reconciliation or the matching terminal turn proved an idle boundary.
    Idle,
    /// The identified native turn is active, including its tool calls.
    Active(String),
    /// This process ended; it cannot receive another occurrence.
    Ended,
}

/// A projection for the one proved process generation. Reopening one starts
/// unknown; a last-known idle snapshot is never current protocol evidence.
pub struct Projection {
    source: Source,
    boundary: Boundary,
}

fn refuse(name: &str, reason: impl Into<String>) -> RunnerError {
    RunnerError::refused(name, reason)
}

impl Projection {
    /// Bind to the launch owner's source; this does not authenticate a launch.
    /// The owner must supply its already authenticated Binding and proved Leader.
    pub fn new(source: Source) -> Result<Self, RunnerError> {
        if source.conversation.is_empty()
            || source.binding.session.is_empty()
            || source.binding.incarnation.is_empty()
            || source.leader.pid == 0
        {
            return Err(refuse(
                "control_source_unproved",
                "source lacks a conversation, session lifetime or process",
            ));
        }
        Ok(Self {
            source,
            boundary: Boundary::Unknown,
        })
    }

    /// The current observed boundary, with no inference from elapsed time.
    pub fn boundary(&self) -> &Boundary {
        &self.boundary
    }

    /// Reject foreign source evidence before it can open a boundary.
    pub fn apply(&mut self, event: &Event) -> Result<(), RunnerError> {
        if event.source != self.source || event.source_id.is_empty() {
            return Err(refuse(
                "control_source_mismatch",
                "event does not belong to the bound process generation and conversation",
            ));
        }
        if let Kind::Rejected { rejection } = &event.kind
            && (rejection.thread_id() != self.source.conversation
                || rejection.source_id() != event.source_id)
        {
            return Err(refuse(
                "control_source_mismatch",
                "native rejection does not match the event's bound thread and source id",
            ));
        }
        if self.boundary == Boundary::Ended && event.kind != Kind::Exited {
            return Err(refuse(
                "control_session_ended",
                "an exited process cannot open another turn",
            ));
        }
        match &event.kind {
            Kind::IdleReconciled if self.boundary == Boundary::Unknown => {
                self.boundary = Boundary::Idle;
            }
            Kind::IdleReconciled if self.boundary == Boundary::Idle => {}
            Kind::IdleReconciled => {
                return Err(refuse(
                    "control_boundary_conflict",
                    "idle reconciliation cannot erase an observed active or ended process",
                ));
            }
            Kind::TurnStarted { turn } if !turn.is_empty() && self.boundary == Boundary::Idle => {
                self.boundary = Boundary::Active(turn.clone());
            }
            Kind::TurnStarted { turn } if self.boundary == Boundary::Active(turn.clone()) => {}
            Kind::TurnStarted { .. } => {
                self.boundary = Boundary::Unknown;
                return Err(refuse(
                    "control_boundary_unknown",
                    "turn start has no proved preceding boundary",
                ));
            }
            Kind::TurnCompleted { turn } if self.boundary == Boundary::Active(turn.clone()) => {
                self.boundary = Boundary::Idle;
            }
            Kind::TurnCompleted { .. } => {
                return Err(refuse(
                    "control_turn_mismatch",
                    "terminal event does not name the active turn",
                ));
            }
            Kind::Lost { .. } => self.boundary = Boundary::Unknown,
            Kind::Exited => self.boundary = Boundary::Ended,
            // Admission, a rejected item and compaction evidence alone do not
            // finish a native turn or release the context measurement hold.
            Kind::Admitted { .. } | Kind::Compacted { .. } | Kind::Rejected { .. } => {}
        }
        Ok(())
    }

    /// A subscriber could not recover its cursor from the existing feed.
    pub fn gap(&mut self) -> RunnerError {
        self.boundary = Boundary::Unknown;
        refuse(
            "control_event_gap",
            "the required native observations were not retained; reconcile the actual harness state",
        )
    }
}
