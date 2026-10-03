//! The little proxy (HOME-001 R10): forwards each call to the provider its
//! path names, with headers and streamed body unchanged, and records one
//! `lys.call` entry (R6) per model call under the session the call's own key
//! names, or under the day's `unlinked` session when it carries none.
//!
//! Invariants, each held in the file that owns it:
//! - Forwarding changes no byte and buffers nothing; only `host` is set to
//!   the upstream's authority; one transport, never sending a request twice
//!   ([`forward`]).
//! - A call is linked only by the key in its own body, read as it passes
//!   with bounded memory ([`link`]).
//! - Every call is captured; only a call that ended whole is `complete`
//!   ([`capture`]).
//! - A call is journalled durably before it is sent, and a call a previous
//!   run left open is recorded `lost` once ([`journal`]).
//! - Event streams are read by grammar as they pass ([`stream`]).
//! - No header value and no body byte is logged, and none is stored outside
//!   the spool and the home's block store ([`error`] names paths and ids).

pub mod capture;
#[cfg(test)]
mod capture_decode_tests;
mod decode;
pub mod error;
pub mod forward;
#[cfg(test)]
mod forward_tests;
pub mod journal;
#[cfg(test)]
mod journal_tests;
pub mod link;
#[cfg(test)]
mod link_tests;
pub mod stream;
pub mod stream_chat;
pub mod stream_messages;
pub mod stream_responses;
pub mod stream_sse;
#[cfg(test)]
mod stream_tests;

#[cfg(test)]
mod timing;
