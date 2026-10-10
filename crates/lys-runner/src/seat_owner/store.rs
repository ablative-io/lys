//! The durable record of supervised-seat ownership (AGENTS-004 R4).
//!
//! One file is the indexed current projection, `seat-owners.json`, replaced
//! whole and atomically at each checkpoint; one file is the bounded recovery
//! tail, `seat-owners.journal`, one JSON intent per line appended and synced
//! once per owner transition. Reopening reads the projection, then only the
//! journal lines after its checkpoint: never the whole history, never a
//! session's permanent record. When the tail reaches [`MAX_TAIL`] intents the
//! next intent is preceded by a checkpoint, so recovery visits at most the
//! live owners plus [`MAX_TAIL`] lines.
//!
//! Every intent carries the id its author chose, 32 lowercase hexadecimal
//! characters; an append whose reply was lost is resolved by sending the same
//! id again, which the store answers [`Applied::Already`] without a second
//! line, so an uncertain transition never becomes two. A different id is a
//! different intent.
//!
//! The installed `lys-runner-sessions/v3` record ([`crate::state`]) is not
//! changed by this module: a state directory with that record and no owner
//! file is an old install, opened as holding no supervised owner, and its
//! manual sessions keep their meaning. A record of an unknown version, a
//! malformed line or an oversized member is refused by name; nothing is
//! skipped and nothing is read as empty.
//!
//! Records hold public identity and credential references only: a credential
//! reference is a name under which the runner's secret store answers, never
//! the bytes.

use std::collections::BTreeMap;
use std::fs;
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

use crate::error::RunnerError;

use super::record::Line;
pub use super::record::{
    Applied, Cursors, Custody, FORMAT, Holder, Intent, Lease, MAX_CREDENTIAL_REFERENCES,
    MAX_MEMBER_BYTES, MAX_OWNERS, MAX_REFERENCE_BYTES, MAX_TAIL, Migrated, OwnerRecord, Projection,
    StoreCounts,
};
use super::rules::{apply, is_hex_id, refused, validate_record};

/// The indexed current projection's file, in the store's directory.
pub const PROJECTION: &str = "seat-owners.json";
/// The bounded recovery tail's file, in the store's directory.
pub const JOURNAL: &str = "seat-owners.journal";

/// The store.
#[derive(Debug)]
pub struct OwnerStore {
    projection_path: PathBuf,
    journal_path: PathBuf,
    projection: Projection,
    /// Intent ids in the tail with their sequence, for duplicate resolution.
    tail_ids: BTreeMap<String, u64>,
    /// The last sequence in the journal.
    last_seq: u64,
    counts: StoreCounts,
}

