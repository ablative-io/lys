//! The app reads: `GET /apps`, every app the caller may see with Lys's own
//! public address beside them, and `GET /apps/{app}`, one of them.
//!
//! The public address is the one an app on another computer calls Lys back
//! at, computed as `network_join::dial_address` computes it for a runner from
//! Lys's registered address (`redirect_url`); when that address is on Lys's
//! computer alone or plain http there is none, and the answer says why, so
//! the approve page hands the app no address it cannot dial without saying
//! so (DIRECTORY-102 R2).

use std::net::IpAddr;
use std::sync::Arc;

use axum::Json;
use axum::extract::{Path as UrlPath, State};
use axum::http::HeaderMap;

use super::with_apps;
use crate::apps_binding::acting;
use crate::apps_error::AppError;
use crate::apps_read::{AppRead, AppsRead, read_view, sees};
use crate::error::ServerError;
use crate::routes::AppState;

pub(super) async fn list(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<AppsRead>, ServerError> {
    let (public_address, public_address_reason) = public_address(state.oidc.redirect_url());
    with_apps(&state, |apps, projection| {
        let who = acting(&state, apps.held(), &headers, projection)?;
        Ok(AppsRead {
            apps: apps
                .held()
                .apps
                .iter()
                .filter(|app| sees(&who, app))
                .map(|app| read_view(&who, app))
                .collect(),
            public_address,
            public_address_reason,
        })
    })
    .map(Json)
}

pub(super) async fn one(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    UrlPath(id): UrlPath<String>,
) -> Result<Json<AppRead>, ServerError> {
    with_apps(&state, |apps, projection| {
        let who = acting(&state, apps.held(), &headers, projection)?;
        apps.app(&id)
            .filter(|app| sees(&who, app))
            .map(|app| read_view(&who, app))
            .ok_or_else(|| AppError::AppUnknown { app: id.clone() }.into())
    })
    .map(Json)
}

/// Lys's public address from its registered address `redirect`, or none
/// with the reason in words an administrator connecting an app reads.
fn public_address(redirect: &str) -> (Option<String>, Option<String>) {
    match crate::network_join::dial_address(redirect) {
        Ok(address) => (Some(address), None),
        Err(_refused) => (None, Some(no_address(redirect))),
    }
}

/// Why `redirect` gives an app on another computer no address to call:
/// `dial_address`'s two refusals, said for an app rather than a runner.
fn no_address(redirect: &str) -> String {
    let Ok(url) = reqwest::Url::parse(redirect) else {
        return format!(
            "Lys's registered address, {redirect}, does not read as an address, so Lys has none an app can call it back at."
        );
    };
    let origin = url.origin().ascii_serialization();
    let host = url.host_str().unwrap_or_default();
    let bare = host.trim_start_matches('[').trim_end_matches(']');
    let only_here = bare.is_empty()
        || bare.eq_ignore_ascii_case("localhost")
        || bare
            .parse::<IpAddr>()
            .is_ok_and(|address| address.is_loopback());
    if only_here {
        format!(
            "Lys is served only at {origin}, an address only the computer it runs on answers, so an app on another computer cannot call Lys back there; Lys needs an https address other computers reach."
        )
    } else {
        format!(
            "Lys is served at {origin} over plain http, and an app on another computer is handed Lys's address only over https."
        )
    }
}

#[cfg(test)]
#[path = "apps_api_read_tests.rs"]
mod tests;
