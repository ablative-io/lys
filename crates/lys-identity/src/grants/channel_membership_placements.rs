//! The placements a consumer's index applies (DIRECTORY-089 R1).
//!
//! A stream read that follows placements orders each placement after the
//! grant revision it was kept after. The index applies them strictly in
//! turn: a placement must be the next one, at exactly the revision the
//! index stands at; an earlier one is an identical duplicate or an
//! integrity failure, a later one a gap. A placement mark moves the
//! placement cursor over placements the consumer may not see, never ahead
//! of the revision the index stands at, and marks the index for readiness
//! when it names that revision.

use super::{Applied, MembershipIndex, STREAM_GAP, STREAM_INTEGRITY, StreamRefusal, refusal};
use crate::grants::change_stream::ChangeFrame;

impl MembershipIndex {
    /// Forget the placement cursor and what was applied at it.
    pub(super) fn forget_placements(&mut self) {
        self.placed = None;
        self.placements.clear();
        self.marked = false;
    }

    fn follows(&self) -> Result<(u64, u64), StreamRefusal> {
        if !self.following {
            return Err(refusal(
                STREAM_INTEGRITY,
                "a placement frame reached an index that does not follow placements",
            ));
        }
        let cursor = self
            .cursor
            .ok_or_else(|| refusal(STREAM_GAP, "no baseline has been applied"))?;
        let placed = self
            .placed
            .ok_or_else(|| refusal(STREAM_GAP, "no placement mark has been applied"))?;
        Ok((cursor, placed))
    }

    /// Apply one placement frame.
    pub(super) fn placement(&mut self, frame: &ChangeFrame) -> Result<Applied, StreamRefusal> {
        let ChangeFrame::Placement {
            revision,
            placement,
            widens,
            ..
        } = frame
        else {
            return Err(refusal(STREAM_INTEGRITY, "the frame is not a placement"));
        };
        let (cursor, placed) = self.follows()?;
        if *placement <= placed {
            return match self.placements.get(placement) {
                Some(held) if held == frame => Ok(Applied::PlacementDuplicate {
                    placement: *placement,
                }),
                Some(_) => Err(refusal(
                    STREAM_INTEGRITY,
                    format!("placement {placement} differs from the one applied"),
                )),
                None => Err(refusal(
                    STREAM_INTEGRITY,
                    format!("placement {placement} was already passed without this frame"),
                )),
            };
        }
        let next = placed.saturating_add(1);
        if *placement > next {
            return Err(StreamRefusal {
                name: STREAM_GAP,
                reason: format!("placements {next} to {} were omitted", placement - 1),
                gap: Some(placement - next),
            });
        }
        if *revision > cursor {
            return Err(StreamRefusal {
                name: STREAM_GAP,
                reason: format!(
                    "placement {placement} stands after revision {revision}; revisions {} to {revision} were omitted",
                    cursor.saturating_add(1)
                ),
                gap: Some(revision - cursor),
            });
        }
        if *revision < cursor {
            return Err(refusal(
                STREAM_INTEGRITY,
                format!(
                    "placement {placement} stands after revision {revision}, and the index has applied {cursor}"
                ),
            ));
        }
        self.placed = Some(*placement);
        self.marked = false;
        self.placements.insert(*placement, frame.clone());
        while self.placements.len() > self.window.get() {
            self.placements.pop_first();
        }
        Ok(Applied::Placement {
            placement: *placement,
            widens: widens.len(),
        })
    }

    /// Apply one placement mark.
    pub(super) fn mark(&mut self, revision: u64, placement: u64) -> Result<Applied, StreamRefusal> {
        if !self.following {
            return Err(refusal(
                STREAM_INTEGRITY,
                "a placement frame reached an index that does not follow placements",
            ));
        }
        let cursor = self
            .cursor
            .ok_or_else(|| refusal(STREAM_GAP, "no baseline has been applied"))?;
        if revision > cursor {
            return Err(StreamRefusal {
                name: STREAM_GAP,
                reason: format!(
                    "the placement mark stands at revision {revision}; revisions {} to {revision} were omitted",
                    cursor.saturating_add(1)
                ),
                gap: Some(revision - cursor),
            });
        }
        // The first mark after a baseline sets the placement cursor; later
        // ones only move it forward.
        let through = self
            .placed
            .map_or(placement, |placed| placed.max(placement));
        self.placed = Some(through);
        self.marked = revision == cursor;
        Ok(Applied::PlacementMark { placement: through })
    }
}