impl OwnerStore {
    /// Upgrades an old install in `dir` to this record. The installed
    /// sessions record is the one the caller already read through
    /// [`crate::state::StateFile`], so no second reader takes its lock; it is
    /// counted, never changed, and none of its sessions is promoted to a
    /// supervised owner. The operations store is not opened. When no owner
    /// projection exists an empty one with its explicit version is written
    /// once, atomically; when one exists nothing is written. The store is then
    /// opened the ordinary way, so what the upgrade reports is what a reopen
    /// finds.
    ///
    /// # Errors
    ///
    /// `seat_owner_format_unknown` when the installed record is not
    /// [`crate::state::FORMAT`], which is never read as empty;
    /// `seat_owner_record_invalid` for an installed session without a name;
    /// `seat_owner_checkpoint_refused` when the projection cannot be written
    /// durably, after which no owner file exists and the installed state
    /// stands; and [`Self::open`]'s refusals.
    pub fn migrate_installed(
        dir: &Path,
        installed: &crate::state::Kept,
    ) -> Result<Migrated, RunnerError> {
        if installed.format != crate::state::FORMAT {
            return Err(refused(
                "seat_owner_format_unknown",
                format!(
                    "the installed record is in format {}, not {}",
                    installed.format,
                    crate::state::FORMAT
                ),
            ));
        }
        let mut live_manual = 0_usize;
        let mut ended = 0_usize;
        for session in &installed.sessions {
            if session.session.is_empty() {
                return Err(refused(
                    "seat_owner_record_invalid",
                    "an installed session has no name",
                ));
            }
            if session.ended.is_some() {
                ended = ended.saturating_add(1);
            } else {
                live_manual = live_manual.saturating_add(1);
            }
        }
        fs::create_dir_all(dir).map_err(|error| {
            refused(
                "seat_owner_store_unavailable",
                format!("making {}: {error}", dir.display()),
            )
        })?;
        let projection_path = dir.join(PROJECTION);
        let already_versioned = match fs::metadata(&projection_path) {
            Ok(_) => true,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
            Err(error) => {
                return Err(refused(
                    "seat_owner_store_unavailable",
                    format!("reading {}: {error}", projection_path.display()),
                ));
            }
        };
        if !already_versioned {
            let bytes = serde_json::to_vec_pretty(&Projection::empty())
                .map_err(|error| refused("seat_owner_record_invalid", error))?;
            crate::state::replace(&projection_path, &bytes).map_err(|error| {
                refused(
                    "seat_owner_checkpoint_refused",
                    format!("writing {}: {error}", projection_path.display()),
                )
            })?;
        }
        let store = Self::open(dir)?;
        Ok(Migrated {
            installed_format: installed.format.clone(),
            live_manual,
            ended,
            owners: store.projection.owners.len(),
            already_versioned,
        })
    }

