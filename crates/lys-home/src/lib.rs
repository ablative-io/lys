//! The home: a session held under its identity, in the shape of Pi's session tree.
//!
//! A home holds sessions as append-only JSONL files in the grammar Pi's coding
//! agent writes (a `session` header line, then entries that each carry `id`,
//! `parentId` and `timestamp`), so a home file is readable by Pi's own parser
//! unchanged. Everything lys adds rides inside Pi's `custom` entry type under
//! `lys.*` custom types: harness events, proxy call records, the authored marker.
//!
//! Beside each session file sit two things Pi does not keep: an offset index,
//! so the path from the head back to the root is read by seeking to the entries
//! on it and never by loading the file, and a persisted head, so reopening
//! restores where the session was rather than the last entry appended.
//!
//! Content blocks (message parts, request and response bodies) live once in a
//! content-addressed store under the home, named by SHA-256; entries reference
//! them by hash.
//!
//! Nothing here interprets, prints or logs transcript contents: errors and
//! reports carry ids, hashes, offsets and counts only.
//!
//! Design: `docs/design/home/` in the lys repository (brief HOME-001).

pub mod cli;
pub mod error;
pub mod harness;
pub mod record;

pub use error::HomeError;
pub use record::blocks::{BlockStore, Hash, Put};
pub use record::call::{Api, CallMeta, CallRecord, CallStatus, IngestReport, OutcomeMeta};
pub use record::canon::{AddReport, Canon, Inherited};
pub use record::entries::{Entry, EntryBase, EntryBody, SessionHeader};
pub use record::epilogue::{Added, add_epilogue};
pub use record::lantern::{Lit, light};
pub use record::reader::SessionReader;
pub use record::recall::{
    Epilogue, LanternRow, RecallReport, Skipped, recall_by_note, recall_by_point,
};
pub use record::{Home, Session};
