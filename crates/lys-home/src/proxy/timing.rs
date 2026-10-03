//! Test measurements separate forwarding work from durable ingestion.

use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

pub(super) static DECODE: AtomicU64 = AtomicU64::new(0);
pub(super) static SPOOL_WRITE: AtomicU64 = AtomicU64::new(0);
pub(super) static SPOOL_SYNC: AtomicU64 = AtomicU64::new(0);
pub(super) static JOURNAL_WRITE: AtomicU64 = AtomicU64::new(0);
pub(super) static INGEST: AtomicU64 = AtomicU64::new(0);
pub(super) static FIRST_BYTE: AtomicU64 = AtomicU64::new(0);
pub(super) static LAST_BYTE: AtomicU64 = AtomicU64::new(0);
pub(super) static DURABLE: AtomicU64 = AtomicU64::new(0);
pub(super) static REPORT_SENT: AtomicU64 = AtomicU64::new(0);
static EPOCH: OnceLock<Instant> = OnceLock::new();

pub(super) fn tick() -> u64 {
    u64::try_from(EPOCH.get_or_init(Instant::now).elapsed().as_nanos()).unwrap_or(u64::MAX)
}

pub(super) fn mark(stage: &AtomicU64) {
    stage.store(tick(), Ordering::Relaxed);
}

pub(super) fn arriving() {
    let now = tick();
    let _ = FIRST_BYTE.compare_exchange(0, now, Ordering::Relaxed, Ordering::Relaxed);
    LAST_BYTE.store(now, Ordering::Relaxed);
}

pub(super) fn add(stage: &AtomicU64, started: Instant) {
    stage.fetch_add(
        u64::try_from(started.elapsed().as_nanos()).unwrap_or(u64::MAX),
        Ordering::Relaxed,
    );
}

pub(super) fn report() {
    let received = tick();
    for (name, stage) in [
        ("decode", &DECODE),
        ("spool_write", &SPOOL_WRITE),
        ("spool_sync", &SPOOL_SYNC),
        ("journal_write", &JOURNAL_WRITE),
        ("ingest", &INGEST),
    ] {
        println!("stage={name} nanoseconds={}", stage.load(Ordering::Relaxed));
    }
    for (name, start, end) in [
        (
            "upstream_arrivals",
            FIRST_BYTE.load(Ordering::Relaxed),
            LAST_BYTE.load(Ordering::Relaxed),
        ),
        (
            "last_arrival_to_durable",
            LAST_BYTE.load(Ordering::Relaxed),
            DURABLE.load(Ordering::Relaxed),
        ),
        (
            "durable_to_report_sent",
            DURABLE.load(Ordering::Relaxed),
            REPORT_SENT.load(Ordering::Relaxed),
        ),
        (
            "report_delivery",
            REPORT_SENT.load(Ordering::Relaxed),
            received,
        ),
    ] {
        println!("stage={name} nanoseconds={}", end.saturating_sub(start));
    }
}
