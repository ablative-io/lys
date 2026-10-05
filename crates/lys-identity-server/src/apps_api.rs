//! The app routes: an app is registered through the API, by the signed-in
//! administrator or by a registrar service account, and has no effect until
//! the administrator approves it on the Apps screen.
//!
//! An id already registered, `lys` among them, is refused `app_exists` before
//! anything else of the registration is read.
//!
//! `POST /apps` records the registration as pending and nothing else: no
//! client exists anywhere and no kind of the app is judged. Approval creates
//! the app's sign-in client only after confirmed broker custody. The app log keeps only the
//! secret's SHA-256. Approval takes the app's sign-in settings, the exact
//! return addresses and whether it is given the person's name, kept beside
//! it (`apps_sign_in`); it binds the service
//! account the registration names, makes the registered schema version 1
//! and gives its kinds to the grants. A declined app never takes effect. A
//! retired app's client credential is refused and every check on its kinds
//! is refused `app_retired`; its grants stay in the log, readable. The app
//! `lys` is never retired.
//!
//! Lys calls no app: a redirect address is kept to be sent back to, never
//! fetched, and nothing here waits on or reads anything an app holds.
//!
//! Each act is sent under an operation id; the same act sent again in the
//! same words answers what it did and writes nothing, except that an
//! approval's secret is answered only the first time.

use std::path::Path;
use std::str::FromStr;
use std::sync::Arc;

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path as UrlPath, State};
use axum::http::HeaderMap;
use axum::routing::{get, post};
use axum::{Json, Router};
use lys_core::Ed25519Identity;
use lys_identity::OperationId;
use lys_identity::grants::{AppSchema, LYS_APP, app_id};
use serde::Deserialize;
use serde_json::Value;

use crate::apps_binding::{Acting, Binding, Registrar, acting, new_secret};
use crate::apps_error::AppError;
use crate::apps_state::{
    Approved, By, Client, Decided, Line, LysRecorded, Registered, SignInSet, Standing,
};
use crate::apps_store::AppStore;
use crate::apps_views::{AppView, Approval, AppsView, RegistrarIssued};
use crate::config::Config;
use crate::error::ServerError;
use crate::routes::{AppState, with_directory};
use crate::session::now;

