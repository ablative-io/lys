//! The lys directory service.
//!
//! People sign in on Lys's own pages; the service carries the sign-in to the
//! configured OIDC issuer server-side, and a browser never reaches the
//! issuer. Every mutation is
//! admitted only for the configured administrator, and every other caller is
//! refused by name (P9). The directory itself is crates/lys-identity: this
//! crate holds sign-in, sessions, admission and the HTTP routes over it.

pub mod accounts;
pub mod admission;
mod agent_sight;
pub mod agent_signature;
pub mod apps_api;
pub mod apps_bench;
pub mod apps_bench_scratch;
pub mod apps_binding;
pub mod apps_error;
pub mod apps_schema_api;
pub mod apps_state;
pub mod apps_store;
pub mod apps_views;
pub mod certificates_api;
mod certificates_issue;
pub mod certificates_store;
pub mod config;
pub mod configuration_api;
pub mod connections_api;
pub mod dev_seed;
pub mod directory_views;
pub mod error;
mod error_status;
pub mod file_stores;
pub mod grant_contract;
mod grant_sight;
pub mod grants;
pub mod grants_batch;
pub mod launch_api;
pub mod launch_harness;
pub mod launch_template;
pub mod link_audit_api;
mod mcp_record;
pub mod memory_api;
pub mod network_api;
pub mod network_store;
pub mod oidc;
pub mod openapi;
mod openapi_refusals;
mod openapi_runner_types;
mod openapi_table;
mod openapi_typed;
mod openapi_types;
pub mod operator;
pub mod provider;
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
pub mod reviews_state;
pub mod reviews_store;
pub mod roles_api;
pub mod roles_records;
pub mod roles_store;
pub mod roles_views;
pub mod routes;
pub mod runner_acts;
pub mod runner_api;
pub mod runner_client;
mod runner_dial;
pub mod runner_sessions;
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
pub mod sign_in;
pub mod sign_in_providers;
pub mod spicedb;
mod spicedb_apps;
mod spicedb_http;
pub mod stop_api;
pub mod stops_state;
pub mod stops_store;
pub mod surface;
pub mod teams_api;
pub mod teams_state;
pub mod teams_store;

pub use config::Config;
pub use error::ServerError;
pub use routes::{AppState, Say, router, service, service_saying};
