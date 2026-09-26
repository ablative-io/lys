//! `lys identity`: prepare, configure and check the development deployment
//! of the standalone identity product — the maintained Rauthy, `SpiceDB` and
//! one `PostgreSQL` database (deploy/identity/).
//!
//! Module manifest (IDENTITY-001 revision 5, row 02): [`cli`] the arguments;
//! [`config`] the typed deployment config and its validation;
//! [`credentials`] redacted, zeroized credential material and its reuse;
//! [`private_files`] owner-only durable files; [`prepare`], [`configure`] and
//! [`health`] the three subcommands; [`rauthy`] the typed Rauthy API;
//! [`themes`] the estate palette mapping; [`error`] the named failures.
//!
//! # Invariants
//!
//! - No secret value reaches output, a log or an error: credentials format
//!   as their names, and every error names a credential, never its value.
//! - `SpiceDB` is configured and health-checked here and nothing else: this
//!   module asks it for no permission decision and writes it no
//!   relationship or schema (DIRECTORY-002 R4).

pub mod cli;
pub mod config;
pub mod configure;
pub mod credentials;
pub mod error;
pub mod health;
pub mod prepare;
pub mod private_files;
pub mod rauthy;
pub mod themes;
