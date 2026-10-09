//! One page of the grant change stream, read under the grants' one hold
//! (DIRECTORY-089 R1).
//!
//! The page is read while `with_grants` holds the directory, the apps and the
//! grants, the same serialization every commit takes, so its barrier, the
//! log's head, is one revision no commit is half way through. The log is
//! settled and projected first; replay then reads only the positions after
//! the cursor, in one pass from the checkpoint at or below it
//! (`GrantLedger::entries_between`), never the history before it. A change is shown when the asker may see it: the
//! administrator sees every one, an app each one on a grant of its own
//! kinds or with a descendant on them. The rest are covered by watermarks.
//! A baseline is read from the grant book, never from the log. No network
//! write, wait or consumer is held under the hold.

use std::collections::BTreeSet;

use lys_identity::grants::change_stream::{
    ChangeFrame, ChangeKind, ChangesPage, ChangesRequest, LogName, ResetReason, event_hex,
};
use lys_identity::grants::{
    GrantBook, GrantChange, GrantError, GrantEvent, GrantId, GrantRecord, Source, owner_of,
};
use lys_log_store::LeafStore;

use crate::channel_membership::named;
use crate::error::ServerError;
use crate::grants::Judged;

/// What one read answered, and whether nothing followed its cursor.
pub(super) struct Read {
    pub(super) page: ChangesPage,
    pub(super) idle: bool,
}

/// Whether `acting_for` may see a change to `grant`: the administrator any,
/// an app one on its own kinds or with a descendant on them.
fn sees(book: &GrantBook, acting_for: Option<&str>, grant: GrantId) -> bool {
    let Some(app) = acting_for else {
        return true;
    };
    std::iter::once(grant)
        .chain(book.descendants(grant))
        .filter_map(|id| book.grant(id))
        .any(|held| owner_of(held.resource().kind()) == app)
}

/// The change `event` shows `acting_for`, if any. A use is never a grant
/// change, and an event the book refused changed nothing.
fn shown(
    book: &GrantBook,
    acting_for: Option<&str>,
    event: &GrantEvent,
    index: u64,
) -> Option<ChangeKind> {
    if book.refused().contains_key(&index) {
        return None;
    }
    let kind = match event.change() {
        GrantChange::Issue(_) => ChangeKind::Issue,
        GrantChange::Revoke { .. } => ChangeKind::Revoke,
        GrantChange::Use { .. } => return None,
    };
    sees(book, acting_for, event.grant()).then_some(kind)
}

/// The revoked grants `acting_for` may see: every one for the
/// administrator; for an app, each revoked grant on its kinds or an
/// ancestor of one, each grant read once.
fn revoked(book: &GrantBook, acting_for: Option<&str>) -> Vec<String> {
    let mut passed = BTreeSet::new();
    let mut found = BTreeSet::new();
    let records: Box<dyn Iterator<Item = &GrantRecord> + '_> = match acting_for {
        None => Box::new(book.records()),
        Some(app) => Box::new(book.in_app(app)),
    };
    for record in records {
        let mut next = Some(record.grant().id());
        while let Some(id) = next {
            if !passed.insert(id) {
                break;
            }
            let Some(held) = book.record(id) else {
                break;
            };
            if held.revoked().is_some() {
                found.insert(id.to_string());
            }
            next = match held.grant().parts().source {
                Source::Grant(source) => Some(source),
                Source::Root => None,
            };
        }
    }
    found.into_iter().collect()
}

/// Ready through `head`, or readiness withheld by the name of what is not
/// complete: a projection behind the head or a degraded reading.
fn readiness<S: LeafStore>(judged: &mut Judged<'_, S>, head: u64) -> ChangeFrame {
    match judged.grants.frame(judged.directory, Some(head)) {
        Ok(frame) => match frame.degradation() {
            None => ChangeFrame::Ready { revision: head },
            Some(degraded) => {
                let (refusal, reason) = named(degraded.error().clone());
                ChangeFrame::Unready { refusal, reason }
            }
        },
        Err(error) => {
            let (refusal, reason) = named(error);
            ChangeFrame::Unready { refusal, reason }
        }
    }
}

/// One page for `acting_for` from the log `served`, after `request`'s cursor.
pub(super) fn page<S: LeafStore>(
    judged: &mut Judged<'_, S>,
    acting_for: Option<&str>,
    served: &LogName,
    request: &ChangesRequest,
) -> Result<Read, ServerError> {
    let answer = |frames, idle| Read {
        page: ChangesPage {
            log: served.clone(),
            frames,
        },
        idle,
    };
    if let Some(asked) = &request.log
        && asked != served
    {
        let reset = ChangeFrame::Reset {
            reason: ResetReason::OtherLog,
            from: Some(asked.clone()),
        };
        return Ok(answer(vec![reset], false));
    }
    // Settle and project first: an uncertain append is resolved, or
    // refused by name, before any position is read.
    judged.grants.frame(judged.directory, None)?;
    let head = judged.grants.revision();
    let mut frames = Vec::new();
    let cursor = match request.after {
        None => {
            frames.push(ChangeFrame::Baseline {
                revision: head,
                revoked: revoked(judged.grants.book(), acting_for),
            });
            head
        }
        Some(after) if after > head => {
            let reset = ChangeFrame::Reset {
                reason: ResetReason::Rollback,
                from: request.log.clone(),
            };
            return Ok(answer(vec![reset], false));
        }
        Some(after) => after,
    };
    let through = head.min(cursor.saturating_add(request.limit));
    let mut hidden = None;
    // One pass from the checkpoint at or below the cursor: the replay's
    // cost is the checkpoint distance and the range, never the history.
    let replayed = judged.grants.ledger().entries_between(cursor, through)?;
    let expected = usize::try_from(through - cursor).ok();
    if Some(replayed.len()) != expected {
        return Err(GrantError::LogUnavailable {
            reason: format!(
                "grant log positions {cursor} to {through} are not all below the head {head}"
            ),
        }
        .into());
    }
    for (signed, coordinate) in &replayed {
        let index = coordinate.index;
        let revision = coordinate.index.saturating_add(1);
        let event = signed.event();
        match shown(judged.grants.book(), acting_for, event, index) {
            Some(change) => {
                if let Some(covered) = hidden.take() {
                    frames.push(ChangeFrame::Watermark { revision: covered });
                }
                frames.push(ChangeFrame::Change {
                    revision,
                    grant: event.grant().to_string(),
                    change,
                    event: event_hex(signed.bytes()),
                });
            }
            None => hidden = Some(revision),
        }
    }
    if let Some(covered) = hidden {
        frames.push(ChangeFrame::Watermark { revision: covered });
    }
    let idle = request.after == Some(head);
    if through == head {
        frames.push(readiness(judged, head));
    }
    Ok(answer(frames, idle))
}
