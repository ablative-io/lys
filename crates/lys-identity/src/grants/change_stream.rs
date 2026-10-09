//! The grant change stream's wire contract (DIRECTORY-089 R1).
//!
//! A consumer opens the stream itself, through Lys's published API, with a
//! [`ChangesRequest`]: the grant log it last read, the last receipt revision
//! it applied, and how many log positions one answer may cover. Lys answers
//! a [`ChangesPage`] of [`ChangeFrame`]s, in revision order, strictly after
//! that cursor. A revision is a receipt's `coordinate.index + 1`
//! ([`super::GrantReceipt::revision`]), never an event-format version, a
//! timestamp or a token tombstone.
//!
//! A change the consumer may see is a [`ChangeFrame::Change`] carrying the
//! signed grant event itself, so the consumer verifies it with the service
//! key; changes it may not see are covered by one
//! [`ChangeFrame::Watermark`] naming the last revision they span. A consumer
//! with no cursor is first given a [`ChangeFrame::Baseline`]: the revoked
//! grants it may see at one revision, bounded by the grant book and never a
//! replay of the whole history. [`ChangeFrame::Ready`] states the revision
//! through which every frame was delivered and the permission projection
//! is complete; [`ChangeFrame::Unready`] withdraws readiness by the name of
//! what is not complete, and [`ChangeFrame::Reset`] says the cursor names
//! another log, or a revision this log no longer holds.
//!
//! A read that asks for `placements` also follows the apps' placements,
//! which change what a grant reaches without being grant events. Each
//! placement is kept after one grant revision and before the next, so the
//! two are one order: a [`ChangeFrame::Placement`] stands after the change
//! at its `revision`, and the cursor gains the count of placements applied
//! (`placed`). Placements the consumer may not see are covered by a
//! [`ChangeFrame::PlacementMark`], and one names the head and every
//! placement before each Ready or Unready of such a read. A read that does
//! not ask is answered exactly as before placements were followed.

use serde::{Deserialize, Serialize};

/// A grant log's stable identity and reset epoch. Revisions of different
/// logs, or of different epochs of one log, never compare.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LogName {
    /// The identity recorded once for the log, surviving restart, key
    /// rotation and compaction.
    pub identity: String,
    /// The reset epoch: an explicit replacement of the log moves it.
    pub epoch: u64,
}

/// One read of the stream, opened by its consumer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChangesRequest {
    /// The log the cursor belongs to; absent for a first read.
    #[serde(default)]
    pub log: Option<LogName>,
    /// The last revision applied; absent for a first read, which is answered
    /// with a baseline.
    #[serde(default)]
    pub after: Option<u64>,
    /// How many log positions one answer may cover, at least one: the
    /// consumer's own bound on the frames it buffers.
    pub limit: u64,
    /// Whether to wait, when nothing follows the cursor, for the next
    /// committed change before answering.
    #[serde(default)]
    pub wait: bool,
    /// Whether the read follows placements too.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub placements: bool,
    /// The count of placements applied, the cursor's second half: absent
    /// for a first read, and given with `after` whenever `placements` is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub placed: Option<u64>,
}

/// What a change did to its grant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeKind {
    /// The grant was issued.
    Issue,
    /// The grant was revoked, irreversibly, and with it everything derived
    /// from it; its descendants are resolved from the pass binding.
    Revoke,
}

/// Why a cursor cannot be continued.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResetReason {
    /// The cursor names another log or another epoch of this one.
    OtherLog,
    /// The cursor is past what this log holds: it was rolled back.
    Rollback,
    /// The cursor's placements and revision are not one position of the
    /// order, or a placement after it was kept before placements were
    /// ordered with the grant log.
    Unordered,
}

/// One frame of the stream.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "frame", rename_all = "snake_case", deny_unknown_fields)]
pub enum ChangeFrame {
    /// The revoked grants the consumer may see, at `revision`.
    Baseline {
        /// The revision the baseline stands at.
        revision: u64,
        /// The revoked grant ids, in id order.
        revoked: Vec<String>,
    },
    /// One committed change the consumer may see.
    Change {
        /// The change's receipt revision.
        revision: u64,
        /// The grant changed.
        grant: String,
        /// What the change did.
        change: ChangeKind,
        /// The signed grant event, as the log holds it, in lowercase hex.
        event: String,
    },
    /// Every change through `revision` not otherwise sent is not the
    /// consumer's to see.
    Watermark {
        /// The last revision covered.
        revision: u64,
    },
    /// One placement the consumer may see: `child` placed in `parent`,
    /// kept after the change at `revision` and before the next.
    Placement {
        /// The grant revision it was kept after.
        revision: u64,
        /// Its position among the placements, counting from one.
        placement: u64,
        /// The resource placed, as `kind:id`.
        child: String,
        /// The resource it was placed in, as `kind:id`.
        parent: String,
        /// Whether nothing held on the parent reaches the child.
        restricted: bool,
        /// The grants, in id order, whose reach the placement widened to
        /// the child when it was kept: issued and not withdrawn by then, on
        /// the parent or a resource the parent was placed in. Empty for a
        /// restricted placement or a child already placed.
        widens: Vec<String>,
    },
    /// Every placement through `placement` not otherwise sent is not the
    /// consumer's to see, the last kept after the change at `revision`.
    PlacementMark {
        /// The grant revision the last covered placement was kept after.
        revision: u64,
        /// The count of placements covered.
        placement: u64,
    },
    /// Every frame through `revision` is delivered, and the permission
    /// projection is complete through it.
    Ready {
        /// The revision delivered and projected.
        revision: u64,
    },
    /// The frames were delivered, but readiness is withheld by name.
    Unready {
        /// The stable name of what is not complete.
        refusal: String,
        /// Its words.
        reason: String,
    },
    /// The cursor cannot be continued: read a baseline again.
    Reset {
        /// Why.
        reason: ResetReason,
        /// The log the cursor named.
        from: Option<LogName>,
    },
}

/// One answer of the stream.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChangesPage {
    /// The log every frame belongs to.
    pub log: LogName,
    /// The frames, in revision order.
    pub frames: Vec<ChangeFrame>,
}

/// `bytes` as lowercase hex, as a change frame carries its event.
#[must_use]
pub fn event_hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut text = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        text.push(char::from(DIGITS[usize::from(byte >> 4)]));
        text.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    text
}

/// The bytes a change frame's lowercase hex event carries, or `None` when
/// it is not lowercase hex of whole bytes.
#[must_use]
pub fn event_bytes(text: &str) -> Option<Vec<u8>> {
    fn digit(byte: u8) -> Option<u8> {
        match byte {
            b'0'..=b'9' => Some(byte - b'0'),
            b'a'..=b'f' => Some(byte - b'a' + 10),
            _ => None,
        }
    }
    let bytes = text.as_bytes();
    if bytes.len() % 2 != 0 {
        return None;
    }
    bytes
        .chunks_exact(2)
        .map(|pair| Some((digit(pair[0])? << 4) | digit(pair[1])?))
        .collect()
}
