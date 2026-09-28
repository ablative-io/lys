//! The HTTP routes mapping requests to the directory, each mutation behind admission.
//! The read-only views the identity screens draw are in `read_api`.

use std::str::FromStr;
use std::sync::{Arc, Mutex, PoisonError};

use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use lys_identity::receipt::Receipt;
use lys_identity::{
    Actor, AgentId, Directory, IdentityId, LifecycleState, LoginBinding, OperationId, PersonId,
    Profile, Transition,
};
use lys_log_store::FileLeafStore;
use serde::Deserialize;
use serde_json::{Value, json};

use lys_identity::signer::load_service_key;

use crate::admission::{AUTHORITY, Admission};
use crate::config::Config;
use crate::error::ServerError;
use crate::grants::{GrantSetup, GrantState};
use crate::oidc::Oidc;
use crate::session::{Sessions, now};

/// Everything a request is served from.
pub struct AppState {
    /// The directory, one caller at a time.
    pub directory: Mutex<Directory<FileLeafStore>>,
    /// Sign-in.
    pub oidc: Oidc,
    /// Live sessions.
    pub sessions: Sessions,
    /// Who is admitted to what.
    pub admission: Admission,
    /// The grants, opened on first use once the root authority exists.
    pub grants: Mutex<Option<GrantState>>,
    /// What the grants are opened from.
    pub grant_setup: GrantSetup,
}

type Shared = Arc<AppState>;

/// Open the directory, discover the issuer and answer the service's routes,
/// as `config` says. The log is created when its directory does not exist.
pub async fn service(config: &Config) -> Result<Router, ServerError> {
    let directory = open_directory(config)?;
    Ok(router(Arc::new(AppState {
        directory: Mutex::new(directory),
        oidc: Oidc::discover(config).await?,
        sessions: Sessions::new(config.session_seconds, config.secure_cookie),
        admission: Admission::new(
            config.administrator_binding()?,
            config.link_audit_binding()?,
        ),
        grants: Mutex::new(None),
        grant_setup: GrantSetup {
            log_dir: config.grant_log_dir.clone(),
            log_origin: config.grant_log_origin.clone(),
            key_file: config.event_key_file.clone(),
            model: config.grant_model()?,
            spicedb: config.spicedb.clone(),
        },
    })))
}

/// Open the directory `config` names, creating its log when the log's
/// directory does not exist.
pub fn open_directory(config: &Config) -> Result<Directory<FileLeafStore>, ServerError> {
    if !config.log_dir.exists() {
        FileLeafStore::create(&config.log_dir, &config.log_origin).map_err(|error| {
            ServerError::ConfigInvalid {
                reason: format!("the log could not be created: {error}"),
            }
        })?;
    }
    let log_dir = config.log_dir.clone();
    let reopen = Box::new(move || FileLeafStore::open(&log_dir));
    Ok(Directory::open(
        reopen,
        load_service_key(&config.event_key_file)?,
    )?)
}

/// The service's routes over `state`.
pub fn router(state: Shared) -> Router {
    Router::new()
        .route("/authority", get(authority))
        .route("/login", get(login))
        .route("/callback", get(callback))
        .route("/setup", post(crate::setup::finish))
        .route("/people", post(register_person))
        .route("/agents", post(register_agent))
        .route("/identities", get(list))
        .route("/identities/{id}", get(read))
        .route("/identities/{id}/profile", post(change_profile))
        .route("/identities/{id}/transitions", post(transition))
        .route("/people/{id}/logins", post(bind_login))
        .merge(crate::read_api::routes())
        .merge(crate::grants::routes())
        .merge(crate::receipts_api::routes())
        .merge(crate::link_audit_api::routes())
        .with_state(state)
}

fn malformed(reason: String) -> ServerError {
    ServerError::RequestMalformed { reason }
}

/// The signed-in actor, or a refusal.
pub(crate) fn signed_in(state: &AppState, headers: &HeaderMap) -> Result<Actor, ServerError> {
    let cookie = headers
        .get(header::COOKIE)
        .and_then(|value| value.to_str().ok());
    state.sessions.actor(cookie)
}

/// Run `act` on the directory, one caller at a time.
pub(crate) fn with_directory<T>(
    state: &AppState,
    act: impl FnOnce(&mut Directory<FileLeafStore>) -> Result<T, ServerError>,
) -> Result<T, ServerError> {
    let mut directory = state
        .directory
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    act(&mut directory)
}

/// A receipt as JSON.
pub(crate) fn receipt_json(receipt: &Receipt) -> Value {
    let coordinate = receipt.coordinate();
    json!({
        "version": receipt.version(),
        "operation": receipt.operation().to_string(),
        "actor": {
            "issuer": receipt.actor().binding().issuer(),
            "subject": receipt.actor().binding().subject(),
            "authenticated_at": receipt.actor().provenance().authenticated_at(),
        },
        "identity": receipt.identity().to_string(),
        "change_kind": receipt.change_kind(),
        "payload_commitment": hex(&receipt.payload_commitment()),
        "payload_commitment_hash": "sha-256",
        "log": {
            "index": coordinate.index,
            "tree_size": coordinate.tree_size,
            "root": hex(&coordinate.root),
            "leaf_hash": hex(&coordinate.leaf_hash),
        },
    })
}

const HEX_DIGITS: &[u8; 16] = b"0123456789abcdef";

/// Lowercase hex.
pub(crate) fn hex(bytes: &[u8]) -> String {
    let mut text = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        text.push(char::from(HEX_DIGITS[usize::from(byte >> 4)]));
        text.push(char::from(HEX_DIGITS[usize::from(byte & 0x0f)]));
    }
    text
}