/// The operation the app `lys` is recorded under.
const LYS_OPERATION: &str = "lys-model";

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct RegisterBody {
    operation: String,
    id: String,
    name: String,
    redirects: Vec<String>,
    #[schema(value_type = Object)]
    schema: Value,
    #[serde(default)]
    service_account: Option<String>,
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct DecideBody {
    operation: String,
    #[serde(default)]
    reason: String,
}

/// An approval: with it the administrator sets the app's sign-in settings,
/// the exact return addresses and whether it is given the person's name, so
/// an approval is complete without a second act. The registration's
/// addresses are what the screen offers; what is sent here is what is kept.
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ApproveBody {
    operation: String,
    /// The addresses the sign-in client may send a person back to, exactly.
    redirects: Vec<String>,
    /// Whether the app is given the person's name (the profile scope).
    profile: bool,
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct RegistrarBody {
    operation: String,
    service_account: String,
}

/// The app routes.
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/apps", post(register).get(list))
        .route("/apps/me", get(me))
        .route("/apps/registrars", post(registrar))
        .route("/apps/{app}", get(one))
        .route("/apps/{app}/approve", post(approve))
        .route("/apps/{app}/sign_in", post(crate::apps_sign_in::set))
        .route(
            "/apps/{app}/credentials/save",
            post(crate::apps_credentials::save),
        )
        .route("/apps/{app}/decline", post(decline))
        .route("/apps/{app}/retire", post(retire))
}

/// The apps kept beside the grant log `config` names, with the app `lys`
/// recorded from the model file when the log does not hold it yet, saying
/// through `say` how the log started and where the app `lys`'s schema came
/// from. Once the log holds the app `lys`, the model file is not read.
pub fn opened(
    config: &Config,
    key: Arc<Ed25519Identity>,
    say: &dyn Fn(&str),
) -> Result<AppStore, ServerError> {
    let dir: &Path = &config.apps_dir();
    let mut store = AppStore::open(dir, key)?;
    say(&format!(
        "apps log {}, holding {} apps",
        store.start(),
        store.held().apps.len()
    ));
    let held = store
        .app(LYS_APP)
        .and_then(|lys| lys.current())
        .map(|current| current.version);
    if let Some(version) = held {
        crate::apps_upgrade::upgrade(config, &mut store, version, say)?;
        return Ok(store);
    }
    let model = config.grant_model()?;
    let relations = model
        .relations()
        .map(|(relation, actions)| (relation.clone(), actions.clone()))
        .collect();
    let schema = AppSchema::lys(relations);
    store.keep(Line::Lys(LysRecorded {
        operation: LYS_OPERATION.to_owned(),
        version: model.version(),
        schema: schema.to_json(),
        at: now(),
    }))?;
    say(&format!(
        "lys_model_recorded: the model in {} is recorded once as the app lys's schema, version {}",
        config.grant_model_file.display(),
        model.version()
    ));
    Ok(store)
}

pub(crate) fn malformed(reason: impl Into<String>) -> ServerError {
    ServerError::RequestMalformed {
        reason: reason.into(),
    }
}

/// Run `act` with the directory held and then the apps, in the service's
/// lock order, the apps settled first.
pub(crate) fn with_apps<T>(
    state: &AppState,
    act: impl FnOnce(&mut AppStore, &lys_identity::projection::Projection) -> Result<T, ServerError>,
) -> Result<T, ServerError> {
    with_directory(state, |directory| {
        let mut apps =
            state
                .apps
                .lock()
                .map_err(|error| crate::apps_error::AppError::AppsUnavailable {
                    reason: format!("the apps lock is poisoned: {error}"),
                })?;
        apps.settle()?;
        act(&mut apps, directory.projection()?)
    })
}

/// Publish the current app model after the calling mutation releases its locks.
pub(crate) fn refresh(state: &AppState) -> Result<(), ServerError> {
    crate::apps_refresh::refresh(
        &state.directory,
        &state.apps,
        &state.grants,
        &state.grant_setup,
        |engine, model| {
            if let Some(engine) = engine {
                engine.set_app_kinds(model.kinds())?;
            }
            Ok(())
        },
    )
}

fn words(name: &str, text: &str) -> Result<String, ServerError> {
    let text = text.trim();
    if text.chars().any(char::is_control) {
        return Err(malformed(format!("{name} carries a control character")));
    }
    Ok(text.to_owned())
}

/// Refuse a redirect address a sign-in client does not take: an absolute
/// `https` address, or `http` to this machine, with no fragment. The one
/// rule for a registration's addresses and for the sign-in settings' alike.
pub(crate) fn redirect(address: &str) -> Result<String, AppError> {
    let refused = |reason| AppError::RedirectInvalid {
        address: address.to_owned(),
        reason,
    };
    let url =
        reqwest::Url::parse(address).map_err(|_unparsed| refused("is not an absolute address"))?;
    let local = matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"));
    match url.scheme() {
        "https" => {}
        "http" if local => {}
        _ => return Err(refused("is neither https nor http to this machine")),
    }
    if url.fragment().is_some() {
        return Err(refused("carries a fragment"));
    }
    Ok(address.to_owned())
}

/// Refuse a service account the service accounts do not hold, or hold retired.
fn service_account_held(state: &AppState, id: &str) -> Result<(), ServerError> {
    let Some(store) = state.service_accounts.as_ref() else {
        return Err(ServerError::ServiceAccountsUnavailable {
            reason: "the configuration names no service_accounts_dir, so no service account can be bound to an app".to_owned(),
        });
    };
    let store = store
        .lock()
        .map_err(|error| ServerError::ServiceAccountsUnavailable {
            reason: format!("the service accounts lock is poisoned: {error}"),
        })?;
    match store.account(id) {
        Some(account) if !account.is_retired() => Ok(()),
        Some(_) => Err(ServerError::ServiceAccountRetired {
            account: id.to_owned(),
        }),
        None => Err(ServerError::ServiceAccountUnknown),
    }
}

pub(crate) async fn register(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Result<Json<RegisterBody>, JsonRejection>,
) -> Result<Json<AppView>, ServerError> {
    let Json(body) = body.map_err(|refused| malformed(refused.body_text()))?;
    let operation = OperationId::from_str(&body.operation)?.to_string();
    crate::grants::with_grants(&state, |mut judged| {
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

pub(crate) fn view(apps: &AppStore, id: &str) -> Result<AppView, ServerError> {
    apps.app(id)
        .map(AppView::from)
        .ok_or_else(|| AppError::AppUnknown { app: id.to_owned() }.into())
}

/// Whether `who` may see `app`: the administrator sees every app, an app
/// itself, a registrar those it registered, and a signed-in person those
/// approved.
fn sees(who: &Acting, app: &crate::apps_state::App) -> bool {
    match who {
        Acting::Administrator(_) => true,
        Acting::App { app: own, .. } => *own == app.registered.app,
        Acting::Registrar { service_account } => {
            app.registered.by
                == By::ServiceAccount {
                    id: service_account.clone(),
                }
        }
        Acting::Person(_) => app.standing() == Standing::Approved,
    }
}

async fn list(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<AppsView>, ServerError> {
    with_apps(&state, |apps, projection| {
        let who = acting(&state, apps.held(), &headers, projection)?;
        Ok(AppsView {
            apps: apps
                .held()
                .apps
                .iter()
                .filter(|app| sees(&who, app))
                .map(AppView::from)
                .collect(),
        })
    })
    .map(Json)
}

async fn one(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    UrlPath(id): UrlPath<String>,
) -> Result<Json<AppView>, ServerError> {
    with_apps(&state, |apps, projection| {
        let who = acting(&state, apps.held(), &headers, projection)?;
        apps.app(&id)
            .filter(|app| sees(&who, app))
            .map(AppView::from)
            .ok_or_else(|| AppError::AppUnknown { app: id.clone() }.into())
    })
    .map(Json)
}

/// The app a credential signs in as: an app's sign-in to Lys's API.
async fn me(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<AppView>, ServerError> {
    with_apps(&state, |apps, projection| {
        match acting(&state, apps.held(), &headers, projection)? {
            Acting::App { app, .. } => view(apps, &app),
            _ => Err(AppError::CredentialRefused {
                reason: "the request carries no app credential",
            }
            .into()),
        }
    })
    .map(Json)
}

/// The administrator acting on `headers`, with the apps held.
pub(crate) fn administrator(
    state: &AppState,
    apps: &AppStore,
    headers: &HeaderMap,
    projection: &lys_identity::projection::Projection,
) -> Result<By, ServerError> {
    let who = acting(state, apps.held(), headers, projection)?;
    who.administrator()?;
    Ok(who.by())
}

async fn approve(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    UrlPath(id): UrlPath<String>,
    body: Result<Json<ApproveBody>, JsonRejection>,
) -> Result<Json<Approval>, ServerError> {
    let Json(body) = body.map_err(|refused| malformed(refused.body_text()))?;
    let operation = OperationId::from_str(&body.operation)?.to_string();
    // The sign-in settings are judged before any broker work: an approval
    // that would leave the app unable to sign anyone in is refused whole.
    let redirects = crate::apps_sign_in::redirects(&id, &body.redirects)?;
    // Authenticate and check pending state before doing any broker work. Never
    // hold directory/apps locks across a network wait; recheck after custody.
    let pending = crate::apps_credentials::pending(&state, &headers, &id, &operation)?;
    let prepared = if pending {
        Some(crate::apps_credentials::prepare(&state, &headers, &id).await?)
    } else {
        None
    };
    let answer = with_apps(&state, |apps, projection| {
        let by = administrator(&state, apps, &headers, projection)?;
        if let Some(Line::Approved(approved)) = apps.held().operation(&operation) {
            if approved.app != id {
                return Err(AppError::AppOperationReused { operation }.into());
            }
            return Ok(Approval {
                app: view(apps, &id)?,
                client: None,
                credentials: None,
            });
        }
        let app = apps
            .app(&id)
            .ok_or_else(|| AppError::AppUnknown { app: id.clone() })?;
        if app.standing() != Standing::Pending {
            return Err(AppError::AppDecided { app: id.clone() }.into());
        }
        let at = now();
        let binding = app
            .registered
            .service_account
            .clone()
            .map(|service_account| Binding {
                service_account,
                app: id.clone(),
                bound_by: by.clone(),
                at,
            });
        let (digest, credentials) = prepared.ok_or_else(|| ServerError::SecretsUnavailable {
            reason: "app approval requires confirmed broker credential custody".to_owned(),
        })?;
        let client = Client {
            client_id: id.clone(),
            secret_sha256: digest,
        };
        apps.keep(Line::Approved(Approved {
            operation: operation.clone(),
            app: id.clone(),
            client,
            binding,
            by: by.clone(),
            at,
        }))?;
        apps.keep_beside_approval(Line::SignInSet(SignInSet {
            operation,
            app: id.clone(),
            redirects,
            profile: body.profile,
            by,
            at,
        }))?;
        Ok(Approval {
            app: view(apps, &id)?,
            client: None,
            credentials: Some(credentials),
        })
    })?;
    refresh(&state)?;
    Ok(Json(answer))
}

/// Keep the decision `made` makes on app `id`, as the administrator.
fn decided(
    state: &AppState,
    headers: &HeaderMap,
    id: &str,
    body: &DecideBody,
    made: fn(Decided) -> Line,
) -> Result<AppView, ServerError> {
    let operation = OperationId::from_str(&body.operation)?.to_string();
    let reason = words("reason", &body.reason)?;
    let (answer, retiring) = with_apps(state, |apps, projection| {
        let by = administrator(state, apps, headers, projection)?;
        let line = made(Decided {
            operation,
            app: id.to_owned(),
            reason,
            by,
            at: now(),
        });
        let retiring = matches!(line, Line::Retired(_));
        apps.keep(line)?;
        Ok((view(apps, id)?, retiring))
    })?;
    if retiring {
        refresh(state)?;
    }
    Ok(answer)
}

async fn decline(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    UrlPath(id): UrlPath<String>,
    body: Result<Json<DecideBody>, JsonRejection>,
) -> Result<Json<AppView>, ServerError> {
    let Json(body) = body.map_err(|refused| malformed(refused.body_text()))?;
    decided(&state, &headers, &id, &body, Line::Declined).map(Json)
}

async fn retire(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    UrlPath(id): UrlPath<String>,
    body: Result<Json<DecideBody>, JsonRejection>,
) -> Result<Json<AppView>, ServerError> {
    let Json(body) = body.map_err(|refused| malformed(refused.body_text()))?;
    if id == LYS_APP {
        return Err(AppError::AppIsLys {
            reason: "the app lys is Lys itself and is never retired",
        }
        .into());
    }
    decided(&state, &headers, &id, &body, Line::Retired).map(Json)
}

/// Make a service account a registrar, answering its credential once.
async fn registrar(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Result<Json<RegistrarBody>, JsonRejection>,
) -> Result<Json<RegistrarIssued>, ServerError> {
    let Json(body) = body.map_err(|refused| malformed(refused.body_text()))?;
    let operation = OperationId::from_str(&body.operation)?.to_string();
    with_apps(&state, |apps, projection| {
        let by = administrator(&state, apps, &headers, projection)?;
        if let Some(Line::Registrar(made)) = apps.held().operation(&operation) {
            if made.service_account != body.service_account {
                return Err(AppError::AppOperationReused { operation }.into());
            }
            return Ok(RegistrarIssued {
                service_account: made.service_account,
                credential: None,
            });
        }
        service_account_held(&state, &body.service_account)?;
        let (secret, digest) = new_secret()?;
        apps.keep(Line::Registrar(Registrar {
            operation,
            service_account: body.service_account.clone(),
            secret_sha256: digest,
            by,
            at: now(),
        }))?;
        Ok(RegistrarIssued {
            credential: Some(format!(
                "{}.{}.{secret}",
                crate::apps_binding::REGISTRAR_CREDENTIAL,
                body.service_account
            )),
            service_account: body.service_account,
        })
    })
    .map(Json)
}
