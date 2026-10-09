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
mod stream;

pub use identity::{Reset, reset};

/// The grant log's served identity and its commit signal.
pub struct GrantChanges {
    identity: Mutex<Option<LogName>>,
    revision: watch::Sender<u64>,
}

impl Default for GrantChanges {
    fn default() -> Self {
        Self {
            identity: Mutex::new(None),
            revision: watch::channel(0).0,
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
    let acting_for = asker(&state, &headers)?;
    let log = served(&state)?;
    // Subscribed before the hold: a commit after the read wakes this wait.
    let mut live = state.grant_setup.changes.revision.subscribe();
    let read = with_grants(&state, |mut judged| {
        stream::page(&mut judged, acting_for.as_deref(), &log, &request)
    })?;
    if !(request.wait && read.idle) {
        return Ok(Json(read.page));
    }
    drop(read);
    let place = state.changes.hold_place()?;
    live.changed()
        .await
        .map_err(|error| ServerError::RuntimeUnavailable {
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

    use super::{ChangesPageSchema, ChangesRequestSchema};

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
                ChangeFrame::Ready { revision: 3 },
                ChangeFrame::Unready {
                    refusal: "StaleDecision".to_owned(),
                    reason: "behind".to_owned(),
                },
                ChangeFrame::Reset {
                    reason: ResetReason::Rollback,
                    from: Some(log.clone()),
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
        };
        let wire = serde_json::to_value(&request)?;
        let published: ChangesRequestSchema = serde_json::from_value(wire.clone())?;
        assert_eq!(serde_json::to_value(&published)?, wire);
        Ok(())
    }
}
