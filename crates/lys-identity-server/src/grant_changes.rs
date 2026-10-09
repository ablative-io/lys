//! The grant change stream (DIRECTORY-089 R1): `POST /grants/changes`.
//!
//! A consumer opens the stream itself through Lys's published API; Lys never
//! calls a product, polls on a clock or holds a consumer's buffer. Each read
//! names the grant log and the last revision the consumer applied, and is
//! answered with the frames strictly after it (`stream`), under the one
//! hold every commit takes. It is a question, open to the administrator and
//! to an app through its credential, exactly as the batch check is, and an
//! app is shown only the changes on its own kinds: subscribing grants no
//! authority.
//!
//! A read that asks to wait, and finds nothing after its cursor, waits for
//! the grants' own commit signal, never a clock: the signal is subscribed
//! before the hold is taken, so a commit after the read is never missed, and
//! a commit before it is already in the read. The wait takes one place among
//! the change subscriptions' shared bound and gives it back when the read
//! ends or its connection is dropped; no task outlives the request. After
//! the wait the asker is judged again and the page read again.
//!
//! A read following placements (`placements`) is given them in order with
//! the changes, and its wait is also released by the placement signal the
//! placements route sends under the same hold, subscribed the same way.
//!
//! The log's identity (`identity`) is recorded once and is the one every
//! membership decision and pass binding names.

use std::path::Path;
use std::sync::{Arc, Mutex};

use axum::Json;
use axum::extract::State;
use axum::extract::rejection::JsonRejection;
use axum::http::HeaderMap;
use lys_identity::grants::change_stream::{ChangesPage, ChangesRequest, LogName};
use serde::{Deserialize, Serialize};
use tokio::sync::watch;

use crate::error::ServerError;
use crate::error_grant_stream::GrantStreamError;
use crate::grants::with_grants;
use crate::grants_batch::asker;
use crate::routes::AppState;

mod identity;
mod placements;
mod stream;

pub use identity::{Reset, reset};

/// The grant log's served identity, its commit signal and its placement
/// signal.
pub struct GrantChanges {
    identity: Mutex<Option<LogName>>,
    revision: watch::Sender<u64>,
    placements: watch::Sender<u64>,
}

impl Default for GrantChanges {
    fn default() -> Self {
        Self {
            identity: Mutex::new(None),
            revision: watch::channel(0).0,
            placements: watch::channel(0).0,
        }
    }
}

impl GrantChanges {
    /// The identity of the grant log kept at `log_dir`, recorded once from
    /// its `origin` and held from then on.
    ///
    /// # Errors
    /// `grant_log_identity_unavailable`, by name; no identity is guessed.
    pub(crate) fn served(&self, log_dir: &Path, origin: &str) -> Result<LogName, ServerError> {
        let mut held =
            self.identity
                .lock()
                .map_err(|error| GrantStreamError::IdentityUnavailable {
                    reason: format!("the grant log identity lock is poisoned: {error}"),
                })?;
        if let Some(name) = &*held {
            return Ok(name.clone());
        }
        let name = identity::establish(log_dir, origin)?;
        *held = Some(name.clone());
        Ok(name)
    }

    /// Signal the grants' revision after a hold of them ends: waiting
    /// readers wake only when it moved.
    pub(crate) fn published(&self, revision: u64) {
        self.revision.send_if_modified(|held| {
            if *held == revision {
                false
            } else {
                *held = revision;
                true
            }
        });
    }

    /// Signal the count of placements once one is kept, under the hold
    /// that kept it: readers following placements wake only when it moved.
    pub(crate) fn placed(&self, count: u64) {
        self.placements.send_if_modified(|held| {
            if *held == count {
                false
            } else {
                *held = count;
                true
            }
        });
    }
}

/// The grant log this service serves, by its recorded identity and epoch.
///
/// # Errors
/// `grant_log_identity_unavailable` when it cannot be read or recorded.
pub(crate) fn served(state: &AppState) -> Result<LogName, ServerError> {
    let setup = &state.grant_setup;
    setup.changes.served(&setup.log_dir, &setup.log_origin)
}

fn malformed(reason: &str) -> ServerError {
    ServerError::RequestMalformed {
        reason: reason.to_owned(),
    }
}

