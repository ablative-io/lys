//! The lys runner: each started agent in its own background
//! pseudo-terminal, driven over a published protocol.
//!
//! A runner holds sessions. Each session is one process in its own
//! pseudo-terminal, with a scrollback of a configured size in bytes; it runs
//! whether or not any screen is open, and its exit status and the instant it
//! was seen are recorded. The runner answers on one Unix socket, mode 0600,
//! and on no network address; it acts only on requests its server's key
//! signed for this runner and for the connection they arrive on, so each
//! is answered once, by the one runner it was made for ([`protocol`]). A
//! runner on another machine is reached by a bridge that dials the server
//! with the machine's own key, over TLS ([`dial`]); the server never dials
//! it.
//!
//! Invariants:
//!
//! - Only this crate spawns a process for a session; the server asks for one
//!   over the protocol and spawns nothing.
//! - A session's end is recorded only when its exit is seen. A session lost
//!   with a runner that stopped without ending it is reported
//!   `ended_by_runner_restart` by the next runner, at the instant it found
//!   it gone, with no exit status: none is invented. Anything its process
//!   group left running is ended first, and the end names that signal.
//! - One runner holds a state directory at a time.
//! - No wait ends on a clock: a wait ends when what it waits for happens,
//!   when the session ends, or when the caller closes its request.
//! - No account value, key or token passes through the runner: a rotation
//!   names account handles, which the secrets broker swaps on the way out.
//!
//! The protocol is described for any tool that would be a machine's runner
//! in [`protocol`], and the conformance suite in `tests/conformance.rs` runs
//! against any runner given its socket.

pub mod admitted;
pub mod claude_judge;
pub mod client;
pub mod codex_judge;
pub mod codex_judge_client;
pub mod codex_policy;
pub mod codex_policy_contract;
pub mod codex_policy_readback;
pub mod codex_refusals;
pub mod collector;
pub mod containment_descriptors;
pub mod containment_entry;
pub mod containment_inputs;
pub mod containment_macos;
pub mod containment_paths;
pub mod containment_policy;
pub mod containment_stdio;
pub mod dial;
mod durable;
pub mod error;
pub mod folders;
pub mod injection;
mod input;
pub mod judge;
pub mod launch_config;
pub mod legacy_input;
pub mod operations;
pub mod peer;
pub mod protocol;
mod protocol_key;
mod protocol_request;
mod protocol_stop;
pub mod pty;
mod pty_command;
#[cfg(test)]
mod pty_command_tests;
pub mod published;
pub mod refusal_log;
pub mod refusals;
pub mod rotation;
pub mod scrollback;
pub mod session;
pub mod socket;
pub mod state;
pub mod terminal_bytes;
pub mod tracking;
pub mod tracking_budget;
mod tracking_fields;
pub mod tracking_proxy;
pub mod tracking_store;
pub mod trust;
#[cfg(test)]
mod trust_tests;

pub use client::{Client, Closer, Connection, GrantChannel, connect};
pub use error::RunnerError;
pub use protocol::{Act, Answer, Ended, EndedHow, Key, Launch, PROTOCOL_VERSION, Stopped};
pub use rotation::{Limit, Rotation};
pub use session::Sessions;
pub use socket::{Options, Runner, Serving};
