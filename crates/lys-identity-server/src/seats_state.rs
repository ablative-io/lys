//! What the seats' log folds to (AGENTS-002 R1), and how that fold is sealed
//! in the log's signed snapshot so a start reads only the leaves after it.
//!
//! A seat is a name an agent runs under on one machine from one reviewed
//! profile version. Its log keeps four kinds of line, each under the
//! operation id it was asked with: the seat added, a session started for
//! it, its session stopped, and a message sent to it. The same line sent
//! again under the same operation answers what was kept; the same operation
//! in other words is refused. Each line that changes a seat raises its
//! revision.

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::runner_acts::Digested;

/// The snapshot domain the seats' folded state is sealed under.
pub const DOMAIN: &str = "lys/identity/seats-state/v1";

const FORMAT: &str = "lys-seats-state/v1";

/// What a seat is when it is added, before any session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Added {
    /// The operation id it was added under.
    pub operation: String,
    /// Its name: 1 to 64 lowercase letters, digits and hyphens.
    pub name: String,
    /// The agent identity it runs as.
    pub agent: String,
    /// The harness its profile version declares, by the operator's name.
    pub harness: String,
    /// The reviewed profile version it starts from.
    pub profile_version: u32,
    /// The machine whose runner starts it.
    pub machine: String,
    /// The folder the harness starts in; absent, the profile's own default.
    pub working_folder: Option<String>,
    /// The account handle it runs under, when one is named.
    pub account: Option<String>,
    /// Who added it.
    pub by: String,
    /// When, in seconds since the Unix epoch.
    pub at: u64,
}

/// A session started for a seat.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Started {
    /// The operation id it was started under; the session is named by it.
    pub operation: String,
    /// The seat.
    pub name: String,
    /// The Lys session.
    pub session: String,
    /// The harness's own session id, when the runner knew it.
    pub harness_session: Option<String>,
    /// Who started it.
    pub by: String,
    /// When.
    pub at: u64,
}

/// A seat's session stopped through Lys.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Stopped {
    /// The operation id it was stopped under.
    pub operation: String,
    /// The seat.
    pub name: String,
    /// The session stopped.
    pub session: String,
    /// Whether force was named.
    pub forced: bool,
    /// Who stopped it.
    pub by: String,
    /// When.
    pub at: u64,
}

/// A message delivered to a seat's session as a user turn.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Sent {
    /// The operation id it was sent under.
    pub operation: String,
    /// The seat.
    pub name: String,
    /// The session it reached.
    pub session: String,
    /// The text, as its length and digest only.
    pub text: Digested,
    /// How it was sent: `send`, or `attach_type` from an attach.
    pub act: String,
    /// Who sent it.
    pub by: String,
    /// When.
    pub at: u64,
}

/// One line of the seats' log.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Line {
    /// A seat added.
    Added(Added),
    /// A session started for a seat.
    Started(Started),
    /// A seat's session stopped.
    Stopped(Stopped),
    /// A message sent to a seat.
    Sent(Sent),
}

impl Line {
    /// The operation id the line was asked under.
    pub fn operation(&self) -> &str {
        match self {
            Self::Added(line) => &line.operation,
            Self::Started(line) => &line.operation,
            Self::Stopped(line) => &line.operation,
            Self::Sent(line) => &line.operation,
        }
    }

    /// The seat the line is about.
    pub fn name(&self) -> &str {
        match self {
            Self::Added(line) => &line.name,
            Self::Started(line) => &line.name,
            Self::Stopped(line) => &line.name,
            Self::Sent(line) => &line.name,
        }
    }

    /// Whether `other` asks the same thing, whenever and by whom each was kept.
    pub fn same_words(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Added(kept), Self::Added(asked)) => {
                Added {
                    by: String::new(),
                    at: 0,
                    ..kept.clone()
                } == Added {
                    by: String::new(),
                    at: 0,
                    ..asked.clone()
                }
            }
            (Self::Started(kept), Self::Started(asked)) => kept.name == asked.name,
            (Self::Stopped(kept), Self::Stopped(asked)) => kept.name == asked.name,
            (Self::Sent(kept), Self::Sent(asked)) => {
                kept.name == asked.name && kept.text == asked.text && kept.act == asked.act
            }
            _ => false,
        }
    }
}

/// A seat as its lines fold it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Seat {
    /// What it was added as.
    pub added: Added,
    /// Its latest session; none before its first start.
    pub session: Option<String>,
    /// The harness's own id for that session, when known.
    pub harness_session: Option<String>,
    /// Whether its latest session was started and not stopped through Lys.
    pub running: bool,
    /// Raised by every line that changes it, from 1.
    pub revision: u64,
}

/// The seats as their log folds them, in the order added.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, from = "Records")]
pub struct Held {
    /// The seats.
    pub seats: Vec<Seat>,
    /// Every line, by the operation id it was asked under.
    pub operations: BTreeMap<String, Line>,
    #[serde(skip)]
    index: Arc<HashMap<String, usize>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Records {
    seats: Vec<Seat>,
    operations: BTreeMap<String, Line>,
}

impl From<Records> for Held {
    fn from(records: Records) -> Self {
        let index = records
            .seats
            .iter()
            .enumerate()
            .map(|(position, seat)| (seat.added.name.clone(), position))
            .collect();
        Self {
            seats: records.seats,
            operations: records.operations,
            index: Arc::new(index),
        }
    }
}

#[derive(Serialize)]
struct Sealing<'a> {
    format: &'static str,
    held: &'a Held,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Sealed {
    format: String,
    held: Held,
}

