//! A consumer's index of the grant change stream (ACCESS-006 R2, with the
//! DIRECTORY-089 R1 stream it reads).
//!
//! A membership consumer keeps one [`MembershipIndex`] per identified grant
//! log. It starts from one trusted [`ChangeFrame::Baseline`] and applies the
//! stream's frames strictly in revision order: each change is verified
//! against the service key and its own frame before it moves the cursor, a
//! watermark moves the cursor over changes the consumer may not see, and an
//! identical duplicate applies nothing. Readiness is published only by a
//! [`ChangeFrame::Ready`] that names exactly the cursor the index holds, on
//! the log the index was made for; an omitted revision, a reset, a foreign
//! log, a bad signature, a conflicting duplicate, a disagreeing Ready or a
//! transport failure withdraws it, by name.
//!
//! [`MembershipIndex::admit`] is the admission fence a consumer asks before
//! acting on a pass binding: it admits only while ready, at or past the
//! binding's required revision, and while no grant on the dependency's path
//! is revoked. Existing connections are rechecked by the consumer with the
//! same call once it is ready again; the index holds no clock and calls
//! nothing.

use std::collections::{BTreeMap, BTreeSet};
use std::num::NonZeroUsize;

use lys_core::merkle::raw_leaf_hash;

use super::change_stream::{ChangeFrame, ChangeKind, LogName, ResetReason, event_bytes};
use super::events::{GrantChange, verify_grant_event};

/// A revision was omitted: the stream must be read again from the cursor.
pub const STREAM_GAP: &str = "grant_stream_gap";
/// The cursor cannot be continued: a baseline must be read again.
pub const STREAM_RESET: &str = "grant_stream_reset_required";
/// A frame names another grant log or epoch than the index's.
pub const STREAM_FOREIGN_LOG: &str = "grant_stream_foreign_log";
/// A frame's evidence is not the signed change it names, or conflicts with
/// what was applied at its revision.
pub const STREAM_INTEGRITY: &str = "grant_stream_integrity";
/// A Ready frame names another revision than the index's cursor.
pub const STREAM_READY_DISAGREES: &str = "grant_stream_ready_disagrees";
/// The index is not ready: no admission is made.
pub const STREAM_NOT_READY: &str = "grant_stream_not_ready";
/// The index has not reached the revision a binding requires.
pub const STREAM_BEHIND: &str = "grant_stream_behind_required_revision";
/// A grant a right depends on is revoked.
pub const DEPENDENCY_REVOKED: &str = "grant_dependency_revoked";

/// A frame or an admission refused by name. A refused frame withdraws readiness.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{name}: {reason}")]
pub struct StreamRefusal {
    /// The stable name.
    pub name: &'static str,
    /// Its words.
    pub reason: String,
    /// How many revisions were omitted, for a gap.
    pub gap: Option<u64>,
}

fn refusal(name: &'static str, reason: impl Into<String>) -> StreamRefusal {
    StreamRefusal {
        name,
        reason: reason.into(),
        gap: None,
    }
}

/// What one applied frame did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Applied {
    /// A baseline replaced the cursor.
    Baseline {
        /// The baseline's revision.
        revision: u64,
    },
    /// A verified change moved the cursor.
    Change {
        /// Its revision.
        revision: u64,
        /// What it did.
        change: ChangeKind,
    },
    /// An identical duplicate: nothing applied.
    Duplicate {
        /// Its revision.
        revision: u64,
    },
    /// A watermark moved the cursor, or was already passed.
    Watermark {
        /// The cursor after it.
        revision: u64,
    },
    /// The index is ready through `revision`.
    Ready {
        /// The revision delivered and projected.
        revision: u64,
    },
}

/// A consumer's index of one identified grant log.
#[derive(Debug)]
pub struct MembershipIndex {
    log: LogName,
    key: [u8; 32],
    cursor: Option<u64>,
    ready: bool,
    revoked: BTreeMap<String, u64>,
    evidence: BTreeMap<u64, [u8; 32]>,
    window: NonZeroUsize,
}