    /// The store in `dir`, the runner's state directory. A directory holding
    /// only the installed sessions record is an old install and opens with
    /// no owner; an owner projection of another format is refused by name.
    ///
    /// # Errors
    ///
    /// `seat_owner_store_unavailable` when the directory or a file cannot be
    /// read; `seat_owner_format_unknown` for a projection of another format;
    /// `seat_owner_record_invalid` for a malformed record, line, sequence or
    /// bound, which is never skipped.
    pub fn open(dir: &Path) -> Result<Self, RunnerError> {
        fs::create_dir_all(dir).map_err(|error| {
            refused(
                "seat_owner_store_unavailable",
                format!("making {}: {error}", dir.display()),
            )
        })?;
        let projection_path = dir.join(PROJECTION);
        let journal_path = dir.join(JOURNAL);
        let mut counts = StoreCounts::default();
        let projection = match fs::read(&projection_path) {
            Ok(bytes) => {
                counts.bytes_copied = counts.bytes_copied.saturating_add(bytes.len() as u64);
                if bytes.len() > MAX_MEMBER_BYTES.saturating_mul(MAX_OWNERS) {
                    return Err(refused(
                        "seat_owner_record_invalid",
                        "the owner projection exceeds its bound",
                    ));
                }
                let value: serde_json::Value = serde_json::from_slice(&bytes).map_err(|error| {
                    refused(
                        "seat_owner_record_invalid",
                        format!("{} does not read: {error}", projection_path.display()),
                    )
                })?;
                let found = value
                    .get("format")
                    .and_then(serde_json::Value::as_str)
                    .ok_or_else(|| {
                        refused(
                            "seat_owner_record_invalid",
                            "the owner projection names no format",
                        )
                    })?;
                if found != FORMAT {
                    return Err(refused(
                        "seat_owner_format_unknown",
                        format!(
                            "{} is in format {found}, not {FORMAT}",
                            projection_path.display()
                        ),
                    ));
                }
                let projection: Projection = serde_json::from_value(value).map_err(|error| {
                    refused(
                        "seat_owner_record_invalid",
                        format!("{} does not read: {error}", projection_path.display()),
                    )
                })?;
                for (session, record) in &projection.owners {
                    counts.record_visits = counts.record_visits.saturating_add(1);
                    if session != &record.session {
                        return Err(refused(
                            "seat_owner_record_invalid",
                            format!("owner {session} is indexed under another session"),
                        ));
                    }
                    validate_record(record)?;
                }
                projection
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Projection::empty(),
            Err(error) => {
                return Err(refused(
                    "seat_owner_store_unavailable",
                    format!("reading {}: {error}", projection_path.display()),
                ));
            }
        };
        let mut store = Self {
            projection_path,
            journal_path,
            projection,
            tail_ids: BTreeMap::new(),
            last_seq: 0,
            counts,
        };
        store.replay_tail()?;
        Ok(store)
    }

    /// Apply the journal lines after the projection's checkpoint.
    fn replay_tail(&mut self) -> Result<(), RunnerError> {
        let file = match fs::File::open(&self.journal_path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                self.last_seq = self.projection.checkpoint;
                return Ok(());
            }
            Err(error) => {
                return Err(refused(
                    "seat_owner_store_unavailable",
                    format!("reading {}: {error}", self.journal_path.display()),
                ));
            }
        };
        let mut last_seq = 0u64;
        let mut visited = 0u64;
        for (index, line) in std::io::BufReader::new(file).split(b'\n').enumerate() {
            let line = line.map_err(|error| {
                refused(
                    "seat_owner_store_unavailable",
                    format!("reading {}: {error}", self.journal_path.display()),
                )
            })?;
            if line.is_empty() {
                continue;
            }
            if line.len() > MAX_MEMBER_BYTES {
                return Err(refused(
                    "seat_owner_record_invalid",
                    format!(
                        "journal line {} exceeds {MAX_MEMBER_BYTES} bytes",
                        index + 1
                    ),
                ));
            }
            self.counts.bytes_copied = self.counts.bytes_copied.saturating_add(line.len() as u64);
            let parsed: Line = serde_json::from_slice(&line).map_err(|error| {
                refused(
                    "seat_owner_record_invalid",
                    format!("journal line {} does not read: {error}", index + 1),
                )
            })?;
            if parsed.seq != last_seq.saturating_add(1) {
                return Err(refused(
                    "seat_owner_record_invalid",
                    format!(
                        "journal line {} has sequence {}, expected {}",
                        index + 1,
                        parsed.seq,
                        last_seq.saturating_add(1)
                    ),
                ));
            }
            last_seq = parsed.seq;
            if parsed.seq <= self.projection.checkpoint {
                // Already inside the projection; its id is still a known intent.
                continue;
            }
            visited = visited.saturating_add(1);
            if visited > MAX_TAIL.saturating_add(1) {
                return Err(refused(
                    "seat_owner_record_invalid",
                    format!("the journal tail exceeds {MAX_TAIL} lines after its checkpoint"),
                ));
            }
            if !is_hex_id(&parsed.intent) {
                return Err(refused(
                    "seat_owner_record_invalid",
                    format!("journal line {} has an invalid intent id", index + 1),
                ));
            }
            self.counts.record_visits = self.counts.record_visits.saturating_add(1);
            apply(&mut self.projection, &parsed.body)?;
            self.tail_ids.insert(parsed.intent, parsed.seq);
        }
        if last_seq < self.projection.checkpoint {
            return Err(refused(
                "seat_owner_record_invalid",
                format!(
                    "the projection checkpoint {} is past the journal's last sequence {last_seq}",
                    self.projection.checkpoint
                ),
            ));
        }
        self.last_seq = last_seq;
        Ok(())
    }

    /// The live owners, by session.
    pub fn owners(&self) -> &BTreeMap<String, OwnerRecord> {
        &self.projection.owners
    }

    /// The owner of `session`, when one is live.
    pub fn owner(&self, session: &str) -> Option<&OwnerRecord> {
        self.projection.owners.get(session)
    }

    /// The work done so far.
    pub fn counts(&self) -> StoreCounts {
        self.counts
    }

    /// The last journal sequence.
    pub fn last_seq(&self) -> u64 {
        self.last_seq
    }

