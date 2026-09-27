//! The home and its sessions.
//!
//! A [`Home`] is a directory: `sessions/<id>.jsonl` files in Pi's grammar,
//! each with its index and head beside it, and `blocks/` for content by hash.
//! A [`Session`] is one open session file: append a child of the head, move
//! the head, read the path from the head to the root by seeking, and build the
//! context path the way Pi's `buildSessionContext` does.
//!
//! One owner at a time: opening or creating a session takes an exclusive lock
//! on `<id>.lock` beside the file (held by the process for as long as the
//! [`Session`] lives), so two owners in one process or two processes cannot
//! both append with their own idea of where the file ends. A second opener is
//! refused by name ([`HomeError::SessionHeld`](crate::error::HomeError::SessionHeld)).
//!
//! An append is durable in three steps: the entry line, then its index row,
//! then the head. When a later step fails after the line is durable, the
//! session reconciles itself from the file before it admits anything else
//! (the index is rebuilt by scanning, the head re-read), so what the process
//! believes about the file never runs ahead of or behind the disk.

pub mod beside;
#[cfg(test)]
mod beside_tests;
pub mod blocks;
#[cfg(test)]
mod blocks_tests;
pub mod call;
#[cfg(test)]
mod call_tests;
pub mod canon;
#[cfg(test)]
mod canon_tests;
pub mod entries;
pub mod epilogue;
#[cfg(test)]
mod epilogue_tests;
pub mod fork;
pub mod fork_cut;
#[cfg(test)]
pub(crate) mod fork_cut_tests;
pub mod fork_report;
#[cfg(test)]
pub(crate) mod fork_tests;
pub mod given;
pub mod given_statement;
#[cfg(test)]
mod given_statement_tests;
#[cfg(test)]
pub(crate) mod given_tests;
mod helpers;
mod home;
pub mod index;
pub mod lantern;
#[cfg(test)]
mod lantern_tests;
pub mod reader;
#[cfg(test)]
pub(crate) mod reader_tests;
pub mod recall;
#[cfg(test)]
mod recall_tests;
#[cfg(test)]
mod record_tests;
mod session;
pub mod templates;
#[cfg(test)]
mod templates_tests;

pub use helpers::{MAX_NAME_BYTES, PI_FORMAT_VERSION, fresh_id, json_len, now, safe_component};
pub use home::Home;
pub use session::Session;
