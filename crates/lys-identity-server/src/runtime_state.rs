//! What the runtime reports' log folds to, and how that fold is sealed in
//! the log's signed snapshot so a start reads only the leaves after it.
//!
//! A session is either an agent's, reported by the agent itself, or found:
//! seen by a runtime and carrying no identity. A found session is never
//! given an identity here; it stays found.

use std::cell::Cell;
use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// The snapshot domain the reports' folded state is sealed under.
pub const DOMAIN: &str = "lys/identity/runtime-reports-state/v1";

const FORMAT: &str = "lys-runtime-state/v1";

/// A state a runtime reports a session in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Reported {
    /// The runtime is starting the session.
    Starting,
    /// The runtime has the session running.
    Running,
    /// The runtime confirms the session stopped.
    Stopped,
    /// This service asked the runtime to end the session, on an emergency
    /// stop; the session stays unconfirmed until the runtime reports.
    StopAsked,
}

impl Reported {
    /// The state's name, as an answer carries it.
    pub fn name(self) -> &'static str {
        match self {
            Self::Starting => "starting",
            Self::Running => "running",
            Self::Stopped => "stopped",
            Self::StopAsked => "stop_asked",
        }
    }
}

/// One report, as it is kept.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Report {
    /// The operation id it was reported with, which names it.
    pub operation: String,
    /// The session it reports on.
    pub session: String,
    /// The agent the session is held under, null for a found session.
    pub agent: Option<String>,
    /// The machine the session runs on.
    pub machine: String,
    /// The state reported.
    pub state: Reported,
    /// What the runtime saw, in its words; empty when it says nothing.
    pub what: String,
    /// The runtime's own confirmation of a stop; empty for any other state.
    pub confirmation: String,
    /// The identity whose signed-in session delivered the report.
    pub reported_by: String,
    /// When it was received, in seconds since the Unix epoch.
    pub at: u64,
    /// For a start the service admitted, the whole start as it was answered,
    /// so the same request sent again answers it exactly.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub launch: Option<serde_json::Value>,
}

/// One session, with every report on it in order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Tracked {
    /// The session.
    pub session: String,
    /// The agent it is held under, null for a found session.
    pub agent: Option<String>,
    /// The machine it runs on.
    pub machine: String,
    /// Every report on it, in the order received; never empty.
    pub reports: Vec<Report>,
}

impl Tracked {
    /// The latest report.
    pub fn latest(&self) -> Option<&Report> {
        self.reports.last()
    }

    /// The first report.
    pub fn first(&self) -> Option<&Report> {
        self.reports.first()
    }

    /// How the session is shown: `stopped` only once its runtime confirmed
    /// the stop, `unconfirmed` for an agent's session whose runtime has
    /// reported nothing since `starting`, and `running` otherwise. Nothing is
    /// inferred from the clock.
    pub fn shown(&self) -> &'static str {
        match self.latest().map(|report| report.state) {
            Some(Reported::Stopped) => "stopped",
            Some(Reported::Running) => "running",
            Some(Reported::Starting | Reported::StopAsked) | None => "unconfirmed",
        }
    }

    /// When this service last asked the runtime to end the session, null
    /// when it never did.
    pub fn stop_asked_at(&self) -> Option<u64> {
        self.reports
            .iter()
            .rev()
            .find(|report| report.state == Reported::StopAsked)
            .map(|report| report.at)
    }

    /// Whether the runtime has confirmed the session stopped.
    pub fn stopped(&self) -> bool {
        self.latest()
            .is_some_and(|report| report.state == Reported::Stopped)
    }
}

/// The sessions as their log folds them, in the order first reported.
///
/// Beside the sessions are two indexes, never sealed: each session's slot by
/// its id, and each report's slot and place by its operation id. They are
/// rebuilt once when a state is decoded and kept as each report is folded,
/// so a report finds the one session it is about without passing over any
/// other. Two states are equal when their sessions are.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Held {
    sessions: Vec<Tracked>,
    #[serde(skip)]
    by_session: HashMap<String, usize>,
    #[serde(skip)]
    by_operation: HashMap<String, (usize, usize)>,
    #[serde(skip)]
    visited: Cell<u64>,
}

impl PartialEq for Held {
    fn eq(&self, other: &Self) -> bool {
        self.sessions == other.sessions
    }
}

impl Eq for Held {}

/// The sealed state as it is read back: owned, since decoding makes it.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Sealed {
    format: String,
    held: Held,
}

/// The sealed state as it is written: the format and a borrow of what is
/// held, so sealing serialises the state where it lies and copies none of it.
/// Its fields and their order are `Sealed`'s, so the bytes are the same.
#[derive(Serialize)]
struct SealedRef<'a> {
    format: &'a str,
    held: &'a Held,
}

/// Where a report lands among the sessions, found through the indexes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Found {
    /// Its operation id is already kept: the session's slot, and the kept
    /// report's place in it.
    Kept(usize, usize),
    /// Its session is kept, at this slot, and its operation id is not.
    Session(usize),
    /// Neither its operation id nor its session is kept.
    Neither,
}

