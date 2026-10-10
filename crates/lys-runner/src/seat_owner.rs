//! The independent owner of a supervised seat (AGENTS-004).
//!
//! A seat AGENTS-002 starts is owned by a process of its own, detached from
//! the terminal that watches it, the runner that launched it and the identity
//! server that admits its acts: closing or killing any of those clients
//! closes none of the seat's descriptors and signals nothing to its harness.
//! The runner and the identity server connect to that owner as authenticated
//! clients; neither launches a second harness for a seat that has one.
//!
//! One writer per session: the owner holds an exclusive lease fenced by a
//! monotonic generation ([`store::Lease`]), recorded durably before it is
//! acted on, so a replacement runner, a restarted identity server or an
//! upgrade's successor binds the same seat, session and generation or is
//! refused by name. Deliberate stop stays AGENTS-002's act under fresh
//! authority; a runner or service restart, and an upgrade, keep the session.
//!
//! - [`store`]: the versioned owner, lease, custody and cursor records, an
//!   indexed projection plus a bounded recovery tail (R4).
//! - [`protocol`]: the typed owner command and its answers (R1).
//! - [`process`]: the owner process, its lease and its harness (R1).
//! - [`recovery`]: rebinding a replacement client to a live owner (R2).
//! - [`handoff`]: custody transfer to an upgrade's successor (R3).
//! - [`counts`]: the counted work of every hot path and its ratchet (R5).

pub mod counts;
pub mod process;
pub mod protocol;
pub mod record;
pub mod recovery;
pub mod rules;
pub mod sessions;
pub mod spawn;
pub mod store;
