//! The service account routes: a signed-in person creates service accounts
//! they own and retires them, and reads the ones they own; the administrator
//! may name any person as the owner, retires any, and reads every one.
//!
//! Each act is sent under an operation id. A creation is named by its
//! operation id; a retirement carries one of its own. The same act sent again
//! in the same words answers what was recorded and writes nothing; the same
//! operation in other words is refused `ServiceAccountReused`.
//!
//! A service account another person owns is refused exactly as one that is
//! not kept, `ServiceAccountUnknown`, so no refusal says whether it exists.

use std::str::FromStr;
use std::sync::{Arc, PoisonError};

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::routing::post;
use axum::{Json, Router};
use lys_identity::{LifecycleState, OperationId, PersonId};
use serde::Deserialize;

use crate::error::ServerError;
use crate::read_api::{login, own_person, person_record};
use crate::read_views::{ServiceAccountView, ServiceAccountsView};
use crate::routes::{AppState, signed_in, with_directory};
use crate::service_accounts_state::{Account, Created, Retired};
use crate::service_accounts_store::ServiceAccountStore;
use crate::session::now;

/// The most characters a service account's name carries.
const NAME_MAX: usize = 100;

/// The most characters a service account's description carries.
const DESCRIPTION_MAX: usize = 500;

/// A service account as its creator sends it.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CreateBody {
    operation: String,
    name: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    owner: Option<String>,
}

/// A retirement as it is sent.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RetireBody {
    operation: String,
}

/// The service account routes.
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/service-accounts", post(create).get(list))
        .route("/service-accounts/{id}/retire", post(retire))
}

fn malformed(reason: impl Into<String>) -> ServerError {
    ServerError::RequestMalformed {
        reason: reason.into(),
    }
}

fn with_service_accounts<T>(
    state: &AppState,
    act: impl FnOnce(&mut ServiceAccountStore) -> Result<T, ServerError>,
) -> Result<T, ServerError> {
    let store =
        state
            .service_accounts
            .as_ref()
            .ok_or_else(|| ServerError::ServiceAccountsUnavailable {
                reason: "the configuration names no service_accounts_dir".to_owned(),
            })?;
    let mut store = store.lock().unwrap_or_else(PoisonError::into_inner);
    store.settle()?;
    act(&mut store)
}

/// The service accounts `person` owns and may use: every one not retired, in
/// the order created; none when the configuration names no service accounts.
pub(crate) fn owned_by(
    state: &AppState,
    person: PersonId,
) -> Result<Vec<ServiceAccountView>, ServerError> {
    if state.service_accounts.is_none() {
        return Ok(Vec::new());
    }
    let owner = person.to_string();
    with_service_accounts(state, |store| {
        Ok(store
            .accounts()
            .iter()
            .filter(|account| account.created.owner == owner && !account.is_retired())
            .map(view)
            .collect())
    })
}

fn view(account: &Account) -> ServiceAccountView {
    let created = &account.created;
    ServiceAccountView {
        id: created.id.clone(),
        owner: created.owner.clone(),
        name: created.name.clone(),
        description: created.description.clone(),
        state: if account.is_retired() {
            "retired"
        } else {
            "active"
        }
        .to_owned(),
        created_by: created.by.clone(),
        created_at: created.at,
        retired_by: account.retired.as_ref().map(|retired| retired.by.clone()),
        retired_at: account.retired.as_ref().map(|retired| retired.at),
    }
}

fn words(name: &str, text: &str, most: usize) -> Result<String, ServerError> {
    let text = text.trim();
    if text.chars().count() > most {
        return Err(malformed(format!(
            "{name} is longer than {most} characters"
        )));
    }
    if text.chars().any(char::is_control) {
        return Err(malformed(format!("{name} carries a control character")));
    }
    Ok(text.to_owned())
}

async fn create(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Result<Json<CreateBody>, JsonRejection>,
) -> Result<Json<ServiceAccountView>, ServerError> {
    let Json(body) = body.map_err(|refused| malformed(refused.body_text()))?;
    let actor = signed_in(&state, &headers)?;
    let administrator = state.admission.administrator(&actor).is_ok();
    let id = OperationId::from_str(&body.operation)?.to_string();
    let name = words("name", &body.name, NAME_MAX)?;
    if name.is_empty() {
        return Err(malformed("a service account has a name"));
    }
    let description = words("description", &body.description, DESCRIPTION_MAX)?;
    with_directory(&state, |directory| {
        let projection = directory.projection()?;
        let owner = match body.owner.as_deref() {
            None => own_person(projection, &actor)?,
            Some(named) => {
                let owner = PersonId::from_str(named)
                    .map_err(|error| malformed(format!("owner: {error}")))?;
                if !administrator && projection.person_for(actor.binding()) != Some(owner) {
                    return Err(ServerError::NotAdmitted {
                        reason: "only the administrator names another person as a service account's owner",
                    });
                }
                owner
            }
        };
        let owner_retired = person_record(projection, owner)?.state() == LifecycleState::Retired;
        let created = Created {
            id,
            owner: owner.to_string(),
            name,
            description,
            by: login(actor.binding()),
            at: now(),
        };
        with_service_accounts(&state, |store| {
            if owner_retired && !store.names(&created.id) {
                return Err(ServerError::ServiceAccountOwnerRetired {
                    owner: created.owner,
                });
            }
            store.create(created).map(|account| view(&account))
        })
    })
    .map(Json)
}

async fn retire(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    body: Result<Json<RetireBody>, JsonRejection>,
) -> Result<Json<ServiceAccountView>, ServerError> {
    let Json(body) = body.map_err(|refused| malformed(refused.body_text()))?;
    let actor = signed_in(&state, &headers)?;
    let administrator = state.admission.administrator(&actor).is_ok();
    let account = OperationId::from_str(&id)
        .ok()
        .ok_or(ServerError::ServiceAccountUnknown)?
        .to_string();
    let operation = OperationId::from_str(&body.operation)?.to_string();
    with_directory(&state, |directory| {
        let projection = directory.projection()?;
        let own = if administrator {
            None
        } else {
            Some(own_person(projection, &actor)?.to_string())
        };
        with_service_accounts(&state, |store| {
            let visible = store.account(&account).is_some_and(|kept| {
                own.as_ref()
                    .is_none_or(|person| *person == kept.created.owner)
            });
            if !visible {
                return Err(ServerError::ServiceAccountUnknown);
            }
            store
                .retire(Retired {
                    operation,
                    account,
                    by: login(actor.binding()),
                    at: now(),
                })
                .map(|account| view(&account))
        })
    })
    .map(Json)
}

async fn list(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<ServiceAccountsView>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    let administrator = state.admission.administrator(&actor).is_ok();
    with_directory(&state, |directory| {
        let own = if administrator {
            None
        } else {
            Some(own_person(directory.projection()?, &actor)?.to_string())
        };
        with_service_accounts(&state, |store| {
            Ok(ServiceAccountsView {
                scope: if own.is_some() {
                    "personal"
                } else {
                    "directory"
                }
                .to_owned(),
                service_accounts: store
                    .accounts()
                    .iter()
                    .filter(|account| {
                        own.as_ref()
                            .is_none_or(|person| *person == account.created.owner)
                    })
                    .map(view)
                    .collect(),
            })
        })
    })
    .map(Json)
}
