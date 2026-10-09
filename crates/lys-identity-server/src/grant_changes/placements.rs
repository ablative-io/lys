//! Placements in the grant change stream (DIRECTORY-089 R1).
//!
//! A placement changes what a grant reaches without being a grant event: it
//! is kept in the apps log, under the grants' one hold, with the grant
//! revision it follows (`Placed::revision`). A placement kept after
//! revision `r` stands after the change at `r` and before the change at
//! `r + 1`, so changes and placements are one order, and a read following
//! placements walks it from its cursor: the revision applied and the count
//! of placements applied. The walk reads only the placements after the
//! cursor, by position, and takes at most the read's limit of positions.
//! A cursor that is not one position of the order, or a placement after it
//! kept before placements were ordered, is answered with a named reset,
//! never skipped.
//!
//! What a placement widened is read as of its own position: the grants on
//! its parent, and on each resource the parent was placed in by an earlier
//! unrestricted placement, issued at or before its revision and not
//! withdrawn by then. A later grant or placement never changes the frame,
//! so a placement read again is the same frame.

use std::collections::BTreeSet;

use lys_identity::grants::change_stream::{ChangeFrame, ResetReason};
use lys_identity::grants::{GrantBook, GrantError, GrantId, Resource, Source};

use super::stream::sees;
use crate::apps_state::{Held, Placed};
use crate::error::ServerError;

/// One position of the order of changes and placements.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Step {
    /// The grant change at this revision.
    Change(u64),
    /// The placement at this position, counting from one.
    Placement(u64),
}

/// The positions one read covers, and where they end.
pub(super) struct Planned {
    /// The positions, in order.
    pub(super) steps: Vec<Step>,
    /// The last grant revision covered.
    pub(super) revision: u64,
    /// The count of placements covered, when the read follows them.
    pub(super) placed: Option<u64>,
}

/// The positions of one read, or the reset its cursor needs.
pub(super) enum Walk {
    Planned(Planned),
    Reset(ResetReason),
}

fn unavailable(reason: String) -> ServerError {
    GrantError::LogUnavailable { reason }.into()
}

/// How many placements the apps hold.
pub(super) fn count(held: &Held) -> Result<u64, ServerError> {
    u64::try_from(held.placements.len())
        .map_err(|error| unavailable(format!("the placements cannot be counted: {error}")))
}

/// The placement at `position`, counting from one, with its index.
pub(super) fn at(held: &Held, position: u64) -> Result<(usize, &Placed), ServerError> {
    let index = position
        .checked_sub(1)
        .ok_or_else(|| unavailable("placements are counted from one".to_owned()))?;
    let index = usize::try_from(index)
        .map_err(|error| unavailable(format!("placement {position} is not a position: {error}")))?;
    let placed = held
        .placements
        .get(index)
        .ok_or_else(|| unavailable(format!("placement {position} is not held")))?;
    Ok((index, placed))
}

/// The positions after the cursor (`cursor`, and `placed` when the read
/// follows placements), at most `limit` of them, through the grants' `head`.
pub(super) fn walk(
    apps: &Held,
    placed: Option<u64>,
    cursor: u64,
    head: u64,
    limit: u64,
) -> Result<Walk, ServerError> {
    if let Some(applied) = placed {
        if applied > count(apps)? {
            return Ok(Walk::Reset(ResetReason::Rollback));
        }
        // The last placement applied must stand at or before the revision
        // applied: a cursor past a placement it never reached is no position.
        if applied > 0 {
            let (_, last) = at(apps, applied)?;
            if last.revision.is_some_and(|kept| kept > cursor) {
                return Ok(Walk::Reset(ResetReason::Unordered));
            }
        }
    }
    let mut steps = Vec::new();
    let mut revision = cursor;
    let mut position = placed;
    let mut taken = 0_u64;
    while taken < limit {
        // The next placement, by its position counting from one, and the
        // revision it was kept after.
        let next = match position {
            Some(applied) => {
                let index = usize::try_from(applied).map_err(|error| {
                    unavailable(format!("placement {applied} is not a position: {error}"))
                })?;
                apps.placements
                    .get(index)
                    .map(|next| (applied + 1, next.revision))
            }
            None => None,
        };
        match next {
            Some((after, Some(kept))) if kept == revision => {
                position = Some(after);
                steps.push(Step::Placement(after));
            }
            Some((_, Some(kept))) if kept > head => return Ok(Walk::Reset(ResetReason::Rollback)),
            // Kept before placements were ordered, or before a revision the
            // cursor has already passed.
            Some((_, kept)) if kept.is_none_or(|kept| kept < revision) => {
                return Ok(Walk::Reset(ResetReason::Unordered));
            }
            Some(_) | None => {
                if revision >= head {
                    break;
                }
                revision += 1;
                steps.push(Step::Change(revision));
            }
        }
        taken += 1;
    }
    Ok(Walk::Planned(Planned {
        steps,
        revision,
        placed: position,
    }))
}

