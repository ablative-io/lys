//! A boot refused by a lost whole store is never Lys down in silence
//! (DIRECTORY-107 R2): the service binds its listener and serves exactly
//! two things, `GET /health`, answering `refused_to_start` with every store
//! found short, what its receipt holds and what was found, the one command
//! that serves what was found and the computer to run it on, and the
//! screens, whose sign-in page reads it and shows those words in place of
//! the form. Every other route answers 503 with the same words.

use std::sync::Arc;

use axum::Json;
use axum::extract::OriginalUri;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use lys_identity::stores_receipt::Loss;
use serde_json::{Value, json};

use crate::config::Config;
use crate::error::ServerError;
use crate::health_api::SERVICE;
use crate::stores_receipt::Started;

/// Why the boot was refused: every whole store found short.
#[derive(Debug, Clone)]
pub struct Refusal {
    /// The stores, each with what its receipt holds and what was found.
    pub stores: Vec<Loss>,
    /// This computer's name.
    pub computer: String,
}

impl Refusal {
    /// The one command that serves what was found, naming every store.
    #[must_use]
    pub fn command(&self) -> String {
        let mut command = "lys identity accept-loss".to_owned();
        for loss in &self.stores {
            command.push_str(" --store ");
            command.push_str(&loss.store);
        }
        command
    }

    /// The refusal in words: every store, both heads, the command and the
    /// computer.
    #[must_use]
    pub fn words(&self) -> String {
        let stores = self
            .stores
            .iter()
            .map(|loss| {
                format!(
                    "{} ({}: the receipt holds {}; found {})",
                    loss.store, loss.refusal, loss.held, loss.found
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "Lys refused to start: {stores}. Nothing is served until this is settled. If what was found is all there is, run `{}` on {} to start Lys at what was found.",
            self.command(),
            self.computer
        )
    }

    fn answer(&self) -> Value {
        json!({
            "refusal": "refused_to_start",
            "reason": format!("refused_to_start: {}", self.words()),
            "stores": self.stores.iter().map(|loss| json!({
                "store": loss.store,
                "refusal": loss.refusal,
                "held": loss.held,
                "found": loss.found,
            })).collect::<Vec<_>>(),
            "command": self.command(),
            "computer": self.computer,
        })
    }
}

/// `GET /health` and every other API path while the boot is refused.
fn answered(refusal: &Refusal, OriginalUri(uri): OriginalUri) -> Response {
    let mut body = refusal.answer();
    let path = uri.path();
    if path == "/health" || path == "/api/health" {
        body["service"] = json!(SERVICE);
        body["version"] = json!(env!("CARGO_PKG_VERSION"));
    }
    (StatusCode::SERVICE_UNAVAILABLE, Json(body)).into_response()
}

/// The service a refused boot serves: the screens, when the configuration
/// names them, and every API path answered with the refusal.
///
/// # Errors
/// The screens unreadable, by name.
pub fn started(config: &Config, refusal: Refusal) -> Result<Started, ServerError> {
    let line = format!("refused_to_start: {}", refusal.words());
    let refusal = Arc::new(refusal);
    let api = axum::Router::new().fallback(move |uri: OriginalUri| {
        let refusal = Arc::clone(&refusal);
        async move { answered(&refusal, uri) }
    });
    let router = match &config.surface_dir {
        Some(dir) => crate::surface::serving(dir, api)?,
        None => api,
    };
    Ok(Started {
        router,
        at_stop: None,
        refused: Some(line),
    })
}

/// The service a boot refused by `stores` serves.
///
/// # Errors
/// The screens unreadable, by name.
pub fn refused(config: &Config, stores: Vec<Loss>) -> Result<Started, ServerError> {
    let computer = crate::stores_receipt::computer();
    started(config, Refusal { stores, computer })
}
