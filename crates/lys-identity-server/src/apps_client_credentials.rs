//! An app's virtual client credentials (DIRECTORY-081): issued on the Apps
//! screen and answered once, listed with the app, and revoked there.
//!
//! Issuing and revoking are judged by their own named permission,
//! `issue_app_client_credential`, held today only through the
//! administrator; registering an app does not hold it. The secrets broker
//! makes each credential and keeps only its digest beside the app's sealed
//! client secret; the apps' record keeps its id, who issued it and when, and
//! never its value.
//!
//! The apps' record says which credentials are live, and the token exchange
//! sends that list with every confirmation it asks of the broker, so a
//! credential the record has ended is refused whatever the broker last
//! heard. A revocation, like a retirement, is kept first and never waits on
//! the broker; the broker is then asked to end the credential, and when it
//! cannot answer the ending is left for the next administrator act on any
//! app's credentials or standing, which asks it before anything else. Each
//! confirmed ending is kept as a `ClientCredentialsEnded` line.
//!
//! No lock is held while the broker is asked.

use std::str::FromStr;
use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::rejection::JsonRejection;
use axum::extract::{Path as UrlPath, State};
use axum::http::{HeaderMap, Method};
use axum::routing::post;
use axum::{Json, Router};
use lys_identity::OperationId;
use serde::Deserialize;
use serde_json::json;

use crate::apps_api::{malformed, view, with_apps};
use crate::apps_binding::{Acting, acting};
use crate::apps_error::AppError;
use crate::apps_state::{
    App, By, Client, ClientCredentialIssued, ClientCredentialRevoked, ClientCredentialsEnded,
    CustodyPrepared, Line, Standing,
};
use crate::apps_views::{AppView, ClientCredentialGiven};
use crate::error::ServerError;
use crate::routes::AppState;
use crate::session::now;

/// The prefix every virtual client credential carries.
const APP_CLIENT_PREFIX: &str = "lys-client.";

/// The routes that issue and revoke an app's client credentials, merged
/// into the app routes.
pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/apps/{app}/credentials/issue", post(issue))
        .route("/apps/{app}/bearer/issue", post(issue_bearer))
        .route("/apps/{app}/credentials/{credential}/revoke", post(revoke))
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ClientCredentialIssueBody {
    operation: String,
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ClientCredentialRevokeBody {
    operation: String,
    /// Why, in the administrator's words; may be empty.
    #[serde(default)]
    reason: String,
}

/// What the broker answers an issue with.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Issued {
    app: String,
    credential_id: String,
    owner: String,
    value: String,
    client_secret_sha256: String,
}

