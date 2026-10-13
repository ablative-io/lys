//! Register an app under the current caller and its prepared edit grant.

use super::{
    Acting, AppError, AppSchema, AppState, AppView, Line, RegisterBody, Registered, ServerError,
    acting, app_id, malformed, now, redirect, service_account_held, view, words,
};
use axum::{
    Json,
    extract::{State, rejection::JsonRejection},
    http::HeaderMap,
};
use lys_identity::OperationId;
use std::str::FromStr;
use std::sync::Arc;

pub(crate) async fn register(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Result<Json<RegisterBody>, JsonRejection>,
) -> Result<Json<AppView>, ServerError> {
    let Json(body) = body.map_err(|refused| malformed(refused.body_text()))?;
    let operation = OperationId::from_str(&body.operation)?.to_string();
    crate::grants_reads::with_read_grants(&state, None, now(), |judged| {
        let who = acting(&state, judged.apps.held(), &headers, judged.directory)?;
        if !matches!(who, Acting::Registrar { .. }) { return Ok(Vec::new()); }
        let caller = crate::service_account_grants::caller(&state, &headers, judged)?;
        Ok(vec![lys_identity::grants::ExerciseRequest { caller, route: lys_identity::grants::Route::Api,
            resource: lys_identity::grants::Resource::new("directory", "apps")?, action: lys_identity::grants::Action::new("edit")? }])
    }, |mut judged| {
        let who = acting(&state, judged.apps.held(), &headers, judged.directory)?;
        if matches!(who, Acting::Registrar { .. }) {
            let caller = crate::service_account_grants::caller(&state, &headers, &judged)?;
            crate::service_account_grants::admit(&mut judged, caller, "apps")?;
        }
        let service_account = match &who {
            Acting::Administrator(_) => body.service_account.clone(),
            Acting::Registrar { service_account } => match &body.service_account {
                Some(named) if named != service_account => {
                    return Err(ServerError::NotAdmitted {
                        reason: "a registrar registers apps that act as itself",
                    });
                }
                _ => Some(service_account.clone()),
            },
            Acting::Person(_) | Acting::App { .. } => {
                return Err(ServerError::NotAdmitted {
                    reason: "only the administrator and a registrar service account register apps",
                });
            }
        };
        let id = app_id(&body.id).map_err(AppError::from)?.to_owned();
        if judged.apps.app(&id).is_some() && judged.apps.held().operation(&operation).is_none() {
            return Err(AppError::AppExists { app: id }.into());
        }
        let name = words("name", &body.name)?;
        if name.is_empty() {
            return Err(malformed("an app has a name"));
        }
        let redirects = body
            .redirects
            .iter()
            .map(|address| redirect(address))
            .collect::<Result<Vec<_>, _>>()?;
        let schema = AppSchema::parse(&id, &body.schema).map_err(AppError::from)?;
        if let Some(account) = &service_account {
            service_account_held(&state, account)?;
        }
        let line = Line::Registered(Registered {
            operation,
            app: id.clone(),
            name,
            redirects,
            schema: schema.to_json(),
            service_account,
            by: who.by(),
            at: now(),
        });
        judged.apps.keep(line)?;
        view(judged.apps, &id)
    })
    .map(Json)
}
