//! The directory route table; handlers and startup state live in their owning modules.

use crate::routes::{
    Shared, bind_login, change_profile, list, read, register_agent, register_person, transition,
};
use axum::{
    Router,
    routing::{get, post},
};

/// The service's routes over `state`.
pub fn router(state: Shared) -> Router {
    Router::new()
        .route("/callback", get(crate::sign_in_callback::callback))
        .route("/people", post(register_person))
        .route("/agents", post(register_agent))
        .route(
            "/agents/{id}/reports-to",
            post(crate::reporting_api::change),
        )
        .route("/identity/import", post(crate::import_api::import))
        .route("/identity/estate-plan", get(crate::estate_api::plan))
        .route("/identity/estate-apply", post(crate::estate_api::apply))
        .route("/identities", get(list))
        .route("/identities/{id}", get(read))
        .route("/identities/{id}/profile", post(change_profile))
        .route("/identities/{id}/transitions", post(transition))
        .route("/people/{id}/logins", post(bind_login))
        .merge(crate::sign_in::routes())
        .merge(crate::setup::routes())
        .merge(crate::accounts::routes())
        .merge(crate::read_api::routes())
        .merge(crate::grants::routes())
        .merge(crate::grant_tokens::routes())
        .merge(crate::receipts_api::routes())
        .merge(crate::reviews_api::routes())
        .merge(crate::roles_api::routes())
        .merge(crate::requests_api::routes())
        .merge(crate::connections_api::routes())
        .merge(crate::sign_in_providers::routes())
        .merge(crate::link_audit_api::routes())
        .merge(crate::network_api::routes())
        .merge(crate::provisioning_api::routes())
        .merge(crate::mcp_requests_api::routes())
        .merge(crate::harness_catalogue::routes())
        .merge(crate::restart_api::routes())
        .merge(crate::runtime_api::routes())
        .merge(crate::runner_api::routes())
        .merge(crate::stop_api::routes())
        .merge(crate::budgets_api::routes())
        .merge(crate::budgets_act::routes())
        .merge(crate::agent_policy_api::routes())
        .merge(crate::refusals_api::routes())
        .merge(crate::goals_api::routes())
        .merge(crate::service_accounts_api::routes())
        .merge(crate::teams_api::routes())
        .merge(crate::tree_api::routes())
        .merge(crate::resources_api::routes())
        .merge(crate::secrets_api::routes())
        .merge(crate::sessions_api::routes())
        .merge(crate::apps_api::routes())
        .merge(crate::apps_schema_api::routes())
        .merge(crate::apps_bench::routes())
        .merge(crate::openapi::routes())
        .merge(crate::changes::routes())
        .layer(axum::middleware::from_fn_with_state(
            std::sync::Arc::clone(&state),
            crate::changes::observe,
        ))
        .with_state(state)
}
