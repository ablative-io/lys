//! Every route the broker serves, and the proxy for every other path.
//!
//! The screen routes answer a person a trusted screen service vouches for,
//! or a handle's holder (see `callers`). The routes that carry a value in
//! their body (adding a secret, replacing its value) admit only a person vouched for by a trusted screen service.

use std::sync::Arc;

use axum::Router;
use axum::routing::{get, post};

use crate::serve::Shared;

/// The broker's routes over `shared`.
pub fn routes(shared: Arc<Shared>) -> Router {
    Router::new()
        .route("/_lys/apps/prepare", post(crate::save_app::prepare))
        .route("/_lys/apps/client", post(crate::app_client::authenticate))
        .route("/_lys/apps/client/issue", post(crate::app_client::issue))
        .route("/_lys/apps/client/end", post(crate::app_client::end))
        .route("/_lys/secrets", get(crate::view::secrets))
        .route("/_lys/audit", get(crate::view::audit))
        .route("/_lys/grants", get(crate::view::grants))
        .route("/_lys/handles", get(crate::view::handles))
        .route("/_lys/next-account", post(crate::serve::next_account))
        .route("/_lys/sign/{secret}", post(crate::signing::sign))
        .route("/_lys/scope", post(crate::manage::scope))
        .route("/_lys/recipients", post(crate::manage::recipients))
        .route("/_lys/settings", get(crate::manage::settings))
        .route("/_lys/revocation", get(crate::manage::revocation))
        .route("/_lys/drop", post(crate::manage::drop_handle))
        .route("/_lys/leases/{lease_id}", get(crate::manage::lease))
        .route(
            "/_lys/leases/{lease_id}/revoke",
            post(crate::manage::revoke),
        )
        .route(
            "/_lys/leases/{lease_id}/relinquish",
            post(crate::manage::relinquish),
        )
        .route("/_lys/add", post(crate::values::add))
        .route("/_lys/replace", post(crate::values::replace))
        .route("/_lys/retire", post(crate::values::retire))
        .fallback(crate::serve::proxy)
        .with_state(shared)
}