impl Held {
    /// Every session, in the order first reported.
    pub fn sessions(&self) -> &[Tracked] {
        &self.sessions
    }

    /// How many kept sessions lookups by id have visited, so a test counts
    /// what a report costs rather than timing it.
    pub fn visited(&self) -> u64 {
        self.visited.get()
    }

    fn visit(&self) {
        self.visited.set(self.visited.get().saturating_add(1));
    }

    /// The session at `slot`, reached directly, visiting nothing.
    pub fn at(&self, slot: usize) -> Option<&Tracked> {
        self.sessions.get(slot)
    }

    /// The slot of the session named `session`, visiting only it.
    pub fn slot(&self, session: &str) -> Option<usize> {
        let slot = self.by_session.get(session).copied()?;
        self.visit();
        Some(slot)
    }

    /// The session named `session`.
    pub fn session(&self, session: &str) -> Option<&Tracked> {
        self.slot(session).and_then(|slot| self.sessions.get(slot))
    }

    /// The report kept under `operation`, with its session.
    pub fn operation(&self, operation: &str) -> Option<&Report> {
        let (slot, place) = self.by_operation.get(operation).copied()?;
        self.visit();
        self.sessions.get(slot)?.reports.get(place)
    }

    /// Where a report under `operation` on `session` lands, visiting at most
    /// the one session it is about.
    pub fn find(&self, operation: &str, session: &str) -> Found {
        if let Some((slot, place)) = self.by_operation.get(operation).copied() {
            self.visit();
            return Found::Kept(slot, place);
        }
        self.slot(session).map_or(Found::Neither, Found::Session)
    }

    /// Rebuild both indexes from the sessions, keeping for each id the
    /// first session or report that carries it.
    fn index(&mut self) {
        self.by_session.clear();
        self.by_operation.clear();
        for (slot, tracked) in self.sessions.iter().enumerate() {
            self.by_session
                .entry(tracked.session.clone())
                .or_insert(slot);
            for (place, report) in tracked.reports.iter().enumerate() {
                self.by_operation
                    .entry(report.operation.clone())
                    .or_insert((slot, place));
            }
        }
    }

    /// Fold one report. A report on a session never begun is refused by
    /// reason, since every kept report was checked against what came before.
    pub fn hold(&mut self, report: Report) -> Result<(), String> {
        let slot = self.slot(&report.session);
        self.keep(slot, report).map(drop)
    }

    /// Fold one report into the session at `slot`, found beforehand, or as
    /// the first of a new session when `slot` is none, and answer the slot
    /// it landed in. A first report that does not begin its session is
    /// refused by reason.
    pub fn keep(&mut self, slot: Option<usize>, report: Report) -> Result<usize, String> {
        if let Some(slot) = slot {
            let tracked = self.sessions.get_mut(slot).ok_or_else(|| {
                format!(
                    "report `{}` names slot {slot}, which holds no session",
                    report.operation
                )
            })?;
            let place = tracked.reports.len();
            self.by_operation
                .entry(report.operation.clone())
                .or_insert((slot, place));
            tracked.reports.push(report);
            return Ok(slot);
        }
        let begins = match report.agent {
            Some(_) => report.state == Reported::Starting,
            None => report.state == Reported::Running,
        };
        if !begins {
            return Err(format!(
                "report `{}` is the first on session `{}` and does not begin it",
                report.operation, report.session
            ));
        }
        let slot = self.sessions.len();
        self.by_session
            .entry(report.session.clone())
            .or_insert(slot);
        self.by_operation
            .entry(report.operation.clone())
            .or_insert((slot, 0));
        self.sessions.push(Tracked {
            session: report.session.clone(),
            agent: report.agent.clone(),
            machine: report.machine.clone(),
            reports: vec![report],
        });
        Ok(slot)
    }

    /// Fold every leaf of `tail`, in order.
    pub fn fold(&mut self, tail: &lys_log_store::Tail) -> Result<(), String> {
        for (index, bytes) in (tail.from..).zip(&tail.leaves) {
            let report = serde_json::from_slice(bytes)
                .map_err(|error| format!("leaf {index} is not a runtime report: {error}"))?;
            self.hold(report)
                .map_err(|reason| format!("leaf {index}: {reason}"))?;
        }
        Ok(())
    }

    /// The state a snapshot seals.
    pub fn encode(&self) -> Result<Vec<u8>, String> {
        serde_json::to_vec(&SealedRef {
            format: FORMAT,
            held: self,
        })
        .map_err(|error| format!("runtime state: {error}"))
    }

    /// The state a snapshot sealed, refused by reason unless it reads whole
    /// in this format.
    pub fn decode(bytes: &[u8]) -> Result<Self, String> {
        let sealed: Sealed =
            serde_json::from_slice(bytes).map_err(|error| format!("runtime state: {error}"))?;
        if sealed.format != FORMAT {
            return Err(format!(
                "runtime state is in format {}, not {FORMAT}",
                sealed.format
            ));
        }
        let mut held = sealed.held;
        held.index();
        Ok(held)
    }
}

#[cfg(test)]
#[path = "runtime_state_tests.rs"]
mod tests;
