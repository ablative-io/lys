//! What the emergency stops' log folds to, and how that fold is sealed in
//! the log's signed snapshot so a start reads only the leaves after it.
//!
//! A stop is kept twice: as asked, before its first part, binding its words
//! to its operation id; and whole, with everything it did, once every part
//! is done. So the same stop sent again answers exactly what was kept, and
//! the same operation in other words is refused, even for a stop cut off
//! between its parts.

use serde::{Deserialize, Serialize};

/// The snapshot domain the stops' folded state is sealed under.
pub const DOMAIN: &str = "lys/identity/stops-state/v1";

const FORMAT: &str = "lys-stops-state/v1";

/// One emergency stop, as it was done.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Stop {
    /// The operation id it was sent under, which names it.
    pub operation: String,
    /// The agent stopped.
    pub agent: String,
    /// The person who stopped it.
    pub by: String,
    /// Why, in their words.
    pub reason: String,
    /// When, in seconds since the Unix epoch.
    pub at: u64,
    /// The serials of the certificates withdrawn.
    pub certificates_withdrawn: Vec<String>,
    /// The sessions whose runtimes were asked to end them.
    pub sessions_asked: Vec<String>,
    /// The credential handles ended at the broker; null when the broker
    /// refused.
    pub credentials_ended: Option<Vec<String>>,
    /// The broker's refusal, by name, when the handles were not ended.
    pub credentials_refused: Option<String>,
    /// Whether every part is done; false while the stop is only asked.
    #[serde(default = "done")]
    pub done: bool,
}

fn done() -> bool {
    true
}

impl Stop {
    /// Whether `other` is the same stop in the same words, whenever each was
    /// sent and whatever each found to do.
    pub fn same_words(&self, other: &Self) -> bool {
        self.operation == other.operation
            && self.agent == other.agent
            && self.by == other.by
            && self.reason == other.reason
    }
}

/// The stops as their log folds them, in the order kept.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Held {
    /// The stops.
    pub stops: Vec<Stop>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Sealed {
    format: String,
    held: Held,
}

impl Held {
    /// The stop kept under `operation`.
    pub fn operation(&self, operation: &str) -> Option<&Stop> {
        self.stops.iter().find(|stop| {
            #[cfg(test)]
            crate::folded_work::visit(crate::folded_work::Work::Stop);
            stop.operation == operation
        })
    }

    /// Every stop kept on `agent`, in the order kept.
    pub fn of_agent<'a>(&'a self, agent: &'a str) -> impl Iterator<Item = &'a Stop> + 'a {
        self.stops.iter().filter(move |stop| {
            #[cfg(test)]
            crate::folded_work::visit(crate::folded_work::Work::Stop);
            stop.agent == agent
        })
    }

    /// Fold one stop. A second stop under an operation already kept is
    /// refused, since every kept stop was checked against what came before.
    pub fn hold(&mut self, stop: Stop) -> Result<(), String> {
        let held = self.stops.iter_mut().find(|held| {
            #[cfg(test)]
            crate::folded_work::visit(crate::folded_work::Work::Stop);
            held.operation == stop.operation
        });
        match held {
            None => self.stops.push(stop),
            Some(asked) if !asked.done && stop.done && asked.same_words(&stop) => *asked = stop,
            Some(_) => {
                return Err(format!(
                    "operation `{}` already names a stop",
                    stop.operation
                ));
            }
        }
        Ok(())
    }

    /// Fold every leaf of `tail`, in order.
    pub fn fold(&mut self, tail: &lys_log_store::Tail) -> Result<(), String> {
        for (index, bytes) in (tail.from..).zip(&tail.leaves) {
            let stop = serde_json::from_slice(bytes)
                .map_err(|error| format!("leaf {index} is not a stop: {error}"))?;
            self.hold(stop)
                .map_err(|reason| format!("leaf {index}: {reason}"))?;
        }
        Ok(())
    }

    /// The state a snapshot seals.
    pub fn encode(&self) -> Result<Vec<u8>, String> {
        serde_json::to_vec(&Sealed {
            format: FORMAT.to_owned(),
            held: self.clone(),
        })
        .map_err(|error| format!("stops state: {error}"))
    }

    /// The state a snapshot sealed, refused by reason unless it reads whole
    /// in this format.
    pub fn decode(bytes: &[u8]) -> Result<Self, String> {
        let sealed: Sealed =
            serde_json::from_slice(bytes).map_err(|error| format!("stops state: {error}"))?;
        if sealed.format != FORMAT {
            return Err(format!(
                "stops state is in format {}, not {FORMAT}",
                sealed.format
            ));
        }
        Ok(sealed.held)
    }
}
