//! An approved app's sign-in settings, set by an administrator: the exact
//! addresses its client may send a person back to, and whether it is given
//! the person's name (the profile scope). They are set with the approval and
//! changed here, at `POST /apps/{app}/sign_in`; each change is a line of the
//! apps' record, and the latest is what the provider reads at its next
//! request, so a change is in force with no restart and nothing cached.
//!
//! Each address is judged by the rule a registration's addresses are judged
//! by, and a list with no address is refused: an app with no return address
//! can sign nobody in. An app that is not approved is refused by its
//! standing's name, and a caller who is not the administrator as the other
//! decisions on the apps refuse. The same settings sent again under their
//! operation answer the same and keep nothing new.

use std::str::FromStr;
use std::sync::Arc;

use axum::Json;
use axum::extract::rejection::JsonRejection;
use axum::extract::{Path as UrlPath, State};
use axum::http::HeaderMap;
use lys_identity::OperationId;
use serde::Deserialize;

use crate::apps_api::{administrator, malformed, view, with_apps};
use crate::apps_error::AppError;
use crate::apps_state::{Line, SignInSet, Standing};
use crate::apps_views::AppView;
use crate::error::ServerError;
use crate::routes::AppState;
use crate::session::now;

/// The sign-in settings an administrator sets for an approved app.
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct AppSignInBody {
    operation: String,
    /// The addresses the sign-in client may send a person back to, exactly.
    redirects: Vec<String>,
    /// Whether the app is given the person's name (the profile scope).
    profile: bool,
}

/// The return addresses `redirects` as they are kept for `app`: each judged by
/// the rule a registration's addresses are judged by, refused
/// `redirect_invalid` naming the address, and a list with no address refused
/// `redirect_invalid` saying the app could sign nobody in.
pub(crate) fn redirects(app: &str, redirects: &[String]) -> Result<Vec<String>, AppError> {
    if redirects.is_empty() {
        return Err(AppError::NoRedirect {
            app: app.to_owned(),
        });
    }
    redirects
        .iter()
        .map(|address| crate::apps_api::redirect(address))
        .collect()
}

/// `POST /apps/{app}/sign_in`: set an approved app's sign-in settings, as the
/// administrator. Answers the app as it now stands.
pub(crate) async fn set(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    UrlPath(id): UrlPath<String>,
    body: Result<Json<AppSignInBody>, JsonRejection>,
) -> Result<Json<AppView>, ServerError> {
    let Json(body) = body.map_err(|refused| malformed(refused.body_text()))?;
    let operation = OperationId::from_str(&body.operation)?.to_string();
    with_apps(&state, |apps, projection| {
        let by = administrator(&state, apps, &headers, projection)?;
        if let Some(kept) = apps.held().operation(&operation) {
            return match kept {
                Line::SignInSet(set) if set.app == id => view(apps, &id),
                _ => Err(AppError::AppOperationReused { operation }.into()),
            };
        }
        let app = apps
            .app(&id)
            .ok_or_else(|| AppError::AppUnknown { app: id.clone() })?;
        match app.standing() {
            Standing::Approved => {}
            Standing::Pending | Standing::Declined => {
                return Err(AppError::AppNotApproved { app: id.clone() }.into());
            }
            Standing::Retired => return Err(AppError::AppRetired { app: id.clone() }.into()),
        }
        let redirects = redirects(&id, &body.redirects)?;
        apps.keep(Line::SignInSet(SignInSet {
            operation,
            app: id.clone(),
            redirects,
            profile: body.profile,
            by,
            at: now(),
        }))?;
        view(apps, &id)
    })
    .map(Json)
}

#[cfg(test)]
#[path = "apps_sign_in_tests.rs"]
mod tests;
