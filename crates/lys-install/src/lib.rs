//! The standalone identity product's install as a library: prepare,
//! configure, install, upgrade and check the maintained Rauthy, `SpiceDB`
//! and one `PostgreSQL` database, the secrets broker and the directory
//! service.
//!
//! `lys identity install` and Lys.app both run this code, the CLI as a thin
//! call and the app in its own process, so there is one install and never a
//! second one written for the app.
//!
//! Invariants: nothing is written outside the data root [`install::layout`]
//! chooses; every wait is on an event, never a clock or a question asked
//! again on a schedule; every failure is an [`IdentityError`] naming its
//! kind, operation and resource, and none carries a credential's bytes. The
//! install reports each [`steps::Step`] as it begins, so a caller can show
//! it as it happens.

pub mod config;
pub mod configure;
pub mod credentials;
pub mod error;
pub mod health;
pub mod install;
pub mod loopback_http;
pub mod output;
pub mod prepare;
pub mod private_files;
pub mod rauthy;
pub mod steps;
pub mod themes;
pub mod upgrade;

pub use error::IdentityError;