impl MembershipIndex {
    /// An index of `log`, whose events the service key `key` signs, holding
    /// the evidence of the last `window` applied changes to tell a duplicate
    /// from a conflict. It holds no cursor and is not ready until a baseline
    /// and a Ready are applied.
    #[must_use]
    pub fn new(log: LogName, key: [u8; 32], window: NonZeroUsize) -> Self {
        Self {
            log,
            key,
            cursor: None,
            ready: false,
            revoked: BTreeMap::new(),
            evidence: BTreeMap::new(),
            window,
        }
    }

    /// The log this index reads.
    #[must_use]
    pub fn log(&self) -> &LogName {
        &self.log
    }

    /// The last revision applied, the cursor a read continues from.
    #[must_use]
    pub fn cursor(&self) -> Option<u64> {
        self.cursor
    }

    /// Whether the index is ready.
    #[must_use]
    pub fn is_ready(&self) -> bool {
        self.ready
    }

    /// The revision `grant` was revoked at, if it is.
    #[must_use]
    pub fn revoked_at(&self, grant: &str) -> Option<u64> {
        self.revoked.get(grant).copied()
    }

    /// How many grants the index holds revoked.
    #[must_use]
    pub fn revoked_count(&self) -> usize {
        self.revoked.len()
    }

    /// Withdraw readiness after a transport or authority failure.
    pub fn withdraw(&mut self) {
        self.ready = false;
    }

    /// Apply every frame of one page of `log`, stopping at the first refusal.
    ///
    /// # Errors
    /// The first frame's refusal; readiness is withdrawn.
    pub fn apply_page(
        &mut self,
        log: &LogName,
        frames: &[ChangeFrame],
    ) -> Result<Vec<Applied>, StreamRefusal> {
        frames.iter().map(|frame| self.apply(log, frame)).collect()
    }

    /// Apply one frame of `log`.
    ///
    /// # Errors
    /// The frame's refusal, by name; readiness is withdrawn.
    pub fn apply(&mut self, log: &LogName, frame: &ChangeFrame) -> Result<Applied, StreamRefusal> {
        let applied = self.judge(log, frame);
        if applied.is_err() {
            self.ready = false;
        }
        applied
    }

    fn judge(&mut self, log: &LogName, frame: &ChangeFrame) -> Result<Applied, StreamRefusal> {
        if log != &self.log {
            return Err(refusal(
                STREAM_FOREIGN_LOG,
                "the frame names another grant log or reset epoch",
            ));
        }
        match frame {
            ChangeFrame::Baseline { revision, revoked } => Ok(self.baseline(*revision, revoked)),
            ChangeFrame::Change {
                revision,
                grant,
                change,
                event,
            } => self.change(*revision, grant, *change, event),
            ChangeFrame::Watermark { revision } => self.watermark(*revision),
            ChangeFrame::Ready { revision } => self.readied(*revision),
            ChangeFrame::Unready {
                refusal: name,
                reason,
            } => Err(refusal(
                STREAM_NOT_READY,
                format!("readiness was withheld: {name}: {reason}"),
            )),
            ChangeFrame::Reset { reason, .. } => {
                if *reason == ResetReason::Rollback {
                    // The log no longer holds what the cursor stood on:
                    // nothing applied after the baseline is trusted again.
                    self.revoked.clear();
                }
                self.cursor = None;
                self.evidence.clear();
                Err(refusal(
                    STREAM_RESET,
                    "the cursor cannot be continued; read a baseline",
                ))
            }
        }
    }

    fn baseline(&mut self, revision: u64, revoked: &[String]) -> Applied {
        // A revocation is permanent: a baseline adds to what is held and
        // never readmits a grant already seen revoked.
        for grant in revoked {
            self.revoked.entry(grant.clone()).or_insert(revision);
        }
        self.cursor = Some(revision);
        self.evidence.clear();
        self.ready = false;
        Applied::Baseline { revision }
    }

