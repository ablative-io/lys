//! Subcommand implementations for the `lys` CLI.
//!
//! Each named module implements one subcommand family; shared plumbing lives
//! in [`duration`], [`error`], [`files`], [`hex`], and [`output`]. Per the repo standards, this file
//! carries declarations only.

pub mod attest;
pub mod ca;
pub mod duration;
pub mod error;
pub mod files;
pub mod hex;
pub mod inspect;
pub mod key;
pub mod log;
pub mod output;
pub mod pem;
pub mod seal;
pub mod verify;
