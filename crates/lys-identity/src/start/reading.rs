//! Deriving a launch record's current state from withdrawals and verified session reports.

use super::LaunchRecords;
use crate::start::error::{StartError, json_string};
use crate::start::launch_record::LaunchRecord;
use crate::start::withdrawal::Withdrawal;
use lys_log_store::LeafStore;

/// A report the sessions brief d5055cc1 keeps, as this card reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionReport {
    /// The session the report made.
    pub session: String,
    /// The agent id the signed message names.
    pub agent: String,
    /// The launch record id the signed message names.
    pub launch_record: String,
    /// Whether the sessions brief verified it, signed with the agent's own key.
    pub verified: bool,
}

/// The sessions record: read, never written, copied or changed.
pub trait SessionReports {
    /// Every report naming `launch_record`.
    fn reports(&self, launch_record: &str) -> Vec<SessionReport>;
}

/// Where a launch record stands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reading {
    /// No verified report names it and no withdrawal does: the request stands.
    Unconfirmed,
    /// A verified report of the agent names it.
    Running {
        /// The session the report made.
        session: String,
    },
    /// A withdrawal names it and no verified report does.
    Withdrawn,
}

/// A launch record's state, and the withdrawal beside it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchState {
    /// The launch record.
    pub launch_record: String,
    /// The agent it names.
    pub agent: String,
    /// Where it stands.
    pub reading: Reading,
    /// Its withdrawal, when one names it.
    pub withdrawal: Option<Withdrawal>,
}

impl LaunchState {
    /// The state's name: `unconfirmed`, `running` or `withdrawn`.
    #[must_use]
    pub fn name(&self) -> &'static str {
        match self.reading {
            Reading::Unconfirmed => "unconfirmed",
            Reading::Running { .. } => "running",
            Reading::Withdrawn => "withdrawn",
        }
    }

    /// Whether the request stands: only while it reads unconfirmed.
    #[must_use]
    pub fn stands(&self) -> bool {
        self.reading == Reading::Unconfirmed
    }

    /// The state in words.
    #[must_use]
    pub fn words(&self) -> String {
        let record = &self.launch_record;
        let mut words = match &self.reading {
            Reading::Unconfirmed => format!(
                "Unconfirmed: no signed report names launch record {record} yet. The request stands. Asking another machine could start it twice: wait for its report, or withdraw it first."
            ),
            Reading::Running { session } => format!(
                "Running: the agent's verified signed report of session {session} names launch record {record}."
            ),
            Reading::Withdrawn => {
                format!("Withdrawn: launch record {record}'s request no longer stands.")
            }
        };
        if let Some(withdrawal) = &self.withdrawal {
            words = format!(
                "{words} Withdrawn by {} at {}.",
                withdrawal.by, withdrawal.at
            );
        }
        words
    }

    /// The state as the JSON the route answers.
    #[must_use]
    pub fn to_json(&self) -> String {
        let mut out = String::from("{\"launch_record\":");
        json_string(&mut out, &self.launch_record);
        out.push_str(",\"agent\":");
        json_string(&mut out, &self.agent);
        out.push_str(",\"state\":");
        json_string(&mut out, self.name());
        out.push_str(",\"words\":");
        json_string(&mut out, &self.words());
        out.push_str(",\"session\":");
        match &self.reading {
            Reading::Running { session } => json_string(&mut out, session),
            Reading::Unconfirmed | Reading::Withdrawn => out.push_str("null"),
        }
        out.push_str(",\"withdrawal\":");
        match &self.withdrawal {
            Some(withdrawal) => {
                out.push_str("{\"by\":");
                json_string(&mut out, &withdrawal.by);
                out.push_str(",\"at\":");
                out.push_str(&withdrawal.at.to_string());
                out.push('}');
            }
            None => out.push_str("null"),
        }
        out.push('}');
        out
    }
}

/// Derive `record`'s state from its withdrawal and the reports naming it.
pub fn derive(
    record: &LaunchRecord,
    withdrawal: Option<&Withdrawal>,
    sessions: &dyn SessionReports,
) -> LaunchState {
    let running = sessions.reports(&record.id).into_iter().find(|report| {
        report.verified && report.agent == record.agent && report.launch_record == record.id
    });
    let reading = match (running, withdrawal) {
        (Some(report), _) => Reading::Running {
            session: report.session,
        },
        (None, Some(_)) => Reading::Withdrawn,
        (None, None) => Reading::Unconfirmed,
    };
    LaunchState {
        launch_record: record.id.clone(),
        agent: record.agent.clone(),
        reading,
        withdrawal: withdrawal.cloned(),
    }
}

/// The state of the kept launch record `id`.
///
/// # Errors
/// - `StartError::LaunchRecordUnknown` if the store holds no launch record under the requested id.
pub fn state_of<S: LeafStore>(
    store: &LaunchRecords<S>,
    id: &str,
    sessions: &dyn SessionReports,
) -> Result<LaunchState, StartError> {
    let record = store
        .record(id)
        .ok_or_else(|| StartError::LaunchRecordUnknown {
            launch_record: id.to_owned(),
        })?;
    Ok(derive(record, store.withdrawal(id), sessions))
}

/// The launch record that stands unconfirmed for `agent`, when one does.
pub fn standing<S: LeafStore>(
    store: &LaunchRecords<S>,
    agent: &str,
    sessions: &dyn SessionReports,
) -> Option<String> {
    store
        .records()
        .iter()
        .filter(|record| record.agent == agent)
        .map(|record| derive(record, store.withdrawal(&record.id), sessions))
        .find(LaunchState::stands)
        .map(|state| state.launch_record)
}
