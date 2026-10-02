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
mod agent_grants_api;
pub mod agent_pass;
mod agent_pass_recovery;
pub mod agent_pass_store;

pub mod agent_policy_api;
pub mod agent_policy_store;
mod agent_roots;
mod agent_sight;
pub mod agent_signature;
pub mod apps_api;
pub mod apps_bench;
pub mod apps_bench_scratch;
pub mod apps_binding;
mod apps_credentials;
pub mod apps_error;
mod apps_refresh;
pub mod apps_schema_api;
pub mod apps_state;
pub mod apps_store;
mod apps_upgrade;
pub mod apps_views;
pub mod budgets_act;
pub mod budgets_api;
pub mod budgets_context;
pub mod budgets_crossing;
pub mod budgets_enforce;
pub mod budgets_feed;
mod budgets_giving;
pub mod budgets_holding;
#[cfg(test)]
mod budgets_holding_tests;
pub mod budgets_index;
pub mod budgets_legacy;
pub mod budgets_limits;
pub mod budgets_members;
pub mod budgets_migration;
pub mod budgets_state;
pub mod budgets_store;
mod budgets_totals;
pub mod budgets_usage;
#[cfg(test)]
mod budgets_work;
pub mod caller_admission;
mod certificate_keys;
pub mod certificates_api;
mod certificates_issue;
pub mod certificates_store;
mod changes;
pub mod config;
pub mod configuration_api;
pub mod configuration_store;
pub mod connections_api;
pub mod dev_seed;
pub mod directory_views;
pub mod drafts_api;
#[cfg(test)]
mod drafts_api_tests;
pub mod error;
pub mod error_budget;
pub mod error_holding;
pub mod error_machine;
mod error_names;
#[cfg(test)]
mod error_names_tests;
mod error_status;
pub mod error_team;
mod estate_api;
pub mod file_stores;
#[cfg(test)]
mod folded_work;
pub mod goals_api;
pub mod goals_edit;
pub mod goals_state;
pub mod goals_store;
pub mod goals_types;
pub mod goals_views;
pub mod grant_contract;
mod grant_sight;
mod grant_token_store;
#[cfg(test)]
mod grant_token_tests;
pub mod grant_tokens;
pub mod grants;
pub mod grants_batch;
pub mod grants_reach;
pub mod grants_refusals;
pub mod harness_catalogue;
pub mod health_api;
mod import_api;
mod import_bootstrap;
mod import_bootstrap_state;
mod kept_responsibilities;
pub mod launch_api;
pub mod launch_fields;
pub mod launch_harness;
pub mod launch_permissions;
mod launch_record_config;
pub mod launch_template;
pub mod link_audit_api;
pub mod list_page;
mod mcp_approval_sight;
mod mcp_callers;
mod mcp_endpoint;
mod mcp_oauth;
mod mcp_oauth_grants;
mod mcp_oauth_store;
mod mcp_receipts;
mod mcp_record;
pub mod mcp_requests_api;
mod mcp_requests_state;
pub mod mcp_requests_store;
mod mcp_tools;
pub mod memory_api;
pub mod message_edges;
pub mod network_api;
pub mod network_store;
pub mod oidc;
pub mod openapi;
mod openapi_accounts_types;
mod openapi_goals_types;
mod openapi_refusals;
mod openapi_runner_types;
mod openapi_table;
mod openapi_typed;
mod openapi_types;
pub mod operator;
#[cfg(test)]
mod pass_provenance_tests;
pub mod provider;
pub mod provisioning_api;
pub mod provisioning_store;
pub mod read_api;
pub mod read_views;
pub mod receipts_api;
pub mod refusals_api;
pub mod refusals_follow;
pub mod refusals_store;
mod reporting_api;
mod reporting_read;
pub mod reporting_views;
pub mod requests_api;
pub mod requests_decide;
mod requests_state;
pub mod requests_store;
pub mod requests_views;
pub mod resources_api;
pub mod restart_api;
pub mod reviews_api;
pub mod reviews_state;
pub mod reviews_store;
pub mod roles_api;
#[cfg(test)]
mod roles_pass_tests;
pub mod roles_records;
pub mod roles_store;
pub mod roles_views;
mod route_actions;
#[cfg(test)]
mod route_actions_tests;
#[cfg(test)]
mod route_group_scope_tests;
#[cfg(test)]
mod route_named_scope_tests;
pub mod routes;
mod routes_startup;
pub mod routes_table;
pub mod runner_acts;
pub mod runner_api;
mod runner_bytes_api;
pub mod runner_client;
mod runner_dial;
pub mod runner_operate;
pub mod runner_sessions;
mod runner_start_pass;
pub mod runtime_api;
pub mod runtime_state;
pub mod runtime_store;
pub mod secrets_api;
pub mod secrets_sign;
mod service_account_grants;
pub mod service_accounts_api;
pub mod service_accounts_state;
pub mod service_accounts_store;
pub mod session;
pub mod session_admission;
pub mod session_store;
pub mod sessions_api;
pub mod setup;
pub mod sign_in;
mod sign_in_callback;
pub mod sign_in_providers;
mod signed_first;
pub mod skills_api;
pub mod spicedb;
mod spicedb_apps;
mod spicedb_http;
mod start_budget;
pub mod start_checks;
pub mod stop_api;
pub mod stops_state;
pub mod stops_store;
pub mod surface;
pub mod teams_api;
pub mod teams_migration;
pub mod teams_nesting;
pub mod teams_state;
pub mod teams_store;
pub mod tree_api;
pub mod tree_views;

pub use config::Config;
mod signed_json;
pub use error::ServerError;
pub use routes::{AppState, Say, router, service, service_saying};

mod provider_browser;

mod sign_in_flights;

mod sign_in_attempts;

mod sign_in_address;

#[cfg(test)]
mod agent_pass_tests;

/// Eligible authorities named by an agent grant refusal.
pub mod who_can_grant;
