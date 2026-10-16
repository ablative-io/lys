//! OAuth discovery describes the same native and registered client flow.
use super::{Apps, native};
use axum::{Json, extract::State};
use serde_json::json;
use std::sync::Arc;

pub(super) async fn resource(State(apps): State<Arc<Apps>>) -> Json<serde_json::Value> {
    Json(json!({
        "resource": apps.resource,
        "authorization_servers": [apps.origin],
        "bearer_methods_supported": ["header"],
        "resource_name": "Lys",
    }))
}

pub(super) async fn server(State(apps): State<Arc<Apps>>) -> Json<serde_json::Value> {
    let origin = &apps.origin;
    Json(json!({
        "issuer": origin,
        "authorization_endpoint": format!("{origin}/oauth/mcp/authorize"),
        "token_endpoint": format!("{origin}/oauth/mcp/token"),
        "registration_endpoint": format!("{origin}/oauth/mcp/register"),
        "response_types_supported": ["code"],
        "grant_types_supported": ["authorization_code", "refresh_token"],
        "code_challenge_methods_supported": ["S256"],
        "token_endpoint_auth_methods_supported": ["none"],
        "native_client_id": native::CLIENT,
    }))
}
