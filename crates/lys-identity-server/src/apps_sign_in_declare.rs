//! One return address declared for an approved app (DIRECTORY-098): by the
//! administrator, or by the app itself acting through its own credential, at
//! `POST /apps/{app}/sign_in/returns`. The address is judged exactly as a
//! saved line is (`apps_api::redirect`, DIRECTORY-097's rule and reasons);
//! an address already listed answers the settings unchanged and writes
//! nothing; otherwise one line records the declaration as the caller's act
//! and the settings in force gain the address. A person, the registrar or
//! another app's credential is refused `NotAdmitted` by name; an app that is
//! not approved is refused by its standing's name, and an approved app with
//! no sign-in settings to join is refused as an app that could sign nobody
//! in. The administrator's `POST /apps/{app}/sign_in` still sets the whole
//! list and may drop a declared address.

use std::str::FromStr;
use std::sync::Arc;

use axum::Json;
use axum::Router;
use axum::extract::rejection::JsonRejection;
use axum::extract::{Path as UrlPath, State};
use axum::http::HeaderMap;
use axum::routing::post;
use lys_identity::OperationId;
use serde::Deserialize;

use crate::apps_api::{malformed, view, with_apps};
use crate::apps_binding::{Acting, acting};
use crate::apps_error::AppError;
use crate::apps_state::{Line, ReturnDeclared, Standing};
use crate::apps_views::AppView;
use crate::error::ServerError;
use crate::routes::AppState;
use crate::session::now;

/// One return address declared for an app.
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReturnDeclarationBody {
    pub(crate) operation: String,
    /// The address the sign-in client may send a person back to, exactly.
    pub(crate) address: String,
}

/// The route, merged beside the apps' others.
pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/apps/{app}/sign_in/returns", post(declare))
}

/// `POST /apps/{app}/sign_in/returns`: declare one return address for an
/// approved app, as the administrator or as the app itself. Answers the app
/// as it now stands.
pub(crate) async fn declare(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    UrlPath(id): UrlPath<String>,
    body: Result<Json<ReturnDeclarationBody>, JsonRejection>,
) -> Result<Json<AppView>, ServerError> {
    let Json(body) = body.map_err(|refused| malformed(refused.body_text()))?;
    let operation = OperationId::from_str(&body.operation)?.to_string();
    with_apps(&state, |apps, projection| {
        let who = acting(&state, apps.held(), &headers, projection)?;
        let by = match &who {
            Acting::Administrator(_) => who.by(),
            Acting::App { app, .. } if *app == id => who.by(),
            _ => {
                return Err(ServerError::NotAdmitted {
                    reason: "only the administrator or the app itself declares the app's return addresses",
                });
            }
        };
        if let Some(kept) = apps.held().operation(&operation) {
            return match kept {
                Line::ReturnDeclared(declared) if declared.app == id => view(apps, &id),
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
        let Some(settings) = &app.sign_in else {
            return Err(AppError::NoRedirect { app: id.clone() }.into());
        };
        let address = crate::apps_api::redirect(&body.address)?;
        if settings.redirects.contains(&address) {
            return view(apps, &id);
        }
        apps.keep(Line::ReturnDeclared(ReturnDeclared {
            operation,
            app: id.clone(),
            address,
            by,
            at: now(),
        }))?;
        view(apps, &id)
    })
    .map(Json)
}

#[cfg(test)]
#[path = "apps_sign_in_declare_tests.rs"]
mod tests;
