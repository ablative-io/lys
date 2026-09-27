//! Moving a home (HOME-019): `ship` makes a home a git repository tracking
//! exactly its tracked set and pushes it as the one ref `refs/lys/home` to a
//! bare repository at a path on this machine; `fetch` pulls that ref into a
//! new, empty home, verifies it strictly, and hangs one arrival beside each
//! session's head, committed as one commit whose parent is the fetched one.
//!
//! Invariants: git runs only as the binary on `PATH`, with a pinned
//! environment ([`git`]); a remote is only ever a path on this machine
//! ([`remote`]); ship writes nothing of the tracked set and never
//! force-pushes; fetch removes exactly what it created when it refuses.

pub mod fetch;
pub mod git;
#[cfg(test)]
mod git_tests;
pub mod remote;
#[cfg(test)]
mod remote_tests;
pub mod ship;
pub mod undo;