/// Whether `acting_for` may see `placed`: the administrator any, an app
/// its own, since a placement is only ever between one app's kinds.
pub(super) fn shown(acting_for: Option<&str>, placed: &Placed) -> bool {
    acting_for.is_none_or(|app| placed.app == app)
}

/// Whether `grant` and every grant it derives from stood unwithdrawn
/// through `revision`.
fn standing(book: &GrantBook, grant: GrantId, revision: u64) -> bool {
    let mut passed = BTreeSet::new();
    let mut next = Some(grant);
    while let Some(id) = next {
        if !passed.insert(id) {
            return false;
        }
        let Some(record) = book.record(id) else {
            return false;
        };
        if record
            .revoked()
            .is_some_and(|revoked| revoked.index < revision)
        {
            return false;
        }
        next = match record.grant().parts().source {
            Source::Grant(source) => Some(source),
            Source::Root => None,
        };
    }
    true
}

/// The grants `acting_for` may see whose reach the placement at `index`,
/// kept after `revision`, widened to its child, in id order.
fn widened(
    held: &Held,
    book: &GrantBook,
    acting_for: Option<&str>,
    index: usize,
    placed: &Placed,
    revision: u64,
) -> Result<Vec<String>, ServerError> {
    // A restricted placement passes nothing down, and a child placed
    // before keeps its first parent.
    if placed.restricted
        || held.parent_position(&placed.child_kind, &placed.child_id) != Some(index)
    {
        return Ok(Vec::new());
    }
    let parent = Resource::new(&placed.parent_kind, &placed.parent_id)?;
    let mut reached = vec![parent.clone()];
    let mut current = parent;
    while let Some(above) = held.parent_position(current.kind(), current.id()) {
        // Only a placement kept before this one is part of what it widened.
        if above >= index {
            break;
        }
        let up = held.placements.get(above).ok_or_else(|| {
            unavailable(format!(
                "the apps index names placement {above}, which is not held"
            ))
        })?;
        if up.restricted {
            break;
        }
        let next = Resource::new(&up.parent_kind, &up.parent_id)?;
        if reached.contains(&next) {
            break;
        }
        reached.push(next.clone());
        current = next;
    }
    let mut widens = BTreeSet::new();
    for resource in &reached {
        for record in book.on_resource(resource) {
            let id = record.grant().id();
            if record.index() < revision
                && standing(book, id, revision)
                && sees(book, acting_for, id)
            {
                widens.insert(id);
            }
        }
    }
    Ok(widens.into_iter().map(|id| id.to_string()).collect())
}

/// The frame of the placement at `position`, kept after `revision`.
pub(super) fn frame(
    held: &Held,
    book: &GrantBook,
    acting_for: Option<&str>,
    position: u64,
    revision: u64,
) -> Result<ChangeFrame, ServerError> {
    let (index, placed) = at(held, position)?;
    let child = Resource::new(&placed.child_kind, &placed.child_id)?;
    let parent = Resource::new(&placed.parent_kind, &placed.parent_id)?;
    Ok(ChangeFrame::Placement {
        revision,
        placement: position,
        child: child.to_string(),
        parent: parent.to_string(),
        restricted: placed.restricted,
        widens: widened(held, book, acting_for, index, placed, revision)?,
    })
}