pub(crate) fn identity_id(text: &str) -> Result<IdentityId, ServerError> {
    if text.starts_with("agent-") {
        return AgentId::from_str(text)
            .map(IdentityId::Agent)
            .map_err(ServerError::from);
    }
    PersonId::from_str(text)
        .map(IdentityId::Person)
        .map_err(ServerError::from)
}

async fn authority() -> &'static str {
    AUTHORITY
}

async fn login(State(state): State<Shared>) -> Result<Response, ServerError> {
    let url = state.oidc.begin()?;
    Ok((StatusCode::SEE_OTHER, [(header::LOCATION, url)]).into_response())
}

#[derive(Deserialize)]
struct Answer {
    code: String,
    state: String,
}

async fn callback(
    State(state): State<Shared>,
    Query(answer): Query<Answer>,
) -> Result<Response, ServerError> {
    let actor = state.oidc.finish(answer.code, &answer.state).await?;
    let body = json!({
        "signed_in": { "issuer": actor.binding().issuer(), "subject": actor.binding().subject() },
        "authority": AUTHORITY,
    });
    let cookie = state.sessions.begin(actor)?;
    Ok(([(header::SET_COOKIE, cookie)], Json(body)).into_response())
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Named {
    operation: String,
    display_name: String,
}

fn operation(text: &str) -> Result<OperationId, ServerError> {
    OperationId::from_str(text).map_err(ServerError::from)
}

async fn register_person(
    State(state): State<Shared>,
    headers: HeaderMap,
    Json(body): Json<Named>,
) -> Result<Json<Value>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    state.admission.administrator(&actor)?;
    let (op, profile) = (
        operation(&body.operation)?,
        Profile::new(&body.display_name)?,
    );
    with_directory(&state, |directory| {
        let (id, receipt) = directory.register_person(actor, op, profile, now())?;
        Ok(Json(
            json!({ "person": id.to_string(), "receipt": receipt_json(&receipt) }),
        ))
    })
}

async fn register_agent(
    State(state): State<Shared>,
    headers: HeaderMap,
    Json(body): Json<Named>,
) -> Result<Json<Value>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    state.admission.administrator(&actor)?;
    let (op, profile) = (
        operation(&body.operation)?,
        Profile::new(&body.display_name)?,
    );
    with_directory(&state, |directory| {
        let responsible = directory
            .projection()?
            .person_for(actor.binding())
            .ok_or(ServerError::NotAdmitted {
                reason: "the administrator's login is bound to no person, so no agent can be registered under them",
            })?;
        let (id, receipt) = directory.register_agent(actor, op, responsible, profile, now())?;
        Ok(Json(json!({
            "agent": id.to_string(),
            "responsible": responsible.to_string(),
            "receipt": receipt_json(&receipt),
        })))
    })
}

fn state_name(state: LifecycleState) -> String {
    state.to_string()
}

fn record_json(id: IdentityId, record: &lys_identity::projection::Record) -> Value {
    json!({
        "id": id.to_string(),
        "display_name": record.profile().display_name(),
        "state": state_name(record.state()),
        "responsible": record.responsible().map(|person| person.to_string()),
        "logins": record.bindings().iter().map(|binding| json!({
            "issuer": binding.issuer(), "subject": binding.subject(),
        })).collect::<Vec<_>>(),
        "events": record.events(),
    })
}

async fn list(State(state): State<Shared>, headers: HeaderMap) -> Result<Json<Value>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    state.admission.administrator(&actor)?;
    with_directory(&state, |directory| {
        let records = directory
            .projection()?
            .records()
            .map(|(id, record)| record_json(*id, record))
            .collect::<Vec<_>>();
        Ok(Json(json!({ "identities": records })))
    })
}

async fn read(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<Value>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    state.admission.administrator(&actor)?;
    let id = identity_id(&id)?;
    with_directory(&state, |directory| {
        let record =
            directory
                .record(id)?
                .ok_or_else(|| lys_identity::IdentityError::IdentityUnknown {
                    identity: id.to_string(),
                })?;
        Ok(Json(record_json(id, &record)))
    })
}

async fn change_profile(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<Named>,
) -> Result<Json<Value>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    state.admission.administrator(&actor)?;
    let (id, op, profile) = (
        identity_id(&id)?,
        operation(&body.operation)?,
        Profile::new(&body.display_name)?,
    );
    with_directory(&state, |directory| {
        let receipt = directory.change_profile(actor, op, id, profile, now())?;
        Ok(Json(json!({ "receipt": receipt_json(&receipt) })))
    })
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Moved {
    operation: String,
    transition: String,
    #[serde(default)]
    reason: String,
}

async fn transition(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<Moved>,
) -> Result<Json<Value>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    state.admission.administrator(&actor)?;
    let moved = match body.transition.as_str() {
        "activate" => Transition::Activate,
        "suspend" => Transition::Suspend,
        "reinstate" => Transition::Reinstate,
        "retire" => Transition::Retire,
        other => return Err(malformed(format!("{other} is not a transition"))),
    };
    let (id, op) = (identity_id(&id)?, operation(&body.operation)?);
    with_directory(&state, |directory| {
        let receipt = directory.transition(actor, op, id, moved, &body.reason, now())?;
        Ok(Json(json!({ "receipt": receipt_json(&receipt) })))
    })
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Bound {
    operation: String,
    issuer: String,
    subject: String,
}

async fn bind_login(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<Bound>,
) -> Result<Json<Value>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    state.admission.administrator(&actor)?;
    let person = PersonId::from_str(&id)?;
    let (op, binding) = (
        operation(&body.operation)?,
        LoginBinding::new(&body.issuer, &body.subject)?,
    );
    with_directory(&state, |directory| {
        let receipt = directory.bind_login(actor, op, person, binding, now())?;
        Ok(Json(json!({ "receipt": receipt_json(&receipt) })))
    })
}
