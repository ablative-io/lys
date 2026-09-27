//! `lys identity`: prepare, configure and check the development install of
//! the standalone identity product's three dependency processes, Rauthy,
//! `SpiceDB` and one `PostgreSQL` database (IDENTITY-001 row 02).
//!
//! The module manifest is exact: each file below carries one responsibility,
//! and unit tests sit in sibling `*_tests.rs` files. `SpiceDB` appears here only
//! in its configuration and in `health`'s readiness check: in step 1 nothing in
//! this module asks it for a permission decision or writes to it.

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

pub use cli::IdentityCommand;
pub use error::IdentityError;