/// What the broker answers an ending with.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Ended {
    app: String,
    ended: Vec<String>,
}

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
) -> Result<Json<crate::apps_views::AppBearerGiven>, ServerError> {
    let Json(body) = body.map_err(|refused| malformed(refused.body_text()))?;
    let operation = OperationId::from_str(&body.operation)?.to_string();
    let (repeated, expected) = with_apps(&state, |apps, projection| {
        may_issue(&acting(&state, apps.held(), &headers, projection)?)?;
        if let Some(line) = apps.held().operation(&operation) {
            return match line {
                Line::CustodyPrepared(prepared) if prepared.app == id && prepared.bearer_issued => {
                    Ok((
                        Some(crate::apps_views::AppBearerGiven {
                            app: id.clone(),
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
        let app = approved(apps, &id)?;
        let approval = app
            .approved
            .as_ref()
            .ok_or_else(|| AppError::AppNotApproved { app: id.clone() })?;
        Ok((None, approval.client.secret_sha256.clone()))
    })?;
    if let Some(given) = repeated {
        return Ok(Json(given));
    }
    let answer = crate::secrets_api::ask(&state, &headers, Method::POST, "/_lys/apps/bearer/issue", encoded(&json!({
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
    let by = with_apps(&state, |apps, projection| {
        may_issue(&acting(&state, apps.held(), &headers, projection)?)
    })?;
    crate::apps_credentials::keep_custody(
        &state,
        &headers,
        CustodyPrepared {
            operation,
            app: id.clone(),
            client: Client {
                client_id: id.clone(),
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
    )?;
    Ok(Json(crate::apps_views::AppBearerGiven {
        app: id,
        reference: issued.api_credential_ref,
        credential: issued.credential,
    }))
}

/// Who acts, when they hold `issue_app_client_credential`; its refusal
/// names the permission.
fn may_issue(who: &Acting) -> Result<By, ServerError> {
    who.administrator()
        .map_err(|_refused| ServerError::NotAdmitted {
            reason: "issue_app_client_credential is held only through the administrator, on a Lys screen",
        })?;
    Ok(who.by())
}

/// The app `id`, approved, or the refusal by its standing's name.
fn approved<'a>(apps: &'a crate::apps_store::AppStore, id: &str) -> Result<&'a App, ServerError> {
    let app = apps
        .app(id)
        .ok_or_else(|| AppError::AppUnknown { app: id.to_owned() })?;
    match app.standing() {
        Standing::Approved => Ok(app),
        Standing::Retired => Err(AppError::AppRetired { app: id.to_owned() }.into()),
        Standing::Pending | Standing::Declined => {
            Err(AppError::AppNotApproved { app: id.to_owned() }.into())
        }
    }
}

fn unconfirmed(what: &str) -> ServerError {
    ServerError::SecretsUnavailable {
        reason: format!("the secrets broker did not confirm {what}"),
    }
}

fn encoded(body: &serde_json::Value) -> Bytes {
    Bytes::from(body.to_string())
}

/// Issue app `id` a client credential, answering its value once.
pub(crate) async fn issue(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    UrlPath(id): UrlPath<String>,
    body: Result<Json<ClientCredentialIssueBody>, JsonRejection>,
) -> Result<Json<ClientCredentialGiven>, ServerError> {
    let Json(body) = body.map_err(|refused| malformed(refused.body_text()))?;
    let operation = OperationId::from_str(&body.operation)?.to_string();
    let (repeated, previous_digest) = with_apps(&state, |apps, projection| {
        may_issue(&acting(&state, apps.held(), &headers, projection)?)?;
        match apps.held().operation(&operation) {
            Some(Line::ClientCredentialIssued(issued)) if issued.app == id => Ok((
                Some(ClientCredentialGiven {
                    app: issued.app,
                    credential_id: issued.credential_id,
                    credential: None,
                }),
                String::new(),
            )),
            Some(_) => Err(AppError::AppOperationReused {
                operation: operation.clone(),
            }
            .into()),
            None => {
                let app = approved(apps, &id)?;
                let approval = app
                    .approved
                    .as_ref()
                    .ok_or_else(|| AppError::AppNotApproved { app: id.clone() })?;
                Ok((None, approval.client.secret_sha256.clone()))
            }
        }
    })?;
    if let Some(given) = repeated {
        return Ok(Json(given));
    }
    let person = crate::secrets_api::person(&state, &headers)?;
    end_pending(&state, &person).await?;
    let answer = crate::secrets_api::ask_as(
        &state,
        &person,
        Method::POST,
        "/_lys/apps/client/issue",
        encoded(&json!({ "app": id })),
    )
    .await;
    let answer = match answer {
        Ok(answer) => answer,
        Err(ServerError::SecretsRefused { refusal, .. }) if refusal == "AppClientNoCustody" => {
            crate::apps_credentials::prepare(&state, &headers, &id).await?;
            crate::secrets_api::ask_as(
                &state,
                &person,
                Method::POST,
                "/_lys/apps/client/issue",
                encoded(&json!({"app": id})),
            )
            .await?
        }
        Err(error) => return Err(error),
    };
    let issued: Issued =
        serde_json::from_value(answer).map_err(|_error| unconfirmed("the credential's issue"))?;
    let shaped = issued.credential_id.len() == 16
        && issued
            .credential_id
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
        && issued
            .value
            .strip_prefix(APP_CLIENT_PREFIX)
            .and_then(|rest| rest.strip_prefix(id.as_str()))
            .is_some_and(|rest| rest.starts_with('.'));
    if issued.app != id
        || !shaped
        || issued.client_secret_sha256.len() != 64
        || !issued
            .client_secret_sha256
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(unconfirmed("the credential's issue"));
    }
    let current = with_apps(&state, |apps, _projection| {
        approved(apps, &id)?
            .approved
            .as_ref()
            .map(|approved| approved.client.secret_sha256.clone())
            .ok_or_else(|| AppError::AppNotApproved { app: id.clone() }.into())
    })?;
    if current != issued.client_secret_sha256 {
        let by = with_apps(&state, |apps, projection| {
            may_issue(&acting(&state, apps.held(), &headers, projection)?)
        })?;
        let prefix = format!("lys-app-{}-{id}", issued.owner);
        crate::apps_credentials::keep_custody(
            &state,
            &headers,
            CustodyPrepared {
                operation: OperationId::generate()?.to_string(),
                app: id.clone(),
                client: Client {
                    client_id: id.clone(),
                    secret_sha256: issued.client_secret_sha256.clone(),
                },
                owner: issued.owner.clone(),
                client_secret_ref: format!("{prefix}-client"),
                api_credential_ref: format!("{prefix}-api"),
                bearer_issued: false,
                by,
                at: now(),
            },
            &previous_digest,
        )?;
    }
    let kept = with_apps(&state, |apps, projection| {
        let by = may_issue(&acting(&state, apps.held(), &headers, projection)?)?;
        if let Some(line) = apps.held().operation(&operation) {
            return match line {
                Line::ClientCredentialIssued(previous) if previous.app == id => {
                    Ok(Some(previous.credential_id))
                }
                _ => Err(AppError::AppOperationReused {
                    operation: operation.clone(),
                }
                .into()),
            };
        }
        apps.keep(Line::ClientCredentialIssued(ClientCredentialIssued {
            operation,
            app: id.clone(),
            credential_id: issued.credential_id.clone(),
            owner: issued.owner.clone(),
            by,
            at: now(),
        }))?;
        Ok(None)
    });
    if let Ok(Some(previous)) = &kept {
        end_at_broker(
            &state,
            &person,
            &id,
            std::slice::from_ref(&issued.credential_id),
            "duplicate issue operation",
        )
        .await?;
        return Ok(Json(ClientCredentialGiven {
            app: id,
            credential_id: previous.clone(),
            credential: None,
        }));
    }
    if let Err(refused) = kept {
        // Kept nowhere, so ended at once: a credential the record does not
        // hold never authenticates, and the broker holds none it does not.
        let ids = vec![issued.credential_id];
        if let Err(error) =
            end_at_broker(&state, &person, &id, &ids, "its issue was not kept").await
        {
            tracing::warn!(app = %id, %error, "an app client credential whose issue was not kept could not be ended at the broker; the apps' record never holds it live");
        }
        return Err(refused);
    }
    Ok(Json(ClientCredentialGiven {
        app: id,
        credential_id: issued.credential_id,
        credential: Some(issued.value),
    }))
}

/// Revoke app `id`'s client credential `credential`.
pub(crate) async fn revoke(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    UrlPath((id, credential)): UrlPath<(String, String)>,
    body: Result<Json<ClientCredentialRevokeBody>, JsonRejection>,
) -> Result<Json<AppView>, ServerError> {
    let Json(body) = body.map_err(|refused| malformed(refused.body_text()))?;
    let operation = OperationId::from_str(&body.operation)?.to_string();
    let reason = body.reason.trim().to_owned();
    if reason.chars().any(char::is_control) {
        return Err(malformed("reason carries a control character"));
    }
    with_apps(&state, |apps, projection| {
        let by = may_issue(&acting(&state, apps.held(), &headers, projection)?)?;
        apps.keep(Line::ClientCredentialRevoked(ClientCredentialRevoked {
            operation,
            app: id.clone(),
            credential_id: credential,
            reason,
            by,
            at: now(),
        }))
    })?;
    ended_after(&state, &headers, &id).await;
    with_apps(&state, |apps, _projection| view(apps, &id)).map(Json)
}

/// Ask the broker to end the credentials the record no longer holds live,
/// after an act already kept that never waits on it: a revocation or a
/// retirement. A broker that cannot answer is said in the log, and the
/// endings are asked again by the next administrator act on any app's
/// credentials or standing.
pub(crate) async fn ended_after(state: &AppState, headers: &HeaderMap, id: &str) {
    let ended = match crate::secrets_api::person(state, headers) {
        Ok(person) => end_pending(state, &person).await,
        Err(error) => Err(error),
    };
    if let Err(error) = ended {
        tracing::warn!(app = %id, %error, "app client credentials no longer live were not yet ended at the broker; the apps' record refuses them, and the next administrator act asks again");
    }
}

/// End at the broker every credential, of every app, the record no longer
/// holds live and the broker has not confirmed ending, keeping each app's
/// confirmation as it comes.
async fn end_pending(state: &AppState, person: &str) -> Result<(), ServerError> {
    let pending: Vec<(String, Vec<String>)> = with_apps(state, |apps, _projection| {
        Ok(apps
            .held()
            .apps
            .iter()
            .map(|app| (app.registered.app.clone(), app.client_credentials_to_end()))
            .filter(|(_app, ids)| !ids.is_empty())
            .collect())
    })?;
    for (id, ids) in pending {
        end_at_broker(
            state,
            person,
            &id,
            &ids,
            "no longer live in the apps' record",
        )
        .await?;
        with_apps(state, |apps, _projection| {
            apps.keep(Line::ClientCredentialsEnded(ClientCredentialsEnded {
                operation: OperationId::generate()?.to_string(),
                app: id.clone(),
                credential_ids: ids,
                at: now(),
            }))
            .map(|_kept| ())
        })?;
    }
    Ok(())
}

/// Ask the broker to end `ids` of app `id`, as `person`.
async fn end_at_broker(
    state: &AppState,
    person: &str,
    id: &str,
    ids: &[String],
    why: &str,
) -> Result<(), ServerError> {
    let answer = crate::secrets_api::ask_as(
        state,
        person,
        Method::POST,
        "/_lys/apps/client/end",
        encoded(&json!({ "app": id, "credential_ids": ids, "why": why })),
    )
    .await?;
    let ended: Ended =
        serde_json::from_value(answer).map_err(|_error| unconfirmed("the credentials' ending"))?;
    if ended.app != id || ended.ended.iter().any(|done| !ids.contains(done)) {
        return Err(unconfirmed("the credentials' ending"));
    }
    Ok(())
}
