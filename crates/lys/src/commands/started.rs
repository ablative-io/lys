//! When this process started, so a service says on its ready line how long its start took
//! (Tom, 8 Oct 2026: a start in seconds is a defect; one nobody timed cannot be shown fast).

use std::sync::OnceLock;
use std::time::Instant;

static STARTED: OnceLock<Instant> = OnceLock::new();

/// Marks this process's start; the first call wins. Called first thing in `main`.
pub fn mark() {
    STARTED.get_or_init(Instant::now);
}

/// Milliseconds since [`mark`]; since the first call here when `mark` was never called.
#[must_use]
pub fn elapsed_ms() -> u128 {
    STARTED.get_or_init(Instant::now).elapsed().as_millis()
}
