//! Alarm records and the revisioned fold shared by writes and restart.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// A condition the service can establish from a runner and its proxy feed.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, utoipa::ToSchema,
)]
#[serde(rename_all = "snake_case")]
#[schema(as = AlarmKind)]
pub enum Kind {
    /// A live seat has stopped signalling.
    SeatStalled,
    /// A proxied session has no recorded usage or no run file.
    SessionUngoverned,
}

/// One occurrence, retained after acknowledgement and recovery.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = AlarmRecord)]
pub struct Alarm {
    /// The occurrence's stable identity.
    pub id: String,
    /// The condition raised.
    pub kind: Kind,
    /// The runner session.
    pub session: String,
    /// The agent holding that session.
    pub agent: String,
    /// The session's machine.
    pub machine: String,
    /// When raised, in Unix seconds.
    pub raised_at: u64,
    /// Why raised, and how it recovered when cleared.
    pub detail: String,
    /// The person who first acknowledged it.
    pub acknowledged_by: Option<String>,
    /// When acknowledged, in Unix seconds.
    pub acknowledged_at: Option<u64>,
    /// When recovery was observed, in Unix seconds.
    pub cleared_at: Option<u64>,
}

/// Named failures, never answered as an empty list.
#[derive(Debug, thiserror::Error)]
pub enum AlarmError {
    /// The durable store or its configuration could not be read or written.
    #[error("AlarmsUnavailable: {0}")]
    Unavailable(String),
    /// The caller supplied an invalid observation or configuration.
    #[error("AlarmMalformed: {0}")]
    Malformed(String),
    /// No visible record has this identity.
    #[error("AlarmUnknown")]
    Unknown,
}

/// Convert an underlying failure without dropping its reason.
pub(crate) fn unavailable(error: impl std::fmt::Display) -> AlarmError {
    AlarmError::Unavailable(error.to_string())
}

/// All changes in one evaluation form one durable operation.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Record {
    pub revision: u64,
    pub alarms: Vec<Alarm>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct Held {
    pub revision: u64,
    pub alarms: BTreeMap<String, Alarm>,
}

impl Held {
    pub fn hold(&mut self, record: Record) -> Result<(), AlarmError> {
        let expected = self
            .revision
            .checked_add(1)
            .ok_or_else(|| unavailable("alarm revision exhausted"))?;
        if record.revision != expected || record.alarms.is_empty() {
            return Err(unavailable(
                "alarm revision does not follow the held revision, or has no changes",
            ));
        }
        let mut ids = BTreeSet::new();
        for alarm in &record.alarms {
            if !ids.insert(&alarm.id)
                || [
                    &alarm.id,
                    &alarm.session,
                    &alarm.agent,
                    &alarm.machine,
                    &alarm.detail,
                ]
                .iter()
                .any(|value| value.trim().is_empty())
                || alarm.acknowledged_by.is_some() != alarm.acknowledged_at.is_some()
                || alarm
                    .acknowledged_by
                    .as_ref()
                    .is_some_and(|by| by.trim().is_empty())
                || alarm.acknowledged_at.is_some_and(|at| at < alarm.raised_at)
                || alarm.cleared_at.is_some_and(|at| at < alarm.raised_at)
            {
                return Err(unavailable(
                    "invalid alarm identity, acknowledgement or instant",
                ));
            }
            if let Some(prior) = self.alarms.get(&alarm.id)
                && (prior.kind != alarm.kind
                    || prior.session != alarm.session
                    || prior.agent != alarm.agent
                    || prior.machine != alarm.machine
                    || prior.raised_at != alarm.raised_at
                    || prior.cleared_at.is_some() && prior.cleared_at != alarm.cleared_at
                    || prior.acknowledged_by.is_some()
                        && (prior.acknowledged_by != alarm.acknowledged_by
                            || prior.acknowledged_at != alarm.acknowledged_at))
            {
                return Err(unavailable(
                    "alarm history changes an established occurrence",
                ));
            }
        }
        let mut next = self.alarms.clone();
        for alarm in record.alarms {
            next.insert(alarm.id.clone(), alarm);
        }
        let mut open = BTreeSet::new();
        for alarm in next.values().filter(|alarm| alarm.cleared_at.is_none()) {
            if !open.insert((alarm.session.as_str(), alarm.kind)) {
                return Err(unavailable(
                    "a session has two open alarms for the same condition",
                ));
            }
        }
        self.alarms = next;
        self.revision = record.revision;
        Ok(())
    }
}