/// Read the grant change stream after the consumer's cursor.
pub async fn changes(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Result<Json<ChangesRequest>, JsonRejection>,
) -> Result<Json<ChangesPage>, ServerError> {
    let Json(request) = body.map_err(|refused| ServerError::RequestMalformed {
        reason: refused.body_text(),
    })?;
    if request.limit == 0 {
        return Err(malformed("a read covers at least one log position"));
    }
    if request.after.is_some() && request.log.is_none() {
        return Err(malformed("a cursor names the grant log it belongs to"));
    }
    match (request.placements, request.after, request.placed) {
        (false, _, Some(_)) => {
            return Err(malformed(
                "a placement cursor is given only by a read following placements",
            ));
        }
        (true, Some(_), None) => {
            return Err(malformed(
                "a cursor following placements names the placements applied",
            ));
        }
        (true, None, Some(_)) => {
            return Err(malformed("a first read names no placements applied"));
        }
        _ => {}
    }
    let acting_for = asker(&state, &headers)?;
    let log = served(&state)?;
    // Subscribed before the hold: a commit or placement after the read
    // wakes this wait.
    let mut live = state.grant_setup.changes.revision.subscribe();
    let mut placing = state.grant_setup.changes.placements.subscribe();
    let read = with_grants(&state, |mut judged| {
        stream::page(&mut judged, acting_for.as_deref(), &log, &request)
    })?;
    if !(request.wait && read.idle) {
        return Ok(Json(read.page));
    }
    drop(read);
    let place = state.changes.hold_place()?;
    let woken = if request.placements {
        tokio::select! {
            woken = live.changed() => woken,
            woken = placing.changed() => woken,
        }
    } else {
        live.changed().await
    };
    woken.map_err(|error| ServerError::RuntimeUnavailable {
        reason: format!("the grant commit signal closed: {error}"),
    })?;
    drop(place);
    let acting_for = asker(&state, &headers)?;
    let read = with_grants(&state, |mut judged| {
        stream::page(&mut judged, acting_for.as_deref(), &log, &request)
    })?;
    Ok(Json(read.page))
}

/// The published shape of a grant log's name ([`LogName`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = GrantLogName)]
pub struct LogNameSchema {
    /// The identity recorded once for the log.
    pub identity: String,
    /// The reset epoch.
    pub epoch: u64,
}

/// The published shape of one read ([`ChangesRequest`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = GrantChangesRequest)]
pub struct ChangesRequestSchema {
    /// The log the cursor belongs to; absent for a first read.
    #[serde(default)]
    pub log: Option<LogNameSchema>,
    /// The last revision applied; absent for a first read.
    #[serde(default)]
    pub after: Option<u64>,
    /// How many log positions one answer may cover, at least one.
    pub limit: u64,
    /// Whether to wait for the next commit when nothing follows the cursor.
    #[serde(default)]
    pub wait: bool,
    /// Whether the read follows placements too.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub placements: bool,
    /// The count of placements applied; given with `after` when following.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub placed: Option<u64>,
}

/// The published shape of what a change did.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = GrantChangeKind)]
pub enum ChangeKindSchema {
    /// Issued.
    Issue,
    /// Revoked, with everything derived from it.
    Revoke,
}

/// The published shape of why a cursor cannot be continued.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = GrantStreamResetReason)]
pub enum ResetReasonSchema {
    /// Another log or epoch.
    OtherLog,
    /// Past what the log holds.
    Rollback,
    /// Not one position of the order of changes and placements.
    Unordered,
}

/// The published shape of one frame ([`lys_identity::grants::change_stream::ChangeFrame`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(tag = "frame", rename_all = "snake_case", deny_unknown_fields)]
#[schema(as = GrantChangeFrame)]
pub enum ChangeFrameSchema {
    /// The revoked grants the consumer may see, at one revision.
    Baseline {
        /// The revision.
        revision: u64,
        /// The revoked grant ids.
        revoked: Vec<String>,
    },
    /// One committed change.
    Change {
        /// Its receipt revision.
        revision: u64,
        /// The grant changed.
        grant: String,
        /// What it did.
        change: ChangeKindSchema,
        /// The signed grant event, lowercase hex.
        event: String,
    },
    /// Hidden changes through this revision.
    Watermark {
        /// The last revision covered.
        revision: u64,
    },
    /// One placement, kept after the change at `revision`.
    Placement {
        /// The grant revision it was kept after.
        revision: u64,
        /// Its position among the placements, from one.
        placement: u64,
        /// The resource placed, `kind:id`.
        child: String,
        /// Its parent, `kind:id`.
        parent: String,
        /// Whether nothing held on the parent reaches the child.
        restricted: bool,
        /// The grants whose reach it widened to the child.
        widens: Vec<String>,
    },
    /// Hidden placements through this count.
    PlacementMark {
        /// The grant revision the last covered placement was kept after.
        revision: u64,
        /// The count of placements covered.
        placement: u64,
    },
    /// Delivered and projected through this revision.
    Ready {
        /// The revision.
        revision: u64,
    },
    /// Readiness withheld by name.
    Unready {
        /// The stable name.
        refusal: String,
        /// Its words.
        reason: String,
    },
    /// The cursor cannot be continued.
    Reset {
        /// Why.
        reason: ResetReasonSchema,
        /// The log the cursor named.
        from: Option<LogNameSchema>,
    },
}

/// The published shape of one answer ([`ChangesPage`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = GrantChangesPage)]
pub struct ChangesPageSchema {
    /// The log every frame belongs to.
    pub log: LogNameSchema,
    /// The frames, in revision order.
    pub frames: Vec<ChangeFrameSchema>,
}

#[cfg(test)]
mod tests {
    use lys_identity::grants::change_stream::{
        ChangeFrame, ChangeKind, ChangesPage, ChangesRequest, LogName, ResetReason,
    };

    use super::{ChangesPageSchema, ChangesRequestSchema, GrantChanges};

