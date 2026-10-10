//! The counted work of every seat-owner hot path and its ratchet
//! (AGENTS-004 R5).
//!
//! Work is counted, never timed: named calls, keyed record visits, copied
//! bytes, journal appends, physical syncs and idle wakes. A checked-in ratchet
//! (`tests/seat_cost_ratchet.json`) names the inputs each figure was taken at
//! and the ceiling each counter may reach; a change is accepted at equal or
//! lower counts only. Raising a ceiling is a change to that file, read in a
//! review; renaming a counter to hide work fails the ratchet's read, since
//! its counters are exactly [`Work`]'s members and nothing else.
//!
//! Durations are kept beside counts as diagnostic evidence by the tests that
//! take them; no gate here reads a clock.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};

use super::store::StoreCounts;
use crate::error::RunnerError;

/// The ratchet file's format.
pub const RATCHET_FORMAT: &str = "lys-runner-seat-cost-ratchet/v1";

/// A hot path with a counted ceiling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HotPath {
    /// One act forwarded through the home proxy to the owner.
    ProxyForward,
    /// One harness hook line taken in and persisted.
    HookIngest,
    /// One control operation delivered to the harness and receipted.
    ControlDelivery,
    /// One keyed read of the owner registry.
    RegistryRead,
    /// One replacement client rebinding to a live owner.
    Reconnect,
    /// One custody transfer: establish, stop, exit, retire.
    CustodyTransfer,
}

impl HotPath {
    /// Every path, in the order the ratchet lists them.
    pub const ALL: [Self; 6] = [
        Self::ProxyForward,
        Self::HookIngest,
        Self::ControlDelivery,
        Self::RegistryRead,
        Self::Reconnect,
        Self::CustodyTransfer,
    ];

    /// The path's name in the ratchet file.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::ProxyForward => "proxy_forward",
            Self::HookIngest => "hook_ingest",
            Self::ControlDelivery => "control_delivery",
            Self::RegistryRead => "registry_read",
            Self::Reconnect => "reconnect",
            Self::CustodyTransfer => "custody_transfer",
        }
    }
}

/// The work one path did, every counter named. A ratchet ceiling has the
/// same members, so a counter cannot be dropped or renamed without the
/// ratchet refusing to read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Work {
    /// Named calls made (store intents, socket round trips, lookups).
    pub calls: u64,
    /// Owner records and journal lines visited by key or by tail.
    pub record_visits: u64,
    /// Bytes read from or written to a file or a socket.
    pub bytes_copied: u64,
    /// Journal lines appended.
    pub journal_appends: u64,
    /// Physical disk syncs issued.
    pub syncs: u64,
    /// Times an idle waiter woke to do nothing.
    pub wakes: u64,
}

impl Work {
    /// The work between two readings of a store's counts, with `calls` the
    /// intents the caller sent in between.
    #[must_use]
    pub fn between(before: StoreCounts, after: StoreCounts, calls: u64) -> Self {
        Self {
            calls,
            record_visits: after.record_visits.saturating_sub(before.record_visits),
            bytes_copied: after.bytes_copied.saturating_sub(before.bytes_copied),
            journal_appends: after.journal_appends.saturating_sub(before.journal_appends),
            syncs: after.syncs.saturating_sub(before.syncs),
            wakes: 0,
        }
    }

    /// Every counter, named, for a readback.
    #[must_use]
    pub fn named(self) -> [(&'static str, u64); 6] {
        [
            ("calls", self.calls),
            ("record_visits", self.record_visits),
            ("bytes_copied", self.bytes_copied),
            ("journal_appends", self.journal_appends),
            ("syncs", self.syncs),
            ("wakes", self.wakes),
        ]
    }

    /// Whether every counter of `self` is at or under `ceiling`'s.
    #[must_use]
    pub fn fits(self, ceiling: Self) -> bool {
        self.excess(ceiling).is_none()
    }

    /// The first counter over `ceiling`, if any.
    #[must_use]
    pub fn excess(self, ceiling: Self) -> Option<Excess> {
        self.named()
            .into_iter()
            .zip(ceiling.named())
            .find(|((_, observed), (_, limit))| observed > limit)
            .map(|((counter, observed), (_, limit))| Excess {
                counter,
                observed,
                ceiling: limit,
            })
    }
}

/// One counter over its ceiling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Excess {
    /// The counter's name.
    pub counter: &'static str,
    /// What was counted.
    pub observed: u64,
    /// What the ratchet permits.
    pub ceiling: u64,
}

