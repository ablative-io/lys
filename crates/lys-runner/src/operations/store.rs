//! A projection checkpoint pins a flushed journal prefix and preserves every spent identity.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::io::{BufRead, BufReader, BufWriter, Seek, SeekFrom, Write};
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::control::Control;
use super::{Certainty, FORMAT, Kept, OperationOutcome, OperationState, Operations, unavailable};
use crate::error::RunnerError;
use crate::session::now_ms;

const STORAGE_FORMAT: &str = "lys-runner-operations/v2";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    outcome: OperationOutcome,
    control: Option<Control>,
}

#[derive(Serialize)]
struct Writing<'a> {
    outcome: &'a OperationOutcome,
    control: Option<&'a Control>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Snapshot {
    format: String,
    offset: u64,
    seen: HashSet<String>,
    held: HashMap<String, OperationOutcome>,
    controls: HashMap<String, Control>,
}

#[derive(Serialize)]
struct Sealing<'a> {
    format: &'static str,
    offset: u64,
    seen: &'a HashSet<String>,
    held: &'a HashMap<String, OperationOutcome>,
    controls: &'a HashMap<String, Control>,
}

impl Operations {
    pub(crate) fn open(dir: &Path) -> Result<Self, RunnerError> {
        Self::open_at(dir, now_ms())
    }

    pub(super) fn open_at(dir: &Path, now: u64) -> Result<Self, RunnerError> {
        let mut operations = Self {
            path: dir.join("operations.v2.journal"),
            checkpoint: dir.join("operations.v2.snapshot"),
            journal_offset: 0,
            checkpoint_offset: 0,
            checkpoint_bytes: 0,
            controls: HashMap::new(),
            by_session: BTreeMap::new(),
            held: HashMap::new(),
            seen: HashSet::new(),
            expires: BTreeSet::new(),
            accepted: BTreeMap::new(),
            active: BTreeMap::new(),
            texts: BTreeMap::new(),
            compacting: BTreeSet::new(),
            writer: None,
        };
        if !operations.path.exists() {
            operations.migrate(dir)?;
        }
        let resumed = operations.resume(now)?;
        operations.read_tail(now)?;
        let interrupted: Vec<_> = operations
            .held
            .values()
            .filter(|outcome| {
                matches!(
                    outcome.state,
                    OperationState::Accepted | OperationState::Delivering
                ) || operations
                    .controls
                    .get(&outcome.operation)
                    .is_some_and(|control| {
                        outcome.state == OperationState::Uncertain && control.matching_admission()
                            || outcome.state == OperationState::Delivered
                                && control.certainty == Certainty::PossiblySent
                    })
            })
            .cloned()
            .collect();
        for mut outcome in interrupted {
            let control = operations.controls.get(&outcome.operation);
            let admitted = control.is_some_and(Control::matching_admission)
                && !matches!(outcome.request.as_str(), "compact" | "context_compact");
            let unsent = control
                .is_some_and(|control| control.certainty == Certainty::SafelyUnsent)
                || (control.is_none() && outcome.state == OperationState::Accepted);
            let (state, reason) = if admitted {
                (
                    OperationState::Confirmed,
                    "harness_admitted: the original preparation has matching saved admission",
                )
            } else if unsent {
                (
                    OperationState::Refused,
                    "session_ended: the runner stopped before a write attempt; the preparation is safely unsent",
                )
            } else {
                (
                    OperationState::Uncertain,
                    "control_readback_unavailable: no matching preparation/admission proves delivery; a write may have happened and the original request is not replayed",
                )
            };
            outcome.state = state;
            outcome.at = now;
            reason.clone_into(&mut outcome.words);
            operations.record_at(outcome, now)?;
        }
        if !resumed {
            operations.checkpoint_now()?;
        }
        Self::archive_legacy(dir)?;
        Ok(operations)
    }

