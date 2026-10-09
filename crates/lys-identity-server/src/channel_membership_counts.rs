//! What the membership capability keeps and counts (ACCESS-006 R5, R6),
//! read at `GET /grants/membership/counts` by the administrator.
//!
//! Every count is of work done, never of time: logical calls of the
//! capability, decisions made, index entries read as candidates, page rows
//! emitted, skipped and pages refused, holds of the grant authority (each
//! takes the directory, apps and grants locks once, in that order) and of
//! the cursor book's lock, and what is retained: the cursors kept and
//! released, and the entries of the grant book's live indexes. A count is
//! never reset while the service runs; a test reads it before and after the
//! work it measures. Physical store reads are counted by the log store
//! where it reads (`lys_log_store::process_read_count`) and shown here as a
//! process-wide reading; durable flushes are counted by the log store too
//! (`lys_log_store::process_flush_count`), at the real backend. The readers
//! registered waiting on a change signal are shown as they stand, so an idle
//! service is seen to hold none.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use axum::Json;
use axum::extract::State;
use axum::http::HeaderMap;
use serde::Serialize;

use lys_pass::membership_pages::CURSORS_UNAVAILABLE;

use crate::channel_membership::Named;
use crate::channel_membership_cursors::CursorBook;
use crate::config::MembershipSettings;
use crate::error::ServerError;
use crate::grants::with_grants;
use crate::grants_batch::asker;
use crate::routes::AppState;

/// The work counters.
#[derive(Debug, Default)]
pub(crate) struct Counts {
    calls: AtomicU64,
    decisions: AtomicU64,
    probes: AtomicU64,
    emitted: AtomicU64,
    skipped: AtomicU64,
    refused_pages: AtomicU64,
    authority_holds: AtomicU64,
    cursor_locks: AtomicU64,
}

fn add(counter: &AtomicU64, by: u64) {
    counter.fetch_add(by, Ordering::Relaxed);
}

impl Counts {
    /// One logical call of the capability, and the authority hold it takes.
    pub(crate) fn call(&self) {
        add(&self.calls, 1);
        add(&self.authority_holds, 1);
    }

    /// One decision made through the grants.
    pub(crate) fn decision(&self) {
        add(&self.decisions, 1);
    }

    /// `n` index entries read.
    pub(crate) fn probe(&self, n: u64) {
        add(&self.probes, n);
    }

    /// A page answered with `emitted` rows and `skipped` candidates.
    pub(crate) fn page(&self, emitted: u64, skipped: u64) {
        add(&self.emitted, emitted);
        add(&self.skipped, skipped);
    }

    /// A page refused whole.
    pub(crate) fn refused_page(&self) {
        add(&self.refused_pages, 1);
    }

    /// One hold of the cursor book's lock.
    pub(crate) fn cursor_lock(&self) {
        add(&self.cursor_locks, 1);
    }
}

/// The membership capability's state: the operator's settings, the kept
/// cursors and the counters.
#[derive(Debug)]
pub struct Membership {
    /// The operator's page bounds and cursor retention; absent, no page is
    /// served.
    pub(crate) settings: Option<MembershipSettings>,
    /// The kept cursors, within the settings' retention.
    pub(crate) cursors: Mutex<CursorBook>,
    /// The work counters.
    pub(crate) counts: Arc<Counts>,
}

impl Membership {
    /// The capability's state under `settings`, keeping no cursor yet.
    #[must_use]
    pub fn new(settings: Option<MembershipSettings>) -> Self {
        let (seconds, most) = settings.map_or((0, 0), |settings| {
            (settings.cursor_seconds, settings.cursors_max)
        });
        Self {
            settings,
            cursors: Mutex::new(CursorBook::new(seconds, most)),
            counts: Arc::new(Counts::default()),
        }
    }

    /// Hold the cursor book, counted, or the page refusal naming why not.
    pub(crate) fn cursors(&self) -> Result<std::sync::MutexGuard<'_, CursorBook>, Named> {
        self.counts.cursor_lock();
        self.cursors.lock().map_err(|error| {
            (
                CURSORS_UNAVAILABLE.to_owned(),
                format!("the membership cursor lock is poisoned: {error}"),
            )
        })
    }
}

/// Every count, as the counts route answers it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
pub struct MembershipCounts {
    /// Logical calls of the decision, page and admission routes.
    pub calls: u64,
    /// Decisions made through the grants.
    pub decisions: u64,
    /// Index entries read as candidates or placements.
    pub index_probes: u64,
    /// Page rows emitted.
    pub rows_emitted: u64,
    /// Page candidates decided and left out.
    pub rows_skipped: u64,
    /// Pages refused whole.
    pub pages_refused: u64,
    /// Holds of the grant authority by the capability.
    pub authority_holds: u64,
    /// Holds of the cursor book's lock.
    pub cursor_locks: u64,
    /// Cursors kept now.
    pub cursors_retained: u64,
    /// Cursors released by completion, expiry or reset.
    pub cursors_released: u64,
    /// (holder, grant) entries the live index keeps now.
    pub live_holder_entries: u64,
    /// (resource, grant) entries the live index keeps now.
    pub live_resource_entries: u64,
    /// Physical leaf and snapshot reads every log store in this process has
    /// made (`lys_log_store::process_read_count`): a reading to take the
    /// difference of across work while nothing else reads.
    pub physical_reads: u64,
    /// Readers registered waiting on a change signal now, the change
    /// stream's readiness waits among them. An idle service has none: no
    /// membership answer registers a wait, a timer or a poll.
    pub readiness_waiting: u64,
}

fn wide(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}

/// The capability's counts, to the administrator alone: an app's counts
/// would describe other apps' traffic.
pub async fn counts(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<MembershipCounts>, ServerError> {
    if asker(&state, &headers)?.is_some() {
        return Err(ServerError::NotAdmitted {
            reason: "only the administrator reads the membership counts",
        });
    }
    // Read before the grants are held, so this reading's own work is never
    // in it.
    let physical_reads = lys_log_store::process_read_count();
    let (holders, resources) =
        with_grants(&state, |judged| Ok(judged.grants.book().live_entries()))?;
    // Read uncounted, so reading the counts never moves them; a poisoned
    // book is refused by name, never read past.
    let (retained, released) = {
        let book = state.membership.cursors.lock().map_err(|error| {
            ServerError::MembershipUnavailable {
                reason: format!("the membership cursor lock is poisoned: {error}"),
            }
        })?;
        (book.retained(), book.released())
    };
    let counts = &state.membership.counts;
    let read = |counter: &AtomicU64| counter.load(Ordering::Relaxed);
    Ok(Json(MembershipCounts {
        calls: read(&counts.calls),
        decisions: read(&counts.decisions),
        index_probes: read(&counts.probes),
        rows_emitted: read(&counts.emitted),
        rows_skipped: read(&counts.skipped),
        pages_refused: read(&counts.refused_pages),
        authority_holds: read(&counts.authority_holds),
        cursor_locks: read(&counts.cursor_locks),
        cursors_retained: wide(retained),
        cursors_released: released,
        live_holder_entries: wide(holders),
        live_resource_entries: wide(resources),
        physical_reads,
        readiness_waiting: wide(state.changes.waiting()),
    }))
}
