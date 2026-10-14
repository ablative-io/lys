//! A seat transition is fully folded before its lines enter one durable batch.

use lys_identity::SNAPSHOT_EVERY;
use lys_log_store::LeafStore;

use super::{Held, Line, SeatError, SeatStore, ServerError, unavailable};

pub(super) fn checked(held: &Held, line: &Line) -> Result<(), ServerError> {
    if matches!(line, Line::Added(_)) && held.seat(line.name()).is_some_and(|seat| !seat.retired) {
        return Err(SeatError::NameTaken {
            name: line.name().to_owned(),
        }
        .into());
    }
    if !matches!(line, Line::Added(_)) && held.seat(line.name()).is_none_or(|seat| seat.retired) {
        return Err(SeatError::Unknown {
            name: line.name().to_owned(),
        }
        .into());
    }
    if matches!(line, Line::Moved(_) | Line::Retired(_))
        && let Some(seat) = held.seat(line.name()).filter(|seat| seat.running)
    {
        let session = seat
            .session
            .clone()
            .ok_or_else(|| unavailable("a running seat names no session"))?;
        return Err(SeatError::Running {
            name: line.name().to_owned(),
            session,
        }
        .into());
    }
    Ok(())
}

impl<S: LeafStore> SeatStore<S> {
    /// Keep the ordered transition with one durable append of its new lines.
    /// Repeated operations answer their retained lines without appending them.
    ///
    /// # Errors
    /// Preserves the single-line seat refusals. Refuses an invalid staged fold,
    /// encoding or log count and an uncertain append that cannot be read back
    /// whole with exactly the offered bytes.
    pub(crate) fn keep_all(&mut self, lines: Vec<Line>) -> Result<Vec<Line>, ServerError> {
        self.settle()?;
        let mut staged = self.held.clone();
        let mut answers = Vec::with_capacity(lines.len());
        let mut bytes = Vec::with_capacity(lines.len());
        for line in lines {
            if let Some(kept) = staged.operation(line.operation()) {
                if kept.same_words(&line) {
                    answers.push(kept.clone());
                    continue;
                }
                return Err(SeatError::OperationReused {
                    operation: line.operation().to_owned(),
                }
                .into());
            }
            checked(&staged, &line)?;
            staged.hold(line.clone()).map_err(unavailable)?;
            bytes.push(serde_json::to_vec(&line).map_err(unavailable)?);
            answers.push(line);
        }
        if bytes.is_empty() {
            return Ok(answers);
        }
        let count = u64::try_from(bytes.len()).map_err(unavailable)?;
        let since_snapshot = self
            .since_snapshot
            .checked_add(count)
            .ok_or_else(|| unavailable("the seats snapshot leaf count exceeds its range"))?;
        let index = self.log.len();
        let leaves: Vec<&[u8]> = bytes.iter().map(Vec::as_slice).collect();
        if let Err(failure) = self.log.append_batch(&leaves) {
            self.uncertain = true;
            self.settle()?;
            for (offset, expected) in (index..).zip(&bytes) {
                match self.log.leaf_bytes(offset).map_err(unavailable)? {
                    Some(held) if held.as_slice() == expected.as_slice() => {}
                    Some(_) => {
                        return Err(unavailable(format!(
                            "leaf {offset} was written by another writer: {failure}"
                        )));
                    }
                    None => {
                        return Err(unavailable(format!(
                            "batch leaf {offset} is missing after the append: {failure}"
                        )));
                    }
                }
            }
        } else {
            self.held = staged;
            self.since_snapshot = since_snapshot;
            if self.since_snapshot >= SNAPSHOT_EVERY.get() {
                self.write_snapshot();
            }
        }
        Ok(answers)
    }
}
