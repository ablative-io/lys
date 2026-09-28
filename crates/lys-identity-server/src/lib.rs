//! The lys directory service.
//!
//! People sign in through the configured OIDC issuer. Every mutation is
//! admitted only for the configured administrator, and every other caller is
//! refused by name (P9). The directory itself is crates/lys-identity: this
//! crate holds sign-in, sessions, admission and the HTTP routes over it.

pub mod admission;
pub mod certificates_store;
pub mod config;
pub mod connections_api;
pub mod dev_seed;
pub mod error;
pub mod grant_contract;
mod grant_sight;
pub mod grants;
pub mod launch_api;
pub mod launch_template;
pub mod link_audit_api;
pub mod network_api;
pub mod network_store;
pub mod oidc;
pub mod provisioning_api;
pub mod provisioning_store;
pub mod read_api;
pub mod read_views;
pub mod receipts_api;
pub mod requests_api;
pub mod requests_decide;
mod requests_state;
pub mod requests_store;
pub mod requests_views;
pub mod resources_api;
pub mod reviews_api;
pub mod roles_api;
pub mod roles_records;
pub mod roles_store;
pub mod roles_views;
pub mod routes;
pub mod runtime_api;
pub mod runtime_state;
pub mod runtime_store;
pub mod secrets_api;
pub mod secrets_sign;
pub mod service_accounts_api;
pub mod service_accounts_state;
pub mod service_accounts_store;
pub mod session;
pub mod sessions_api;
pub mod setup;
pub mod spicedb;
mod spicedb_http;

pub use config::Config;
pub use error::ServerError;
pub use routes::{AppState, Say, router, service, service_saying};
