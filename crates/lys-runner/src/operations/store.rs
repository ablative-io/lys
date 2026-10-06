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
    original_request: Option<[u8; 32]>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Bundle {
    batch: Vec<Record>,
}
#[derive(Deserialize)]
#[serde(untagged)]
enum Row {
    One(Box<Record>),
    Batch(Bundle),
}

#[derive(Serialize)]
struct Writing<'a> {
    outcome: &'a OperationOutcome,
    control: Option<&'a Control>,
    original_request: Option<&'a [u8; 32]>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Snapshot {
    format: String,
    offset: u64,
    seen: HashSet<String>,
    held: HashMap<String, OperationOutcome>,
    controls: HashMap<String, Control>,
    original_requests: HashMap<String, [u8; 32]>,
}

#[derive(Serialize)]
struct Sealing<'a> {
    format: &'static str,
    offset: u64,
    seen: &'a HashSet<String>,
    held: &'a HashMap<String, OperationOutcome>,
    controls: &'a HashMap<String, Control>,
    original_requests: &'a HashMap<String, [u8; 32]>,
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
            batch: None,
            checkpoint_offset: 0,
            checkpoint_bytes: 0,
            controls: HashMap::new(),
            original_requests: HashMap::new(),
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
        if snapshot
            .original_requests
            .keys()
            .any(|id| !snapshot.held.contains_key(id))
        {
            return Err(unavailable(
                "checkpoint contains an orphaned request identity",
            ));
        }
        self.original_requests = snapshot.original_requests;
        self.controls = snapshot.controls;
        for (id, outcome) in snapshot.held {
            if id != outcome.operation || !self.seen.contains(&id) {
                return Err(unavailable("checkpoint outcome identity is inconsistent"));
            }
            if super::control::managed_request(&outcome.request)
                && !self.original_requests.contains_key(&id)
            {
                return Err(unavailable("managed request has no original identity"));
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
            let records = match serde_json::from_slice::<Row>(&line).map_err(unavailable)? {
                Row::One(record) => vec![*record],
                Row::Batch(bundle) => bundle.batch,
            };
            if records.is_empty() {
                return Err(unavailable("a journal decision batch is empty"));
            }
            let mut prepared = BTreeSet::new();
            let mut identities = BTreeMap::new();
            for record in &records {
                let original = identities
                    .get(&record.outcome.operation)
                    .or_else(|| self.original_requests.get(&record.outcome.operation));
                if super::control::managed_request(&record.outcome.request)
                    && record.original_request.is_none()
                    || original
                        .is_some_and(|original| Some(original) != record.original_request.as_ref())
                {
                    return Err(unavailable(
                        "journal record drops or changes the original request identity",
                    ));
                }
                if let Some(identity) = record.original_request {
                    identities.insert(record.outcome.operation.clone(), identity);
                }
                if let Some(control) = &record.control {
                    control.validate(&record.outcome)?;
                    prepared.insert(record.outcome.operation.as_str());
                } else if self.controls.contains_key(&record.outcome.operation)
                    || prepared.contains(record.outcome.operation.as_str())
                {
                    return Err(unavailable(
                        "a journal record drops its original preparation",
                    ));
                }
            }
            let offset = self
                .journal_offset
                .checked_add(u64::try_from(read).map_err(unavailable)?)
                .ok_or_else(|| unavailable("journal offset overflows"))?;
            for record in records {
                if let Some(identity) = record.original_request {
                    self.original_requests
                        .insert(record.outcome.operation.clone(), identity);
                }
                if let Some(control) = record.control {
                    self.controls
                        .insert(record.outcome.operation.clone(), control);
                }
                self.fold(record.outcome, now);
            }
            self.journal_offset = offset;
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
        if super::control::managed_request(&outcome.request) {
            return Err(RunnerError::refused(
                "operation_legacy_request_invalid",
                "the installed legacy record cannot contain a managed request",
            ));
        }
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
                original_request: None,
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
        self.record_change(outcome, now, None, None)
    }

    pub(super) fn record_control(
        &mut self,
        outcome: OperationOutcome,
        control: Control,
    ) -> Result<(), RunnerError> {
        control.validate(&outcome)?;
        self.record_change(outcome, now_ms(), Some(control), None)
    }

    pub(super) fn record_request(
        &mut self,
        outcome: OperationOutcome,
        identity: [u8; 32],
    ) -> Result<(), RunnerError> {
        self.record_change(outcome, now_ms(), None, Some(identity))
    }

    fn record_change(
        &mut self,
        outcome: OperationOutcome,
        now: u64,
        control: Option<Control>,
        original_request: Option<[u8; 32]>,
    ) -> Result<(), RunnerError> {
        let mut bytes = serde_json::to_vec(&Writing {
            outcome: &outcome,
            control: control
                .as_ref()
                .or_else(|| self.controls.get(&outcome.operation)),
            original_request: original_request
                .as_ref()
                .or_else(|| self.original_requests.get(&outcome.operation)),
        })
        .map_err(unavailable)?;
        let length = if let Some(batch) = &self.batch {
            batch.increment(bytes.len())?
        } else {
            bytes.push(b'\n');
            u64::try_from(bytes.len()).map_err(unavailable)?
        };
        let offset = self
            .journal_offset
            .checked_add(length)
            .ok_or_else(|| unavailable("journal offset overflows"))?;
        if let Some(batch) = &mut self.batch {
            batch.append(self.writer.as_ref(), &self.path, &bytes)?;
        } else if let Some(writer) = &self.writer {
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
        self.journal_offset = offset;
        if let Some(identity) = original_request {
            self.original_requests
                .insert(outcome.operation.clone(), identity);
        }
        if let Some(control) = control {
            self.controls.insert(outcome.operation.clone(), control);
        }
        self.fold(outcome, now);
        if self.batch.is_none() {
            self.checkpoint_if_due()?;
        }
        Ok(())
    }

    pub(super) fn checkpoint_if_due(&mut self) -> Result<(), RunnerError> {
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
            original_requests: &self.original_requests,
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

#[cfg(test)]
impl Operations {
    pub(crate) fn test_journal_path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn test_journal_offset(&mut self, offset: u64) -> u64 {
        std::mem::replace(&mut self.journal_offset, offset)
    }
}

#[cfg(test)]
mod batch_validation_tests {
    use super::{Control, Operations, Writing};
    use crate::operations::{Certainty, OperationOutcome, OperationState};
    #[test]
    fn every_batch_member_is_validated_before_any_member_folds()
    -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let mut operations = Operations::open_at(dir.path(), 1)?;
        let outcome = |id: &str| OperationOutcome {
            operation: id.to_owned(),
            session: "session".to_owned(),
            request: "notice".to_owned(),
            state: OperationState::Delivered,
            at: 1,
            words: "observed".to_owned(),
            text: None,
            ended: None,
        };
        let first = outcome("first");
        let second = outcome("second");
        let invalid = Control {
            prepared: None,
            original_text: None,
            certainty: Certainty::Observed,
            admitted: None,
            decision: None,
        };
        let mut bytes = serde_json::to_vec(&serde_json::json!({"batch":[
            serde_json::to_value(Writing {outcome:&first,control:None,original_request:None})?,
            serde_json::to_value(Writing {outcome:&second,control:Some(&invalid),original_request:None})?,
        ]}))?;
        bytes.push(b'\n');
        std::fs::write(&operations.path, bytes)?;
        let error = operations
            .read_tail(1)
            .err()
            .ok_or("invalid batch member was accepted")?;
        assert!(error.to_string().contains("control_record_invalid"));
        assert!(operations.get("first").is_none());
        assert!(operations.get("second").is_none());
        assert_eq!(operations.journal_offset, 0);
        Ok(())
    }
}
