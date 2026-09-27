//! The lys directory service.
//!
//! People sign in through the configured OIDC issuer. Every mutation is
//! admitted only for the configured administrator, and every other caller is
//! refused by name (P9). The directory itself is crates/lys-identity: this
//! crate holds sign-in, sessions, admission and the HTTP routes over it.

pub mod admission;
pub mod config;
pub mod error;
pub mod link_audit_api;
pub mod oidc;
pub mod receipts_api;
pub mod routes;
pub mod session;

pub use config::Config;
pub use error::ServerError;
pub use routes::{AppState, router};
