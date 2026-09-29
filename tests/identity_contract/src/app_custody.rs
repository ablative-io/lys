//! A named custody stand-in for app contract tests. Real broker sealing and
//! signed requests are covered by the broker and secrets API test suites.
use axum::{Json, http::HeaderMap};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::error::Error;

/// Fixed fixture bytes, never used by a production service.
pub fn secret() -> String {
    "ab".repeat(32)
}
/// The app credential held by this stand-in, not returned by approval.
pub fn credential(app: &str) -> String {
    format!("lys-app.{app}.{}", secret())
}
/// Start this test's isolated broker endpoint.
///
/// # Errors
/// Returns listener binding errors.
pub async fn start() -> Result<String, Box<dyn Error>> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let base = format!("http://{}", listener.local_addr()?);
    let router = axum::Router::new().route("/_lys/apps/prepare", axum::routing::post(prepare));
    tokio::spawn(async move { axum::serve(listener, router).await });
    Ok(base)
}
async fn prepare(headers: HeaderMap, Json(body): Json<Value>) -> Json<Value> {
    let owner = headers
        .get("lys-on-behalf-of")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("missing");
    let app = body["app"].as_str().unwrap_or("missing");
    let prefix = format!("lys-app-{owner}-{app}");
    Json(
        json!({"app": app, "client_secret_ref": format!("{prefix}-client"),
        "api_credential_ref": format!("{prefix}-api"),
        "client_secret_sha256": format!("{:x}", Sha256::digest(secret().as_bytes()))}),
    )
}
