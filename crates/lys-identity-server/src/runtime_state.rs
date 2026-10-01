//! What the runtime reports' log folds to, and how that fold is sealed in
//! the log's signed snapshot so a start reads only the leaves after it.
//!
//! A session is either an agent's, reported by the agent itself, or found:
//! seen by a runtime and carrying no identity. A found session is never
//! given an identity here; it stays found.

use std::collections::BTreeMap;
use std::ops::Bound;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

/// The snapshot domain the reports' folded state is sealed under.
pub const DOMAIN: &str = "lys/identity/runtime-reports-state/v1";

const FORMAT: &str = "lys-runtime-state/v1";

/// A state a runtime reports a session in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
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
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, from = "Records")]
pub struct Held {
    /// The sessions.
    pub sessions: Vec<Tracked>,
    #[serde(skip)]
    index: Arc<Index>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct Index {
    positions: BTreeMap<String, usize>,
    live: BTreeMap<(String, usize), usize>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Records {
    sessions: Vec<Tracked>,
}

impl From<Records> for Held {
    fn from(records: Records) -> Self {
        let mut index = Index::default();
        for (position, tracked) in records.sessions.iter().enumerate() {
            index
                .positions
                .entry(tracked.session.clone())
                .or_insert(position);
            if tracked.agent.is_some() && !tracked.stopped() && tracked.first().is_some() {
                index
                    .live
                    .insert((tracked.session.clone(), position), position);
            }
        }
        Self {
            sessions: records.sessions,
            index: Arc::new(index),
        }
    }
}

#[derive(Serialize)]
struct Sealing<'a> {
    format: &'static str,
    held: &'a Held,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Sealed {
    format: String,
    held: Held,
}

impl Held {
    /// The session named `session`.
    pub fn session(&self, session: &str) -> Option<&Tracked> {
        self.index
            .positions
            .get(session)
            .and_then(|position| self.sessions.get(*position))
    }

    /// Live agent sessions in identifier order, starting after the supplied identifier.
    pub fn live_ordered(&self, after: Bound<&str>) -> impl Iterator<Item = &Tracked> {
        let after = match after {
            Bound::Unbounded => Bound::Unbounded,
            Bound::Excluded(id) => Bound::Excluded((id.to_owned(), usize::MAX)),
            Bound::Included(id) => Bound::Included((id.to_owned(), 0)),
        };
        self.index
            .live
            .range((after, Bound::Unbounded))
            .map(|(_, position)| &self.sessions[*position])
    }

    /// The maintained number of live agent sessions.
    pub fn live_count(&self) -> usize {
        self.index.live.len()
    }

    /// The report kept under `operation`, with its session.
    pub fn operation(&self, operation: &str) -> Option<&Report> {
        self.sessions
            .iter()
            .flat_map(|tracked| tracked.reports.iter())
            .find(|report| report.operation == operation)
    }

    /// Fold one report. A report on a session never begun is refused by
    /// reason, since every kept report was checked against what came before.
    pub fn hold(&mut self, report: Report) -> Result<(), String> {
        if let Some(position) = self.index.positions.get(&report.session).copied() {
            let tracked = &mut self.sessions[position];
            tracked.reports.push(report);
            let index = Arc::make_mut(&mut self.index);
            let key = (tracked.session.clone(), position);
            if tracked.agent.is_some() && !tracked.stopped() {
                index.live.insert(key, position);
            } else {
                index.live.remove(&key);
            }
            return Ok(());
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
        let index = Arc::make_mut(&mut self.index);
        index
            .positions
            .insert(report.session.clone(), self.sessions.len());
        if report.agent.is_some() {
            index.live.insert(
                (report.session.clone(), self.sessions.len()),
                self.sessions.len(),
            );
        }
        self.sessions.push(Tracked {
            session: report.session.clone(),
            agent: report.agent.clone(),
            machine: report.machine.clone(),
            reports: vec![report],
        });
        Ok(())
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
        serde_json::to_vec(&Sealing {
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
        Ok(sealed.held)
    }
}
