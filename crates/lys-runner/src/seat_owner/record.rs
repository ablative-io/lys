//! The record types of the seat-owner store (AGENTS-004 R4): what one
//! owner is, what one journal intent says, and the bounds every shape keeps.
//! The store ([`super::store`]) reads and writes them; the rules
//! ([`super::rules`]) validate and apply them.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::peer::Leader;

/// The projection's format.
pub const FORMAT: &str = "lys-runner-seat-owners/v1";
/// Intents kept in the tail before the next intent is preceded by a checkpoint.
pub const MAX_TAIL: u64 = 256;
/// The most bytes one journal line or one handover member may hold.
pub const MAX_MEMBER_BYTES: usize = 64 * 1024;
/// The most credential references one owner may hold.
pub const MAX_CREDENTIAL_REFERENCES: usize = 16;
/// The most bytes one credential reference may hold.
pub const MAX_REFERENCE_BYTES: usize = 256;
/// The most owners one runner state directory may hold live.
pub const MAX_OWNERS: usize = 4096;

/// Who holds a session's single-writer lease.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum Holder {
    /// The independent owner process itself, by its process-start identity.
    Owner { start: Leader },
    /// A successor owner prepared for an upgrade, not yet acknowledged.
    Successor { start: Leader },
}

/// The lease: one writer per session, fenced by a monotonic generation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Lease {
    /// The generation this lease was taken at; a lower one is stale.
    pub generation: u64,
    /// The holder.
    pub holder: Holder,
    /// When it was taken, in milliseconds since the Unix epoch.
    pub taken_at: u64,
}

/// Where the session's custody stands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case", tag = "stage")]
pub enum Custody {
    /// The lease holder serves the seat.
    Owned,
    /// A successor was prepared under `intent` and has not acknowledged.
    HandingOver {
        intent: String,
        successor: Leader,
        since: u64,
    },
    /// A deliberate stop was fenced under `intent`; the exit proof follows it.
    Stopping { intent: String, since: u64 },
    /// The harness exited; the proof is the kernel's.
    Exited {
        intent: String,
        exit: crate::protocol::Ended,
    },
}

/// The cursors a replacement client resumes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cursors {
    /// The last control receipt delivered, by its operation sequence.
    pub receipt: u64,
    /// The last feed row delivered.
    pub feed: u64,
    /// The last accepted hook, by its owner sequence.
    pub hook: u64,
}

/// One supervised seat's owner record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnerRecord {
    /// The AGENTS-002 seat this session serves.
    pub seat: String,
    /// The session.
    pub session: String,
    /// The conversation the harness holds.
    pub conversation: String,
    /// The harness process-start identity, once spawned.
    pub harness: Option<Leader>,
    /// The lease.
    pub lease: Lease,
    /// The custody stage.
    pub custody: Custody,
    /// The cursors.
    pub cursors: Cursors,
    /// Names under which credentials are answered; never their bytes.
    pub credential_references: Vec<String>,
    /// When the owner was established.
    pub established_at: u64,
}

/// The indexed current projection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Projection {
    /// [`FORMAT`].
    pub format: String,
    /// The journal sequence this projection includes, inclusive.
    pub checkpoint: u64,
    /// Live owners by session.
    pub owners: BTreeMap<String, OwnerRecord>,
}

impl Projection {
    pub(super) fn empty() -> Self {
        Self {
            format: FORMAT.to_owned(),
            checkpoint: 0,
            owners: BTreeMap::new(),
        }
    }
}

/// What one journal line does.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case", tag = "kind")]
pub enum Intent {
    /// Establish an owner for a session no record holds.
    Establish { record: OwnerRecord },
    /// Move the lease to `holder` at `generation`, which must exceed the held one.
    Lease {
        session: String,
        generation: u64,
        holder: Holder,
        taken_at: u64,
    },
    /// Set the harness process-start identity.
    Harness { session: String, start: Leader },
    /// Change custody.
    Custody { session: String, custody: Custody },
    /// Advance the cursors; a cursor never moves back.
    Cursors { session: String, cursors: Cursors },
    /// Retire the session's owner record from the live projection.
    Retire { session: String },
}

/// One journal line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct Line {
    /// Monotonic from 1.
    pub(super) seq: u64,
    /// The author's id for this intent, 32 lowercase hexadecimal characters.
    pub(super) intent: String,
    #[serde(flatten)]
    pub(super) body: Intent,
}

/// How an intent was taken.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Applied {
    /// Appended and durable at this sequence.
    Recorded { seq: u64 },
    /// An intent of this id was already durable at this sequence; nothing was written.
    Already { seq: u64 },
}

/// The work a store did, for the counted ratchets (AGENTS-004 R5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StoreCounts {
    /// Owner records and journal lines visited.
    pub record_visits: u64,
    /// Bytes read or written.
    pub bytes_copied: u64,
    /// Journal lines appended.
    pub journal_appends: u64,
    /// Physical syncs issued.
    pub syncs: u64,
    /// Projection checkpoints written.
    pub checkpoints: u64,
}

/// What an old install's upgrade to this record did (AGENTS-004 R4).
///
/// The installed `lys-runner-sessions/v3` record is read through its own
/// reader and left byte for byte as it was; its sessions keep their manual
/// meaning and are never promoted to supervised owners. The operations store
/// (DIRECTORY-064) is not opened: its operation ids and uncertainty stay
/// where they are. The one thing written is the versioned owner projection,
/// replaced atomically, so an interrupted upgrade leaves no owner file or the
/// previous one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Migrated {
    /// The format the installed record read as.
    pub installed_format: String,
    /// Installed sessions still running when the upgrade read them.
    pub live_manual: usize,
    /// Installed sessions that had ended.
    pub ended: usize,
    /// Supervised owners after the upgrade: none come from an old install.
    pub owners: usize,
    /// Whether an owner projection already existed, in which case the
    /// upgrade wrote nothing.
    pub already_versioned: bool,
}
