//! The one act that gives an app approved before connectors were its
//! connector (DIRECTORY-080 R1, box 12.3), at `POST /apps/{app}/connector`:
//! the administrator writes the line an approval writes today, under an
//! operation of its own, answering to the person who holds their login.
//! Nothing is derived at open and no line already written changes; an app
//! that holds a connector is refused `connector_exists`, and one that is not
//! approved by its standing's name. The same act sent again under its
//! operation answers the same and keeps nothing new.

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
use crate::apps_state::Line;
use crate::apps_views::AppView;
use crate::error::ServerError;
use crate::routes::AppState;
use crate::session::now;

/// The act an administrator sends to give an app its connector.
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ConnectorBody {
    operation: String,
}

/// `POST /apps/{app}/connector`: give an approved app that has none its
/// connector, as the administrator. Answers the app as it now stands.
pub(crate) async fn give(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    UrlPath(id): UrlPath<String>,
    body: Result<Json<ConnectorBody>, JsonRejection>,
) -> Result<Json<AppView>, ServerError> {
    let Json(body) = body.map_err(|refused| malformed(refused.body_text()))?;
    let operation = OperationId::from_str(&body.operation)?.to_string();
    with_apps(&state, |apps, projection| {
        let by = administrator(&state, apps, &headers, projection)?;
        if let Some(kept) = apps.held().operation(&operation) {
            return match kept {
                Line::Connector(line) if line.app == id => view(apps, &id),
                _ => Err(AppError::AppOperationReused { operation }.into()),
            };
        }
        let line = crate::apps_connector::made(&by, projection, &operation, &id, now())?;
        apps.keep(Line::Connector(line))?;
        view(apps, &id)
    })
    .map(Json)
}
