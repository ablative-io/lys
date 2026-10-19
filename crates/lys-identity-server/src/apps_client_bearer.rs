//! An approved app's bearer, its workspace credential: issued or rotated by
//! the administrator on `POST /apps/{app}/bearer/issue`, or by the connect
//! exchange as the administrator who approved the connect (DIRECTORY-102
//! R1), so a reconnect rotates the running app's bearer only when the app's
//! server collects the new one. Its value is answered once and kept nowhere.

use std::str::FromStr;
use std::sync::Arc;

use axum::Json;
use axum::extract::rejection::JsonRejection;
use axum::extract::{Path as UrlPath, State};
use axum::http::{HeaderMap, Method};
use lys_identity::OperationId;
use lys_identity::projection::Projection;
use serde::Deserialize;
use serde_json::json;

use super::{ClientCredentialIssueBody, approved, encoded, may_issue, unconfirmed};
use crate::apps_api::{malformed, with_apps};
use crate::apps_binding::acting;
use crate::apps_error::AppError;
use crate::apps_state::{By, Client, CustodyPrepared, Line};
use crate::apps_store::AppStore;
use crate::apps_views::AppBearerGiven;
use crate::error::ServerError;
use crate::routes::AppState;
use crate::session::now;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct IssuedBearer {
    app: String,
    owner: String,
    client_secret_ref: String,
    api_credential_ref: String,
    client_secret_sha256: String,
    credential: Option<String>,
}

/// Issue or rotate an approved app's bearer, answering its value once.
pub(crate) async fn issue_bearer(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    UrlPath(id): UrlPath<String>,
    body: Result<Json<ClientCredentialIssueBody>, JsonRejection>,
) -> Result<Json<AppBearerGiven>, ServerError> {
    let Json(body) = body.map_err(|refused| malformed(refused.body_text()))?;
    let operation = OperationId::from_str(&body.operation)?.to_string();
    let admitted = |apps: &AppStore, projection: &Projection| {
        may_issue(&acting(&state, apps.held(), &headers, projection)?)
    };
    with_apps(&state, |apps, projection| admitted(apps, projection))?;
    let person = crate::secrets_api::person(&state, &headers)?;
    bearer_as(&state, &person, &id, operation, admitted)
        .await
        .map(Json)
}

/// Issue or rotate app `id`'s bearer under `operation`, asking the broker on
/// behalf of `person` and keeping its custody as the one `admitted` answers,
/// judged before the broker is asked and again after. A repeated operation
/// answers no value.
pub(crate) async fn bearer_as(
    state: &AppState,
    person: &str,
    id: &str,
    operation: String,
    admitted: impl Fn(&AppStore, &Projection) -> Result<By, ServerError>,
) -> Result<AppBearerGiven, ServerError> {
    let (repeated, expected) = with_apps(state, |apps, projection| {
        admitted(apps, projection)?;
        if let Some(line) = apps.held().operation(&operation) {
            return match line {
                Line::CustodyPrepared(prepared) if prepared.app == id && prepared.bearer_issued => {
                    Ok((
                        Some(AppBearerGiven {
                            app: id.to_owned(),
                            reference: prepared.api_credential_ref,
                            credential: None,
                        }),
                        String::new(),
                    ))
                }
                _ => Err(AppError::AppOperationReused {
                    operation: operation.clone(),
                }
                .into()),
            };
        }
        let app = approved(apps, id)?;
        let approval = app
            .approved
            .as_ref()
            .ok_or_else(|| AppError::AppNotApproved { app: id.to_owned() })?;
        Ok((None, approval.client.secret_sha256.clone()))
    })?;
    if let Some(given) = repeated {
        return Ok(given);
    }
    let answer = crate::secrets_api::ask_as(state, person, Method::POST, "/_lys/apps/bearer/issue", encoded(&json!({
        "app": id, "operation": operation, "expected_digest": expected, "upstream": state.identity_upstream,
    }))).await?;
    let issued: IssuedBearer =
        serde_json::from_value(answer).map_err(|_error| unconfirmed("the app bearer's issue"))?;
    let prefix = format!("lys-app-{}-{id}", issued.owner);
    let shaped = issued.client_secret_sha256.len() == 64
        && issued
            .client_secret_sha256
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit());
    let value_valid = issued.credential.as_ref().is_none_or(|value| {
        value
            .strip_prefix(&format!("lys-app.{id}."))
            .is_some_and(|secret| {
                secret.len() == 64
                    && secret.bytes().all(|byte| byte.is_ascii_hexdigit())
                    && crate::apps_binding::sha256_hex(secret) == issued.client_secret_sha256
            })
    });
    if issued.app != id
        || !shaped
        || !value_valid
        || issued.client_secret_ref != format!("{prefix}-client")
        || issued.api_credential_ref != format!("{prefix}-api-{operation}")
    {
        return Err(unconfirmed("the app bearer's custody"));
    }
    let by = with_apps(state, |apps, projection| admitted(apps, projection))?;
    crate::apps_credentials::keep_custody_by(
        state,
        CustodyPrepared {
            operation,
            app: id.to_owned(),
            client: Client {
                client_id: id.to_owned(),
                secret_sha256: issued.client_secret_sha256,
            },
            owner: issued.owner,
            client_secret_ref: issued.client_secret_ref,
            api_credential_ref: issued.api_credential_ref.clone(),
            bearer_issued: true,
            by,
            at: now(),
        },
        &expected,
        |apps, projection| admitted(apps, projection).map(|_by| ()),
    )?;
    Ok(AppBearerGiven {
        app: id.to_owned(),
        reference: issued.api_credential_ref,
        credential: issued.credential,
    })
}