impl Held {
    /// The seat named `name`.
    pub fn seat(&self, name: &str) -> Option<&Seat> {
        self.index
            .get(name)
            .and_then(|position| self.seats.get(*position))
    }

    /// The line kept under `operation`.
    pub fn operation(&self, operation: &str) -> Option<&Line> {
        self.operations.get(operation)
    }

    /// Fold one line, refused by reason when its operation is spent or its
    /// seat does not stand as the line needs.
    pub fn hold(&mut self, line: Line) -> Result<(), String> {
        if self.operations.contains_key(line.operation()) {
            return Err(format!(
                "operation `{}` already names a seat line",
                line.operation()
            ));
        }
        let name = line.name().to_owned();
        match &line {
            Line::Added(added) => {
                if self.index.contains_key(&name) {
                    return Err(format!("seat `{name}` is already added"));
                }
                Arc::make_mut(&mut self.index).insert(name, self.seats.len());
                self.seats.push(Seat {
                    added: added.clone(),
                    session: None,
                    harness_session: None,
                    running: false,
                    revision: 1,
                });
            }
            Line::Started(started) => {
                let seat = self.changed(&name)?;
                seat.session = Some(started.session.clone());
                seat.harness_session.clone_from(&started.harness_session);
                seat.running = true;
                seat.revision += 1;
            }
            Line::Stopped(stopped) => {
                let seat = self.changed(&name)?;
                if seat.session.as_deref() != Some(stopped.session.as_str()) {
                    return Err(format!(
                        "seat `{name}` does not hold session `{}`",
                        stopped.session
                    ));
                }
                seat.running = false;
                seat.revision += 1;
            }
            Line::Sent(sent) => {
                let seat = self.changed(&name)?;
                if seat.session.as_deref() != Some(sent.session.as_str()) {
                    return Err(format!(
                        "seat `{name}` does not hold session `{}`",
                        sent.session
                    ));
                }
            }
        }
        self.operations.insert(line.operation().to_owned(), line);
        Ok(())
    }

    fn changed(&mut self, name: &str) -> Result<&mut Seat, String> {
        let position = *self
            .index
            .get(name)
            .ok_or_else(|| format!("no seat is named `{name}`"))?;
        self.seats
            .get_mut(position)
            .ok_or_else(|| format!("seat `{name}` is indexed where no seat is held"))
    }

    /// Fold every leaf of `tail`, in order.
    pub fn fold(&mut self, tail: &lys_log_store::Tail) -> Result<(), String> {
        for (index, bytes) in (tail.from..).zip(&tail.leaves) {
            let line = serde_json::from_slice(bytes)
                .map_err(|error| format!("leaf {index} is not a seat line: {error}"))?;
            self.hold(line)
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
        .map_err(|error| format!("seats state: {error}"))
    }

    /// The state a snapshot sealed, refused by reason unless it reads whole
    /// in this format.
    pub fn decode(bytes: &[u8]) -> Result<Self, String> {
        let sealed: Sealed =
            serde_json::from_slice(bytes).map_err(|error| format!("seats state: {error}"))?;
        if sealed.format != FORMAT {
            return Err(format!(
                "seats state is in format {}, not {FORMAT}",
                sealed.format
            ));
        }
        Ok(sealed.held)
    }
}

#[cfg(test)]
mod tests {
    use super::{Added, Held, Line, Started, Stopped};

    fn added(operation: &str, name: &str) -> Line {
        Line::Added(Added {
            operation: operation.to_owned(),
            name: name.to_owned(),
            agent: "agent".to_owned(),
            harness: "claude".to_owned(),
            profile_version: 1,
            machine: "op-1".to_owned(),
            working_folder: None,
            account: None,
            by: "person".to_owned(),
            at: 1,
        })
    }

    #[test]
    fn a_seat_folds_its_sessions_and_raises_its_revision() -> Result<(), String> {
        let mut held = Held::default();
        held.hold(added("op-a", "waffles"))?;
        held.hold(Line::Started(Started {
            operation: "op-b".to_owned(),
            name: "waffles".to_owned(),
            session: "op-b".to_owned(),
            harness_session: Some("uuid".to_owned()),
            by: "person".to_owned(),
            at: 2,
        }))?;
        let seat = held.seat("waffles").ok_or("no seat")?;
        assert!(seat.running);
        assert_eq!(seat.revision, 2);
        assert_eq!(seat.session.as_deref(), Some("op-b"));
        held.hold(Line::Stopped(Stopped {
            operation: "op-c".to_owned(),
            name: "waffles".to_owned(),
            session: "op-b".to_owned(),
            forced: false,
            by: "person".to_owned(),
            at: 3,
        }))?;
        let seat = held.seat("waffles").ok_or("no seat")?;
        assert!(!seat.running);
        assert_eq!(seat.revision, 3);
        let sealed = held.encode()?;
        assert_eq!(Held::decode(&sealed)?, held);
        assert_eq!(Held::decode(&sealed)?.seat("waffles"), held.seat("waffles"));
        Ok(())
    }

    #[test]
    fn a_name_or_an_operation_is_held_once() -> Result<(), String> {
        let mut held = Held::default();
        held.hold(added("op-a", "waffles"))?;
        assert!(held.hold(added("op-b", "waffles")).is_err());
        assert!(held.hold(added("op-a", "other")).is_err());
        assert!(held.seat("other").is_none());
        Ok(())
    }
}
