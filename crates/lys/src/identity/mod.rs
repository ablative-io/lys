//! `lys identity`: prepare, configure and check the standalone identity
//! product's dependencies — the maintained Rauthy, `SpiceDB` and one
//! `PostgreSQL` database. Declarations and re-exports only.

pub mod cli;
pub mod config;
pub mod configure;
pub mod credentials;
pub mod error;
pub mod health;
pub mod install;
pub mod loopback_http;
pub mod prepare;
pub mod private_files;
pub mod rauthy;
pub mod themes;

pub use cli::IdentityCommand;
pub use error::IdentityError;
