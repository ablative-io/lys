//! Subcommand implementations for the `lys-anchor` CLI.
//!
//! [`anchor`] implements the subcommands; shared plumbing lives in [`error`],
//! [`files`], [`hex`] and [`output`]. Per the repo standards, this file carries
//! declarations only.

pub mod anchor;
pub mod error;
pub mod files;
pub mod hex;
pub mod output;
