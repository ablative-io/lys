//! Native observations use the existing committed feed and its cursor.
//! Their source identities share its atomic commit and crash recovery, so
//! replay never appends a second observation or creates a second usage owner.
use std::io::{BufRead, BufReader, Seek, SeekFrom};

use sha2::{Digest, Sha256};

use super::{Body, Commit, Feed, FeedEntry, failed};
use crate::error::RunnerError;
use crate::harness_control::events::{Event, Projection, Source};

fn identity(event: &Event) -> Result<String, RunnerError> {
    let bytes = serde_json::to_vec(&("lys-managed-event/v1", &event.source, &event.source_id))
        .map_err(failed)?;
    Ok(crate::protocol::hex(&Sha256::digest(bytes)))
}

impl Feed {
    /// Commit one observation from the launch owner's proved source. A replay
    /// returns the original sequence without appending; reusing a native event
    /// identity for different facts refuses. Callers still validate turn order
    /// through the dispatcher before using this durable-keep callback.
    ///
    /// # Errors
    /// `control_source_mismatch`, `control_event_reused`, or feed storage errors.
    pub fn append_control(
        &mut self,
        expected: &Source,
        at: u64,
        event: &Event,
    ) -> Result<u64, RunnerError> {
        Projection::new(expected.clone())?;
        if &event.source != expected || event.source_id.is_empty() {
            return Err(RunnerError::refused(
                "control_source_mismatch",
                "native observation differs from the proved session generation",
            ));
        }
        let key = identity(event)?;
        if let Some(offset) = self.index.controls.get(&key).copied() {
            let kept = self.control_at(offset)?;
            if kept.session != expected.binding.session || kept.body != Body::Control(event.clone())
            {
                return Err(RunnerError::refused(
                    "control_event_reused",
                    "native event identity already records different lifecycle facts",
                ));
            }
            return Ok(kept.seq);
        }
        self.append(
            &expected.binding.session,
            at,
            vec![Body::Control(event.clone())],
            Commit {
                control: Some(key),
                ..Commit::default()
            },
        )
    }

    // Read only the indexed observation, not all later events in the log.
    fn control_at(&self, offset: u64) -> Result<FeedEntry, RunnerError> {
        let mut file = std::fs::File::open(&self.path).map_err(failed)?;
        file.seek(SeekFrom::Start(offset)).map_err(failed)?;
        let mut reader = BufReader::new(file);
        let mut line = String::new();
        reader.read_line(&mut line).map_err(failed)?;
        if !line.ends_with('\n') || offset.saturating_add(line.len() as u64) > self.index.committed
        {
            return Err(failed(
                "the indexed managed observation is outside committed coverage",
            ));
        }
        serde_json::from_str(&line).map_err(failed)
    }
}
