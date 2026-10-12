//! Evaluation and durable alarm storage; no independent polling task.

use super::alarms_state::{Alarm, AlarmError, Held, Kind, Record, unavailable};
use lys_log_store::{FileLeafStore, FrontierLog};
use std::path::{Path, PathBuf};

#[path = "alarms/configuration.rs"]
pub mod configuration;
#[path = "alarms/follow.rs"]
pub mod follow;

/// Mutually exclusive proxy evidence for one run.
#[derive(Debug, Clone, Copy)]
pub enum ProxyEvidence {
    /// The session reaches its provider directly.
    Direct,
    /// Its declared proxy run has no file.
    MissingFile,
    /// The file exists but no whole usage line has arrived.
    AwaitingUsage,
    /// A whole usage line exists for the run.
    UsageSeen,
}

/// One session's evidence, read in the existing usage follow pass.
#[derive(Debug, Clone)]
pub struct Observation {
    /// The runner session.
    pub session: String,
    /// Its agent.
    pub agent: String,
    /// Its machine.
    pub machine: String,
    /// Whether its runner says it is live.
    pub live: bool,
    /// Its start, in Unix milliseconds.
    pub started_at_ms: u64,
    /// The runner's latest signal, in Unix milliseconds.
    pub last_signal_at_ms: Option<u64>,
    /// The launch's proxy declaration and the evidence for its run.
    pub proxy: ProxyEvidence,
}

/// The revisioned alarm log; opening it replays authoritative records.
pub struct AlarmStore {
    dir: PathBuf,
    log: FrontierLog<FileLeafStore>,
    held: Held,
}

impl AlarmStore {
    /// Open or create the alarm log without changing any existing stored shape.
    ///
    /// # Errors
    /// Refuses an unreadable log or invalid history.
    pub fn open(dir: &Path) -> Result<Self, AlarmError> {
        if !dir.exists() {
            FileLeafStore::create(dir, "lys/identity/alarms").map_err(unavailable)?;
        }
        let (log, tail) = FrontierLog::open(FileLeafStore::open(dir).map_err(unavailable)?)
            .map_err(unavailable)?;
        let mut held = Held::default();
        for bytes in tail.leaves {
            held.hold(serde_json::from_slice(&bytes).map_err(unavailable)?)?;
        }
        Ok(Self {
            dir: dir.to_owned(),
            log,
            held,
        })
    }

    /// The durable revision, one per changing evaluation or acknowledgement.
    #[must_use]
    pub fn revision(&self) -> u64 {
        self.held.revision
    }

    /// All retained occurrences, open first and newest first within each group.
    #[must_use]
    pub fn list(&self) -> Vec<Alarm> {
        let mut alarms: Vec<_> = self.held.alarms.values().cloned().collect();
        alarms.sort_by(|left, right| {
            left.cleared_at
                .is_some()
                .cmp(&right.cleared_at.is_some())
                .then(right.raised_at.cmp(&left.raised_at))
                .then(right.id.cmp(&left.id))
        });
        alarms
    }

    /// One retained occurrence.
    ///
    /// # Errors
    /// Refuses an unknown identity.
    pub fn get(&self, id: &str) -> Result<Alarm, AlarmError> {
        self.held.alarms.get(id).cloned().ok_or(AlarmError::Unknown)
    }

    fn keep(&mut self, alarms: Vec<Alarm>) -> Result<(), AlarmError> {
        if alarms.is_empty() {
            return Ok(());
        }
        let revision = self
            .held
            .revision
            .checked_add(1)
            .ok_or_else(|| unavailable("alarm revision exhausted"))?;
        let record = Record { revision, alarms };
        let bytes = serde_json::to_vec(&record).map_err(unavailable)?;
        let mut next = self.held.clone();
        next.hold(record)?;
        let index = self.log.len();
        if let Err(failure) = self.log.append(&bytes) {
            let reopened = Self::open(&self.dir)?;
            let kept = reopened.log.leaf_bytes(index).map_err(unavailable)?;
            *self = reopened;
            return if kept.as_deref() == Some(bytes.as_slice()) {
                Ok(())
            } else {
                Err(unavailable(failure))
            };
        }
        self.held = next;
        Ok(())
    }