    /// Take `intent` under `id`, durably: one line, one sync. The same id a
    /// second time answers [`Applied::Already`] and writes nothing.
    ///
    /// # Errors
    ///
    /// `seat_owner_intent_invalid` for an id that is not 32 hexadecimal
    /// characters; the refusal the held record names for an intent it forbids
    /// (`seat_owner_unknown`, `seat_owner_held`, `seat_owner_generation_stale`,
    /// `seat_owner_harness_mismatch`, `seat_owner_stop_fenced`,
    /// `seat_owner_exited`, `seat_owner_cursor_regression`, `seat_owner_live`);
    /// `seat_owner_bound_exceeded` for an oversized intent or a full roster;
    /// `seat_owner_append_refused` when the journal cannot be appended and
    /// synced, in which case nothing changed.
    pub fn record(&mut self, id: &str, intent: &Intent) -> Result<Applied, RunnerError> {
        if !is_hex_id(id) {
            return Err(refused(
                "seat_owner_intent_invalid",
                "an intent id must be 32 lowercase hexadecimal characters",
            ));
        }
        if let Some(seq) = self.tail_ids.get(id) {
            return Ok(Applied::Already { seq: *seq });
        }
        // Validate against the current projection before anything is written.
        let mut next = self.projection.clone_for_check(intent);
        apply(&mut next, intent)?;
        if matches!(intent, Intent::Establish { .. }) && self.projection.owners.len() >= MAX_OWNERS
        {
            return Err(refused(
                "seat_owner_bound_exceeded",
                format!("{MAX_OWNERS} live owners are the bound"),
            ));
        }
        if self.last_seq.saturating_sub(self.projection.checkpoint) >= MAX_TAIL {
            self.checkpoint()?;
        }
        let seq = self.last_seq.saturating_add(1);
        let line = Line {
            seq,
            intent: id.to_owned(),
            body: intent.clone(),
        };
        let mut bytes = serde_json::to_vec(&line)
            .map_err(|error| refused("seat_owner_intent_invalid", error))?;
        if bytes.len() > MAX_MEMBER_BYTES {
            return Err(refused(
                "seat_owner_bound_exceeded",
                format!(
                    "the intent is {} bytes, over the {MAX_MEMBER_BYTES} byte bound",
                    bytes.len()
                ),
            ));
        }
        bytes.push(b'\n');
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.journal_path)
            .map_err(|error| {
                refused(
                    "seat_owner_store_unavailable",
                    format!("opening {}: {error}", self.journal_path.display()),
                )
            })?;
        file.write_all(&bytes)
            .and_then(|()| file.sync_data())
            .map_err(|error| {
                refused(
                    "seat_owner_append_refused",
                    format!("appending {}: {error}", self.journal_path.display()),
                )
            })?;
        self.counts.bytes_copied = self.counts.bytes_copied.saturating_add(bytes.len() as u64);
        self.counts.journal_appends = self.counts.journal_appends.saturating_add(1);
        self.counts.syncs = self.counts.syncs.saturating_add(1);
        apply(&mut self.projection, intent)?;
        self.last_seq = seq;
        self.tail_ids.insert(id.to_owned(), seq);
        Ok(Applied::Recorded { seq })
    }

    /// Write the projection at the last sequence, durably. The previous
    /// projection stays readable until the replacement is renamed over it; an
    /// interruption before that leaves the old checkpoint and the full tail.
    ///
    /// # Errors
    ///
    /// `seat_owner_checkpoint_refused` when the replacement cannot be written
    /// and synced; the previous projection is then still the one read.
    pub fn checkpoint(&mut self) -> Result<(), RunnerError> {
        let mut projection = self.projection.clone();
        projection.checkpoint = self.last_seq;
        let bytes = serde_json::to_vec_pretty(&projection)
            .map_err(|error| refused("seat_owner_record_invalid", error))?;
        crate::state::replace(&self.projection_path, &bytes).map_err(|error| {
            refused(
                "seat_owner_checkpoint_refused",
                format!("writing {}: {error}", self.projection_path.display()),
            )
        })?;
        self.counts.bytes_copied = self.counts.bytes_copied.saturating_add(bytes.len() as u64);
        self.counts.syncs = self.counts.syncs.saturating_add(2);
        self.counts.checkpoints = self.counts.checkpoints.saturating_add(1);
        self.projection.checkpoint = self.last_seq;
        self.tail_ids.clear();
        Ok(())
    }
}