/// A live meter the owner process counts into while it runs, read by the
/// tests that hold its signals closed. Idle work is a wake; every counter
/// stays at zero while nothing arrives.
#[derive(Debug, Default)]
pub struct Meter {
    calls: AtomicU64,
    record_visits: AtomicU64,
    bytes_copied: AtomicU64,
    journal_appends: AtomicU64,
    syncs: AtomicU64,
    wakes: AtomicU64,
}

impl Meter {
    /// One named call.
    pub fn call(&self) {
        self.calls.fetch_add(1, Ordering::Relaxed);
    }

    /// `n` keyed record visits.
    pub fn visit(&self, n: u64) {
        self.record_visits.fetch_add(n, Ordering::Relaxed);
    }

    /// `n` bytes copied.
    pub fn copied(&self, n: u64) {
        self.bytes_copied.fetch_add(n, Ordering::Relaxed);
    }

    /// One journal append with its sync.
    pub fn appended(&self) {
        self.journal_appends.fetch_add(1, Ordering::Relaxed);
        self.syncs.fetch_add(1, Ordering::Relaxed);
    }

    /// One wake that found nothing to do.
    pub fn woke(&self) {
        self.wakes.fetch_add(1, Ordering::Relaxed);
    }

    /// The counters now.
    #[must_use]
    pub fn snapshot(&self) -> Work {
        Work {
            calls: self.calls.load(Ordering::Relaxed),
            record_visits: self.record_visits.load(Ordering::Relaxed),
            bytes_copied: self.bytes_copied.load(Ordering::Relaxed),
            journal_appends: self.journal_appends.load(Ordering::Relaxed),
            syncs: self.syncs.load(Ordering::Relaxed),
            wakes: self.wakes.load(Ordering::Relaxed),
        }
    }
}

/// The checked-in ratchet: the inputs each ceiling was taken at, and the
/// ceiling for every path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ratchet {
    /// [`RATCHET_FORMAT`].
    pub format: String,
    /// The commit the ceilings were taken at, as its reader finds it.
    pub taken_at: String,
    /// Where the figures came from: a measured battery, or a reading of the
    /// counting code before any battery ran.
    pub provenance: String,
    /// The fixed inputs: owners live, tail lines, frame bytes and the like.
    pub inputs: BTreeMap<String, u64>,
    /// The ceiling per path, every path present.
    pub ceilings: BTreeMap<HotPath, Work>,
}

impl Ratchet {
    /// Reads a ratchet file's bytes.
    ///
    /// # Errors
    ///
    /// `seat_cost_ratchet_invalid` when the bytes do not read, name another
    /// format, or miss a path.
    pub fn parse(bytes: &str) -> Result<Self, RunnerError> {
        let ratchet: Self = serde_json::from_str(bytes).map_err(|error| {
            RunnerError::refused(
                "seat_cost_ratchet_invalid",
                format!("the ratchet does not read: {error}"),
            )
        })?;
        if ratchet.format != RATCHET_FORMAT {
            return Err(RunnerError::refused(
                "seat_cost_ratchet_invalid",
                format!(
                    "the ratchet is in format {}, not {RATCHET_FORMAT}",
                    ratchet.format
                ),
            ));
        }
        if let Some(missing) = HotPath::ALL
            .iter()
            .find(|path| !ratchet.ceilings.contains_key(path))
        {
            return Err(RunnerError::refused(
                "seat_cost_ratchet_invalid",
                format!("the ratchet names no ceiling for {}", missing.name()),
            ));
        }
        Ok(ratchet)
    }

    /// The ceiling for `path`; every path has one once parsed.
    #[must_use]
    pub fn ceiling(&self, path: HotPath) -> Work {
        self.ceilings.get(&path).copied().unwrap_or_default()
    }

    /// Accepts `observed` for `path` at equal or lower counts only.
    ///
    /// # Errors
    ///
    /// `seat_cost_over_ceiling`, naming the path and the first counter over.
    pub fn check(&self, path: HotPath, observed: Work) -> Result<(), RunnerError> {
        match observed.excess(self.ceiling(path)) {
            None => Ok(()),
            Some(excess) => Err(RunnerError::refused(
                "seat_cost_over_ceiling",
                format!(
                    "{} counted {} {}, over the ceiling of {}",
                    path.name(),
                    excess.observed,
                    excess.counter,
                    excess.ceiling
                ),
            )),
        }
    }
}
