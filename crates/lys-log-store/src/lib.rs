//! Durable, append-only, write-once storage for a lys transparency log.
//!
//! `lys-core` performs no I/O by design: it holds the trust primitives, and a
//! primitive that opens files is a primitive nobody can audit in isolation.
//! This crate is where persistence lives instead — the [`LeafStore`] trait that
//! says what a log needs from storage, a [`FileLeafStore`] that provides it from
//! a directory, and the [`Log`] that maintains the RFC 6962 tree over either.
//!
//! ```no_run
//! use lys_log_store::{FileLeafStore, Log};
//!
//! # fn main() -> Result<(), lys_log_store::StoreError> {
//! let dir = std::path::Path::new("/tmp/example-log");
//! FileLeafStore::create(dir, "example.com/lys/demo")?;
//!
//! let mut log = Log::open(FileLeafStore::open(dir)?)?;
//! let (index, leaf_hash) = log.append(b"the first entry")?;
//! assert_eq!(index, 0);
//! assert_eq!(log.tree().len(), 1);
//! # let _ = leaf_hash;
//! # Ok(())
//! # }
//! ```
//!
//! # The invariants this crate exists to hold
//!
//! - **Write-once.** A stored leaf is never rewritten, and a second write to an
//!   occupied index is a reported error rather than an overwrite. Index reuse
//!   means two writers believe they own one position, which is worth hearing
//!   about.
//! - **No gaps, and no silent ones.** Writes are refused unless they land at the
//!   next free index, so the only way to reach a missing leaf is through an
//!   error a caller received. A **reported** gap is attributable to a specific
//!   refused write; a silent one is indistinguishable from an entry nobody ever
//!   offered.
//! - **Durable on return.** `Ok` from a write means the bytes reached stable
//!   storage, file and directory both. A storage layer that acknowledges writes
//!   it has not persisted manufactures history the log has already counted —
//!   arriving from the one direction a log's own checks cannot attribute to
//!   anybody, because they are built to catch a dishonest operator and this is
//!   not one.
//! - **No fork and no merge**, at all, deliberately — see [`store`] for why a
//!   branch of an append-only log is equivocation with a nicer name.
//!
//! # What is storage's business and what is not
//!
//! Storage holds an ordered trail of opaque bytes plus a name. It knows nothing
//! about keys, signatures, identities, or rotation: those are events *in* the
//! trail, appended like anything else. Keeping that line clean is what lets the
//! backend be replaced without renegotiating what the log means.

pub mod error;
pub mod file;
pub mod log;
pub mod store;

pub use error::{StoreError, StoreResult};
pub use file::FileLeafStore;
pub use log::{Log, validate_origin};
pub use store::{LeafStore, PinnedRoot};