    fn resume(&mut self, now: u64) -> Result<bool, RunnerError> {
        let bytes = match std::fs::read(&self.checkpoint) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
            Err(error) => return Err(unavailable(error)),
        };
        let snapshot: Snapshot = serde_json::from_slice(&bytes).map_err(unavailable)?;
        if snapshot.format != STORAGE_FORMAT {
            return Err(unavailable(format!(
                "checkpoint is in format {}",
                snapshot.format
            )));
        }
        self.checkpoint_bytes = u64::try_from(bytes.len()).map_err(unavailable)?;
        self.checkpoint_offset = snapshot.offset;
        self.journal_offset = snapshot.offset;
        self.seen = snapshot.seen;
        if snapshot
            .controls
            .keys()
            .any(|id| !snapshot.held.contains_key(id))
        {
            return Err(unavailable("checkpoint contains an orphaned preparation"));
        }
        for (id, control) in &snapshot.controls {
            control.validate(
                snapshot
                    .held
                    .get(id)
                    .ok_or_else(|| unavailable("checkpoint preparation has no outcome"))?,
            )?;
        }
        self.controls = snapshot.controls;
        for (id, outcome) in snapshot.held {
            if id != outcome.operation || !self.seen.contains(&id) {
                return Err(unavailable("checkpoint outcome identity is inconsistent"));
            }
            self.fold(outcome, now);
        }
        Ok(true)
    }

    fn read_tail(&mut self, now: u64) -> Result<(), RunnerError> {
        let mut file = std::fs::File::open(&self.path).map_err(unavailable)?;
        let length = file.metadata().map_err(unavailable)?.len();
        if self.journal_offset > length {
            return Err(unavailable("checkpoint is ahead of its journal"));
        }
        file.seek(SeekFrom::Start(self.journal_offset))
            .map_err(unavailable)?;
        let mut reader = BufReader::new(file);
        loop {
            let mut line = Vec::new();
            let read = reader.read_until(b'\n', &mut line).map_err(unavailable)?;
            if read == 0 || line.last() != Some(&b'\n') {
                break;
            }
            let record: Record = serde_json::from_slice(&line).map_err(unavailable)?;
            if let Some(control) = record.control {
                control.validate(&record.outcome)?;
                self.controls
                    .insert(record.outcome.operation.clone(), control);
            } else if self.controls.contains_key(&record.outcome.operation) {
                return Err(unavailable(
                    "a journal record drops its original preparation",
                ));
            }
            self.fold(record.outcome, now);
            self.journal_offset = self
                .journal_offset
                .checked_add(u64::try_from(read).map_err(unavailable)?)
                .ok_or_else(|| unavailable("journal offset overflows"))?;
        }
        if self.journal_offset < length {
            let file = std::fs::OpenOptions::new()
                .write(true)
                .open(&self.path)
                .map_err(unavailable)?;
            file.set_len(self.journal_offset)
                .and_then(|()| file.sync_all())
                .map_err(unavailable)?;
            crate::error::said(
                "operations_tail_incomplete: the uncommitted last record was removed",
            );
        }
        Ok(())
    }

    fn migrate(&mut self, dir: &Path) -> Result<(), RunnerError> {
        let beside = self.path.with_extension("migrating");
        let mut output = BufWriter::new(std::fs::File::create(&beside).map_err(unavailable)?);
        let old_log = dir.join("operations.jsonl");
        if old_log.exists() {
            let mut input = BufReader::new(std::fs::File::open(old_log).map_err(unavailable)?);
            loop {
                let mut line = Vec::new();
                let read = input.read_until(b'\n', &mut line).map_err(unavailable)?;
                if read == 0 || line.last() != Some(&b'\n') {
                    break;
                }
                let outcome = serde_json::from_slice(&line).map_err(unavailable)?;
                Self::migrate_outcome(&mut output, &outcome)?;
            }
        } else {
            match std::fs::read(dir.join("operations.json")) {
                Ok(bytes) => {
                    let kept: Kept = serde_json::from_slice(&bytes).map_err(unavailable)?;
                    if kept.format != FORMAT {
                        return Err(unavailable(format!(
                            "legacy record is in format {}",
                            kept.format
                        )));
                    }
                    for outcome in kept.operations {
                        Self::migrate_outcome(&mut output, &outcome)?;
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(unavailable(error)),
            }
        }
        output
            .flush()
            .and_then(|()| output.get_ref().sync_all())
            .map_err(unavailable)?;
        drop(output);
        std::fs::rename(beside, &self.path).map_err(unavailable)?;
        std::fs::File::open(dir)
            .and_then(|file| file.sync_all())
            .map_err(unavailable)?;
        Ok(())
    }

    fn migrate_outcome(
        output: &mut impl Write,
        outcome: &OperationOutcome,
    ) -> Result<(), RunnerError> {
        let legacy = matches!(
            outcome.state,
            OperationState::Accepted | OperationState::Delivering | OperationState::Uncertain
        )
        .then_some(Control {
            prepared: None,
            original_text: outcome.text.clone(),
            certainty: Certainty::PossiblySent,
            admitted: None,
            decision: None,
        });
        serde_json::to_writer(
            &mut *output,
            &Writing {
                outcome,
                control: legacy.as_ref(),
            },
        )
        .map_err(unavailable)?;
        output.write_all(b"\n").map_err(unavailable)?;
        Ok(())
    }

    fn archive_legacy(dir: &Path) -> Result<(), RunnerError> {
        for (source, kept) in [
            ("operations.jsonl", "operations.v1.jsonl"),
            ("operations.json", "operations.v1.json"),
        ] {
            let source = dir.join(source);
            if source.exists() {
                let kept = dir.join(kept);
                if kept.exists() {
                    return Err(unavailable(
                        "legacy archive already exists beside an unmoved source",
                    ));
                }
                std::fs::rename(source, kept).map_err(unavailable)?;
                std::fs::File::open(dir)
                    .and_then(|file| file.sync_all())
                    .map_err(unavailable)?;
            }
        }
        Ok(())
    }

    pub(super) fn record(&mut self, outcome: OperationOutcome) -> Result<(), RunnerError> {
        self.record_at(outcome, now_ms())
    }

    fn record_at(&mut self, outcome: OperationOutcome, now: u64) -> Result<(), RunnerError> {
        let mut bytes = serde_json::to_vec(&Writing {
            outcome: &outcome,
            control: self.controls.get(&outcome.operation),
        })
        .map_err(unavailable)?;
        bytes.push(b'\n');
        let length = u64::try_from(bytes.len()).map_err(unavailable)?;
        if let Some(writer) = &self.writer {
            writer.append(&self.path, bytes)?;
        } else {
            let mut file = std::fs::OpenOptions::new()
                .append(true)
                .open(&self.path)
                .map_err(unavailable)?;
            file.write_all(&bytes)
                .and_then(|()| file.sync_data())
                .map_err(unavailable)?;
        }
        self.journal_offset = self
            .journal_offset
            .checked_add(length)
            .ok_or_else(|| unavailable("journal offset overflows"))?;
        self.fold(outcome, now);
        if self.checkpoint_bytes > 0
            && self.journal_offset - self.checkpoint_offset >= self.checkpoint_bytes
        {
            self.checkpoint_now()?;
        }
        Ok(())
    }

    fn checkpoint_now(&mut self) -> Result<(), RunnerError> {
        let bytes = serde_json::to_vec(&Sealing {
            format: STORAGE_FORMAT,
            offset: self.journal_offset,
            seen: &self.seen,
            held: &self.held,
            controls: &self.controls,
        })
        .map_err(unavailable)?;
        let length = u64::try_from(bytes.len()).map_err(unavailable)?;
        // The journal sorts before the checkpoint, so a writer batch flushes its prefix first.
        if let Some(writer) = &self.writer {
            writer.replace(&self.checkpoint, bytes)?;
        } else {
            crate::state::replace(&self.checkpoint, &bytes).map_err(unavailable)?;
        }
        self.checkpoint_offset = self.journal_offset;
        self.checkpoint_bytes = length;
        Ok(())
    }
}