    /// Record the first acknowledgement, without clearing the condition.
    ///
    /// # Errors
    /// Refuses an unknown alarm, invalid person or instant, or failed durable write.
    pub fn acknowledge(&mut self, id: &str, by: &str, at: u64) -> Result<Alarm, AlarmError> {
        let mut alarm = self.get(id)?;
        if by.trim().is_empty() || at < alarm.raised_at {
            return Err(AlarmError::Malformed(
                "acknowledgement needs a person and an instant after the raise".to_owned(),
            ));
        }
        if alarm.acknowledged_by.is_none() {
            alarm.acknowledged_by = Some(by.to_owned());
            alarm.acknowledged_at = Some(at);
            self.keep(vec![alarm.clone()])?;
        }
        Ok(alarm)
    }

    /// Evaluate observed sessions once per crossing; recoveries retain their records.
    ///
    /// # Errors
    /// Refuses malformed evidence, invalid thresholds or a failed durable write.
    pub fn observe(
        &mut self,
        observations: &[Observation],
        at_ms: u64,
        stall_seconds: u64,
    ) -> Result<bool, AlarmError> {
        let threshold = stall_seconds
            .checked_mul(1000)
            .filter(|value| *value > 0)
            .ok_or_else(|| {
                AlarmError::Malformed(
                    "stall_after_seconds must be positive and fit milliseconds".to_owned(),
                )
            })?;
        let revision = self
            .held
            .revision
            .checked_add(1)
            .ok_or_else(|| unavailable("alarm revision exhausted"))?;
        let mut changes = Vec::new();
        let mut sessions = std::collections::BTreeSet::new();
        for seen in observations {
            if [&seen.session, &seen.agent, &seen.machine]
                .iter()
                .any(|value| value.trim().is_empty())
                || !sessions.insert(&seen.session)
                || seen.started_at_ms > at_ms
                || seen
                    .last_signal_at_ms
                    .is_some_and(|signal| signal > at_ms || signal < seen.started_at_ms)
            {
                return Err(AlarmError::Malformed(
                    "session identity or signal instant is invalid".to_owned(),
                ));
            }
            let stale = at_ms - seen.last_signal_at_ms.unwrap_or(seen.started_at_ms) > threshold;
            let ungoverned = match seen.proxy {
                ProxyEvidence::MissingFile => true,
                ProxyEvidence::AwaitingUsage => at_ms - seen.started_at_ms > threshold,
                ProxyEvidence::Direct | ProxyEvidence::UsageSeen => false,
            };
            for (kind, active, detail, recovery) in [
                (
                    Kind::SeatStalled,
                    seen.live && stale,
                    "seat has stopped signalling",
                    "a signal arrived or the session ended",
                ),
                (
                    Kind::SessionUngoverned,
                    seen.live && ungoverned,
                    if matches!(seen.proxy, ProxyEvidence::MissingFile) {
                        "proxy run key has no usage file"
                    } else {
                        "proxy run has no usage line after its start allowance"
                    },
                    "proxy usage appeared or the session ended",
                ),
            ] {
                let prior = self.held.alarms.values().find(|alarm| {
                    alarm.session == seen.session
                        && alarm.kind == kind
                        && alarm.cleared_at.is_none()
                });
                match (active, prior) {
                    (true, None) => changes.push(Alarm {
                        id: format!("alarm-{revision}-{}", changes.len() + 1),
                        kind,
                        session: seen.session.clone(),
                        agent: seen.agent.clone(),
                        machine: seen.machine.clone(),
                        raised_at: at_ms / 1000,
                        detail: detail.to_owned(),
                        acknowledged_by: None,
                        acknowledged_at: None,
                        cleared_at: None,
                    }),
                    (false, Some(prior)) => {
                        let mut cleared = prior.clone();
                        cleared.cleared_at = Some(at_ms / 1000);
                        cleared.detail.push_str("; cleared: ");
                        cleared.detail.push_str(recovery);
                        changes.push(cleared);
                    }
                    _ => {}
                }
            }
        }
        let changed = !changes.is_empty();
        self.keep(changes)?;
        Ok(changed)
    }
}
