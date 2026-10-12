//! Forward migration of installed operation records.
use super::Certainty;
use super::store::Writing;
use super::*;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::Path;

impl Operations {
    pub(super) fn migrate(&mut self, dir: &Path) -> Result<(), RunnerError> {
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

    pub(super) fn archive_legacy(dir: &Path) -> Result<(), RunnerError> {
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
}
