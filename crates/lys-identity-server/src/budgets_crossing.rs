//! A reached budget's crossings and what came of each act.
//!
//! A crossing is kept in the same leaf as the use that made it, so a use is
//! never charged without its crossing. Each names a stable operation id,
//! the budget (holder, measure, version and limit), the figure and the
//! instant, and the session the act goes to. What the runner answered is
//! kept after, under the same id: a crossing with nothing settled is asked
//! again under that id, never replaced. A periodic budget crosses when a
//! use takes its period's total from under the limit to at or over it; a
//! context budget crosses when a session's figure rises from under the
//! limit to at or over it.

use std::collections::BTreeMap;

use lys_runner::Ended;
use lys_runner::operations::{OperationOutcome, OperationState};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::budgets_state::{Act, Holder, Measure};
use crate::routes::hex;

/// One act a reached budget asks, kept before it is asked.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct Crossing {
    /// The act's stable operation id.
    pub operation: String,
    /// The budget's holder.
    pub holder: Holder,
    /// The budget's measure.
    pub measure: Measure,
    /// The budget's version.
    pub version: u64,
    /// The budget's limit.
    #[schema(value_type = f64)]
    pub limit: serde_json::Number,
    /// The figure that reached it; absent when context could not be measured.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<f64>)]
    pub figure: Option<serde_json::Number>,
    /// Why an unmeasured context requires a stop.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unavailable: Option<String>,
    /// The stable position within this holder version.
    #[serde(default)]
    pub limit_index: u64,
    /// A warning uses its own once-per-period identity and tells the responsible person.
    #[serde(default)]
    pub warning: bool,
    /// The paying account whose reported plan window reached the threshold.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub account: Option<String>,
    /// What the budget does.
    pub act: Act,
    /// The agent whose use crossed it.
    pub agent: String,
    /// The session the act goes to; none for a notice to the person.
    pub session: Option<String>,
    /// The words the act types: the compaction command or the notice.
    pub text: Option<String>,
    /// When, in milliseconds since the Unix epoch.
    pub at_ms: i64,
}

impl Crossing {
    /// An unavailable crossing names its cause and asks only a context stop.
    pub(crate) fn checked(&self) -> Result<(), String> {
        match (&self.figure, &self.unavailable) {
            (Some(_), None) => Ok(()),
            (None, Some(reason))
                if !reason.is_empty()
                    && self.measure == Measure::ContextPercent
                    && self.act == Act::Stop =>
            {
                Ok(())
            }
            _ => Err(
                "a crossing needs a measured figure or a named unavailable context Stop".to_owned(),
            ),
        }
    }

    /// The operation id of a crossing of `holder`'s `measure` budget at
    /// `version`, in the period or rise `mark` names, for `session`.
    pub fn id(
        holder: &Holder,
        measure: Measure,
        version: u64,
        mark: &str,
        session: &str,
    ) -> String {
        let named = format!(
            "lys-budget-crossing\n{:?}\n{}\n{measure:?}\n{version}\n{mark}\n{session}",
            holder.kind, holder.id
        );
        hex(&Sha256::digest(named.as_bytes()))[..32].to_owned()
    }
}

/// Where a crossing's act stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Stands {
    /// The runner accepted it and waits for the session's boundary.
    Accepted,
    /// Typed, or for a stop, the end sent.
    Delivered,
    /// Seen done: a compaction begun, a stop's exit seen.
    Confirmed,
    /// Whether it reached the session is unknown; ask again under the same id.
    Uncertain,
    /// Not done, by name.
    Refused,
    /// A notice kept for the responsible person to read; nothing is sent.
    Told,
}

/// What came of a crossing's act.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct Acted {
    /// The crossing's operation id.
    pub operation: String,
    /// Where it stands.
    pub stands: Stands,
    /// Why, in words.
    pub words: String,
    /// When it came to stand there, in milliseconds since the Unix epoch.
    pub at_ms: i64,
    /// The session's end, for a confirmed stop.
    #[schema(value_type = Option<Object>)]
    pub ended: Option<Ended>,
}

impl Acted {
    /// What the runner's `outcome` says of the crossing.
    pub fn from_runner(outcome: &OperationOutcome, at_ms: i64) -> Self {
        let stands = match outcome.state {
            OperationState::Accepted | OperationState::Delivering => Stands::Accepted,
            OperationState::Delivered => Stands::Delivered,
            OperationState::Confirmed => Stands::Confirmed,
            OperationState::Uncertain => Stands::Uncertain,
            OperationState::Refused => Stands::Refused,
        };
        Self {
            operation: outcome.operation.clone(),
            stands,
            words: outcome.words.clone(),
            at_ms,
            ended: outcome.ended.clone(),
        }
    }
}

/// The crossings as the log folds them.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Crossings {
    /// Every crossing, in the order kept.
    pub crossed: Vec<Crossing>,
    /// What came of each, by operation id.
    pub acted: BTreeMap<String, Acted>,
    /// Each session's last context figure.
    pub context: BTreeMap<String, u64>,
}

impl Crossings {
    /// Keep `crossing` unless its operation is kept already.
    pub fn hold(&mut self, crossing: Crossing) {
        if !self
            .crossed
            .iter()
            .any(|held| held.operation == crossing.operation)
        {
            self.crossed.push(crossing);
        }
    }

    /// The crossings whose act is not settled: never answered, accepted and
    /// waiting, or a stop sent whose exit is not yet seen.
    pub fn unsettled(&self) -> Vec<Crossing> {
        self.crossed
            .iter()
            .filter(|crossing| {
                #[cfg(test)]
                crate::budgets_work::visit(crate::budgets_work::Work::Crossing);
                match self.acted.get(&crossing.operation) {
                    None => true,
                    Some(acted) => {
                        matches!(acted.stands, Stands::Accepted | Stands::Uncertain)
                            || (acted.stands == Stands::Delivered && crossing.act == Act::Stop)
                    }
                }
            })
            .cloned()
            .collect()
    }

    /// Each crossing of `agent`, with what came of it: its receipt.
    pub fn of_agent(&self, agent: &str) -> Vec<Receipt> {
        self.crossed
            .iter()
            .filter(|crossing| {
                #[cfg(test)]
                crate::budgets_work::visit(crate::budgets_work::Work::Crossing);
                crossing.agent == agent
            })
            .map(|crossing| Receipt {
                crossing: crossing.clone(),
                acted: self.acted.get(&crossing.operation).cloned(),
            })
            .collect()
    }
}

/// A crossing and what came of it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
pub struct Receipt {
    /// The crossing, as kept before its act was asked.
    pub crossing: Crossing,
    /// What came of it; none while nothing is known.
    pub acted: Option<Acted>,
}