    use std::future::Future;
    use std::pin::pin;
    use std::task::{Context, Poll, Waker};

    /// `d089_r1_replay_live_race`, signal half: the subscription is the owned
    /// installation signal and `published` the owned commit signal, placed
    /// exactly as the route places them (subscribe, then read the barrier
    /// under the hold; commit, then publish under the hold). A commit
    /// before the subscription is in the read; one between the read and the
    /// wait, or during the wait, releases it; none is lost and none is
    /// delivered twice, because the reread starts strictly after the cursor.
    /// No clock, sleep or timeout is used: each poll is one owned step.
    #[test]
    fn a_commit_at_any_point_of_a_wait_is_seen_once() {
        let mut context = Context::from_waker(Waker::noop());
        // Revoke before open: committed and published at r+1 before the
        // subscription; the read under the hold already holds it, so the
        // page is not idle and no wait is taken.
        let changes = GrantChanges::default();
        changes.published(5);
        let mut before = changes.revision.subscribe();
        assert_eq!(
            *before.borrow_and_update(),
            5,
            "the read's barrier holds r+1"
        );
        assert!(matches!(
            pin!(before.changed()).as_mut().poll(&mut context),
            Poll::Pending
        ));

        // Revoke at the barrier: subscribed, the barrier read at r, then the
        // commit publishes r+1 before the wait begins: the wait returns at once.
        let changes = GrantChanges::default();
        changes.published(4);
        let mut at_barrier = changes.revision.subscribe();
        let barrier = *at_barrier.borrow();
        changes.published(5);
        assert!(matches!(
            pin!(at_barrier.changed()).as_mut().poll(&mut context),
            Poll::Ready(Ok(()))
        ));
        assert_eq!(*at_barrier.borrow_and_update(), barrier + 1);

        // Revoke after Ready: the wait is pending until the commit's
        // signal, and is released by it alone.
        let changes = GrantChanges::default();
        changes.published(4);
        let mut after = changes.revision.subscribe();
        {
            let mut waiting = pin!(after.changed());
            assert!(matches!(waiting.as_mut().poll(&mut context), Poll::Pending));
            changes.published(4);
            assert!(
                matches!(waiting.as_mut().poll(&mut context), Poll::Pending),
                "a hold that committed nothing releases no wait"
            );
            changes.published(5);
            assert!(matches!(
                waiting.as_mut().poll(&mut context),
                Poll::Ready(Ok(()))
            ));
        }
        assert_eq!(*after.borrow_and_update(), 5);
    }

    /// The published schema types and the wire types read the same JSON,
    /// both ways, every frame named.
    #[test]
    fn the_published_shapes_are_the_wire_shapes() -> Result<(), Box<dyn std::error::Error>> {
        let log = LogName {
            identity: "example.test/lys/grants#1".to_owned(),
            epoch: 2,
        };
        let page = ChangesPage {
            log: log.clone(),
            frames: vec![
                ChangeFrame::Baseline {
                    revision: 1,
                    revoked: vec!["grant-00".to_owned()],
                },
                ChangeFrame::Change {
                    revision: 2,
                    grant: "grant-01".to_owned(),
                    change: ChangeKind::Revoke,
                    event: "d2".to_owned(),
                },
                ChangeFrame::Watermark { revision: 3 },
                ChangeFrame::Placement {
                    revision: 3,
                    placement: 1,
                    child: "app.channel:general".to_owned(),
                    parent: "app.workspace:team".to_owned(),
                    restricted: false,
                    widens: vec!["grant-02".to_owned()],
                },
                ChangeFrame::PlacementMark {
                    revision: 3,
                    placement: 2,
                },
                ChangeFrame::Ready { revision: 3 },
                ChangeFrame::Unready {
                    refusal: "StaleDecision".to_owned(),
                    reason: "behind".to_owned(),
                },
                ChangeFrame::Reset {
                    reason: ResetReason::Rollback,
                    from: Some(log.clone()),
                },
                ChangeFrame::Reset {
                    reason: ResetReason::Unordered,
                    from: None,
                },
            ],
        };
        let wire = serde_json::to_value(&page)?;
        let published: ChangesPageSchema = serde_json::from_value(wire.clone())?;
        assert_eq!(serde_json::to_value(&published)?, wire);
        let request = ChangesRequest {
            log: Some(log),
            after: Some(3),
            limit: 8,
            wait: true,
            placements: false,
            placed: None,
        };
        let wire = serde_json::to_value(&request)?;
        assert!(
            wire.get("placements").is_none() && wire.get("placed").is_none(),
            "a read not following placements keeps its bytes: {wire}"
        );
        let published: ChangesRequestSchema = serde_json::from_value(wire.clone())?;
        assert_eq!(serde_json::to_value(&published)?, wire);
        let following = ChangesRequest {
            placements: true,
            placed: Some(2),
            ..request
        };
        let wire = serde_json::to_value(&following)?;
        let published: ChangesRequestSchema = serde_json::from_value(wire.clone())?;
        assert_eq!(serde_json::to_value(&published)?, wire);
        Ok(())
    }
}