    fn change(
        &mut self,
        revision: u64,
        grant: &str,
        change: ChangeKind,
        event: &str,
    ) -> Result<Applied, StreamRefusal> {
        let cursor = self
            .cursor
            .ok_or_else(|| refusal(STREAM_GAP, "no baseline has been applied"))?;
        let bytes = event_bytes(event)
            .ok_or_else(|| refusal(STREAM_INTEGRITY, "the event is not lowercase hex"))?;
        let evidence = raw_leaf_hash(&bytes);
        if revision <= cursor {
            return match self.evidence.get(&revision) {
                Some(held) if *held == evidence => Ok(Applied::Duplicate { revision }),
                Some(_) => Err(refusal(
                    STREAM_INTEGRITY,
                    format!("revision {revision} carries evidence other than what was applied"),
                )),
                None => Err(refusal(
                    STREAM_INTEGRITY,
                    format!("revision {revision} was already passed without this change"),
                )),
            };
        }
        let next = cursor.saturating_add(1);
        if revision > next {
            return Err(StreamRefusal {
                name: STREAM_GAP,
                reason: format!("revisions {next} to {} were omitted", revision - 1),
                gap: Some(revision - next),
            });
        }
        let signed = verify_grant_event(&bytes, &self.key).map_err(|error| {
            refusal(
                STREAM_INTEGRITY,
                format!("the event is not signed by the service: {error}"),
            )
        })?;
        let event = signed.event();
        let named = match event.change() {
            GrantChange::Issue(_) => ChangeKind::Issue,
            GrantChange::Revoke { .. } => ChangeKind::Revoke,
            GrantChange::Use { .. } => {
                return Err(refusal(
                    STREAM_INTEGRITY,
                    "a use is not a grant change the stream carries",
                ));
            }
        };
        if named != change || event.grant().to_string() != grant {
            return Err(refusal(
                STREAM_INTEGRITY,
                "the frame names another grant or change than its signed event",
            ));
        }
        if change == ChangeKind::Revoke {
            self.revoked.entry(grant.to_owned()).or_insert(revision);
        }
        self.cursor = Some(revision);
        self.evidence.insert(revision, evidence);
        while self.evidence.len() > self.window.get() {
            self.evidence.pop_first();
        }
        Ok(Applied::Change { revision, change })
    }

    fn watermark(&mut self, revision: u64) -> Result<Applied, StreamRefusal> {
        let cursor = self
            .cursor
            .ok_or_else(|| refusal(STREAM_GAP, "no baseline has been applied"))?;
        let through = cursor.max(revision);
        self.cursor = Some(through);
        Ok(Applied::Watermark { revision: through })
    }

    fn readied(&mut self, revision: u64) -> Result<Applied, StreamRefusal> {
        if self.cursor != Some(revision) {
            return Err(refusal(
                STREAM_READY_DISAGREES,
                format!(
                    "Ready names revision {revision}, and the index stands at {:?}",
                    self.cursor
                ),
            ));
        }
        self.ready = true;
        Ok(Applied::Ready { revision })
    }

    /// Whether a right whose binding requires `required` and rests on the
    /// grants of `path` may be admitted now.
    ///
    /// # Errors
    /// [`STREAM_NOT_READY`], [`STREAM_BEHIND`] or [`DEPENDENCY_REVOKED`], by name.
    pub fn admit(&self, required: u64, path: &[String]) -> Result<(), StreamRefusal> {
        if !self.ready {
            return Err(refusal(STREAM_NOT_READY, "the grant stream is not ready"));
        }
        let cursor = self.cursor.unwrap_or_default();
        if cursor < required {
            return Err(refusal(
                STREAM_BEHIND,
                format!("the binding requires revision {required}; the index stands at {cursor}"),
            ));
        }
        let mut seen = BTreeSet::new();
        for grant in path {
            if !seen.insert(grant.as_str()) {
                return Err(refusal(
                    STREAM_INTEGRITY,
                    format!("the dependency path names {grant} twice"),
                ));
            }
            if let Some(at) = self.revoked.get(grant) {
                return Err(refusal(
                    DEPENDENCY_REVOKED,
                    format!("{grant} was revoked at revision {at}"),
                ));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "channel_membership_index_tests.rs"]
mod tests;
