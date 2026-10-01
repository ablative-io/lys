//! The HTTP routes mapping requests to the directory, each mutation behind admission.
//! The read-only views the identity screens draw are in `read_api`. The start
//! route and the door's handle records it reads are declared here, in
//! [`start`] and [`door_handles`].

pub use crate::routes_table::router;

use std::path::PathBuf;
use std::str::FromStr;
use std::sync::{Arc, Mutex};

use axum::body::Bytes;
use axum::extract::rejection::BytesRejection;
use axum::extract::{OriginalUri, Path, State};
use axum::http::{HeaderMap, header};
use axum::{Json, Router};
use lys_identity::{
    Actor, AgentId, Directory, IdentityId, LoginBinding, OperationId, PersonId, Profile, Transition,
};
use lys_log_store::FileLeafStore;
use serde::Deserialize;

use lys_identity::signer::load_service_key;

use crate::admission::Admission;
use crate::config::Config;
use crate::directory_views::{
    AgentRegistered, IdentitiesView, IdentityRecordView, PersonRegistered, ReceiptAnswer,
    receipt_view, record_view,
};
use crate::error::ServerError;
use crate::grants::{GrantSetup, GrantState};
use crate::oidc::Oidc;
use crate::reviews_store::ReviewStore;
use crate::service_accounts_store::ServiceAccountStore;
use crate::session::{Sessions, now};

#[path = "people_giving.rs"]
pub(crate) mod people_giving;

#[cfg(test)]
#[path = "people_giving_tests.rs"]
pub(crate) mod people_giving_tests;

/// Everything a request is served from.
pub struct AppState {
    pub(crate) changes: crate::changes::Changes,
    /// The directory, one caller at a time.
    pub directory: Mutex<Directory<FileLeafStore>>,
    /// Sign-in.
    pub oidc: Oidc,
    /// The issuer's own sign-in, carried server-side from Lys's sign-in page.
    pub sign_in: crate::sign_in::IssuerSignIn,
    /// Lys's `OpenID` provider, when the configuration names it.
    pub provider: Option<crate::provider::OpenIdProvider>,
    /// First-run setup, when the configuration names it.
    pub setup: Option<crate::setup::SetupSettings>,
    /// One setup act at a time, so one code makes one administrator.
    pub setup_lock: tokio::sync::Mutex<()>,
    /// The private loader credential provisioned by install or upgrade.
    pub import_credential_file: Option<PathBuf>,
    /// The fixed local destination for app credentials held by the broker.
    pub identity_upstream: String,
    /// The estate plan placed beside the directory log by install or upgrade.
    pub estate_plan_file: PathBuf,
    /// Live sessions.
    pub sessions: Sessions,
    /// Who is admitted to what.
    pub admission: Admission,
    /// The grants, opened on first use once the root authority exists.
    pub grants: Mutex<Option<GrantState>>,
    /// What the grants are opened from.
    pub grant_setup: GrantSetup,
    /// The secrets broker the secrets screens ask, when one is configured.
    pub secrets: Option<crate::secrets_api::SecretsBroker>,
    /// The access requests, when the configuration names their file.
    pub requests: Option<Mutex<crate::requests_store::RequestStore>>,
    /// Requests for declared MCP servers, recorded without changing provisioning.
    pub mcp_requests: Option<Box<Mutex<crate::mcp_requests_store::McpRequestStore>>>,
    /// The machines, when the configuration names their file.
    pub network: Option<Mutex<crate::network_store::NetworkStore>>,
    /// The roles, when the configuration names their file.
    pub roles: Option<Mutex<crate::roles_store::RolesStore>>,
    /// The provisioning profiles, when the configuration names their file.
    pub provisioning: Option<Mutex<crate::provisioning_store::ProvisioningStore>>,
    /// The certificate log, when the configuration names its directory.
    pub certificates: Option<Mutex<crate::certificates_store::CertificateStore>>,
    /// The runtime reports, when the configuration names their directory.
    pub runtime: Option<Mutex<crate::runtime_store::RuntimeStore>>,
    /// The service accounts, when the configuration names their directory.
    pub service_accounts: Option<Mutex<ServiceAccountStore>>,
    /// The review decisions, when the configuration names their directory.
    pub reviews: Option<Mutex<ReviewStore>>,
    /// The teams, when the configuration names their directory.
    pub teams: Option<Mutex<crate::teams_store::TeamStore>>,
    /// The budgets, when the configuration names their directory.
    pub budgets: Option<Mutex<crate::budgets_store::BudgetStore>>,
    /// The persisted organisation settings.
    pub configuration: Mutex<Box<crate::configuration_store::ConfigurationStore>>,
    /// The agents' tool-boundary policies, when the configuration names
    /// their directory.
    pub policies: Option<Mutex<crate::agent_policy_store::PolicyStore>>,
    /// The emergency stops, when the configuration names their directory.
    pub stops: Option<Mutex<crate::stops_store::StopStore>>,
    /// The goals and their reminders, when the configuration names their directory.
    pub goals: Option<crate::goals_store::Goals>,
    /// The apps, kept beside the grant log: always open, holding at least
    /// the app `lys`.
    pub apps: Mutex<crate::apps_store::AppStore>,
    /// Cached durable grant-bound credentials.
    pub grant_tokens: Mutex<crate::grant_token_store::Tokens>,
    /// Cached digests for the agent runs this install starts.
    pub agent_passes: Mutex<crate::agent_pass_store::Passes>,
    /// Responsibilities loaded from the deployment data before serving.
    pub(crate) kept_responsibilities: crate::kept_responsibilities::Kept,
    /// The schema builder's test benches, each a throwaway draft.
    pub benches: crate::apps_bench::Benches,
    /// The issuer's administration API the sign-in providers are set
    /// through, when the configuration names it.
    pub sign_in_providers: Option<crate::sign_in_providers::SignInProviders>,
    /// The nonces agents' signed requests carried within the last minute.
    pub agent_nonces: crate::agent_signature::Nonces,
    /// The install's operator token, when the configuration names its file.
    pub operator_token: Option<zeroize::Zeroizing<String>>,
    /// The managed install's durable upgrade intent, checked on each operator request.
    pub operator_upgrade_file: Option<std::path::PathBuf>,
    /// How machines' runners are reached.
    pub runners: crate::runner_client::Runners,
    /// The acts on sessions through a runner, each kept as its receipt.
    pub acts: Mutex<crate::runner_acts::ActStore>,
    /// Where the service says how a thing it keeps was started.
    pub say: Say,
}

/// Where the service says how a thing it keeps was started.
pub type Say = Arc<dyn Fn(&str) + Send + Sync>;

pub(crate) type Shared = Arc<AppState>;

#[path = "door_handles.rs"]
pub mod door_handles;
#[path = "start.rs"]
pub mod start;

/// Open the directory, discover the issuer and answer the service's routes,
/// as `config` says. The log is created when its directory does not exist.
pub async fn service(config: &Config) -> Result<Router, ServerError> {
    service_saying(config, Arc::new(|_| {})).await
}

/// As `service`, saying through `say` how each thing kept was started: the
/// directory log from its snapshot or from every leaf, each store with what
/// it read and how much it holds, and the grant log when the grants are
/// opened on their first use.
pub async fn service_saying(config: &Config, say: Say) -> Result<Router, ServerError> {
    crate::routes_startup::service_saying(config, say).await
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
    let key = load_service_key(&config.event_key_file)?;
    let store = FileLeafStore::open(&log_dir).map_err(|error| ServerError::ConfigInvalid {
        reason: format!("the directory migration could not open the log: {error}"),
    })?;
    lys_identity::directory_migration::migrate(store, &key)?;
    let reopen = Box::new(move || FileLeafStore::open(&log_dir));
    let mut directory = Directory::open(reopen, key)?;
    if let Some(from) = &config.issuer_moved_from
        && directory.record_issuer_move(
            from,
            &config.issuer,
            crate::read_api::BUILD,
            crate::session::now(),
        )?
    {
        println!(
            "lys-identity-server recorded the issuer move from {from} to {}",
            config.issuer
        );
    }
    Ok(directory)
}

fn malformed(reason: String) -> ServerError {
    ServerError::RequestMalformed { reason }
}

/// The request's Cookie header, when it carries one that is text.
pub(crate) fn cookie_header(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(header::COOKIE)
        .and_then(|value| value.to_str().ok())
}

/// The signed-in actor, or a refusal: the administrator for a request
/// carrying the install's operator token, else the session's actor.
pub(crate) fn signed_in(state: &AppState, headers: &HeaderMap) -> Result<Actor, ServerError> {
    if let Some((agent, provenance)) = crate::agent_pass::verified(state, headers)? {
        return Ok(Actor::new(
            lys_identity::LoginBinding::new(state.oidc.issuer(), &agent.to_string())?,
            provenance,
        ));
    }
    if let Some(actor) = crate::operator::actor(state, headers)? {
        return Ok(actor);
    }
    state.sessions.actor(cookie_header(headers))
}

/// Require an active administrator using the current directory projection.
pub(crate) fn administrator(state: &AppState, actor: &Actor) -> Result<(), ServerError> {
    with_directory(state, |directory| {
        if admitted_agent(directory.projection()?, actor)? {
            return Ok(());
        }
        state
            .admission
            .administrator(directory.projection()?, actor)
    })
}

/// Admit a pass actor whose route grant was exercised by the guarded router.
/// A signed agent cannot borrow this exception to personal admission.
pub(crate) fn admitted_agent(
    directory: &lys_identity::projection::Projection,
    actor: &Actor,
) -> Result<bool, ServerError> {
    if matches!(
        actor.provenance().method(),
        lys_identity::AuthMethod::AgentPass(_)
    ) {
        crate::caller_admission::active_caller(directory, actor)?;
        return Ok(true);
    }
    Ok(false)
}

/// Whether the caller is an administrator, retaining operational refusals.
pub(crate) fn is_administrator(state: &AppState, actor: &Actor) -> Result<bool, ServerError> {
    with_directory(state, |directory| {
        state
            .admission
            .is_administrator(directory.projection()?, actor)
    })
}

/// Run `act` on the directory, one caller at a time.
pub(crate) fn with_directory<T>(
    state: &AppState,
    act: impl FnOnce(&mut Directory<FileLeafStore>) -> Result<T, ServerError>,
) -> Result<T, ServerError> {
    let mut directory =
        state
            .directory
            .lock()
            .map_err(|error| ServerError::DirectoryUnavailable {
                reason: format!("the directory lock is poisoned: {error}"),
            })?;
    act(&mut directory)
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
    if text.starts_with("op-") {
        return lys_identity::ServiceAccountId::from_str(text)
            .map(IdentityId::ServiceAccount)
            .map_err(ServerError::from);
    }
    if text.starts_with("agent-") {
        return AgentId::from_str(text)
            .map(IdentityId::Agent)
            .map_err(ServerError::from);
    }
    PersonId::from_str(text)
        .map(IdentityId::Person)
        .map_err(ServerError::from)
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = NamedBody)]
pub(crate) struct Named {
    operation: String,
    display_name: String,
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = AgentRegistrationBody)]
pub(crate) struct AgentRegistration {
    operation: String,
    display_name: String,
    #[serde(default)]
    answers_to: Option<String>,
}

fn registration_target(
    own: PersonId,
    requested: Option<&str>,
    may_choose: bool,
) -> Result<IdentityId, ServerError> {
    let target = requested.map_or(Ok(IdentityId::Person(own)), identity_id)?;
    if target != IdentityId::Person(own) && !may_choose {
        return Err(ServerError::NotAdmitted {
            reason: "only an administrator may register an agent under another person",
        });
    }
    Ok(target)
}

fn operation(text: &str) -> Result<OperationId, ServerError> {
    OperationId::from_str(text).map_err(ServerError::from)
}

pub(crate) async fn register_person(
    State(state): State<Shared>,
    headers: HeaderMap,
    Json(body): Json<Named>,
) -> Result<Json<PersonRegistered>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    crate::routes::administrator(&state, &actor)?;
    let (op, profile) = (
        operation(&body.operation)?,
        Profile::new(&body.display_name)?,
    );
    with_directory(&state, |directory| {
        let (id, receipt) = directory.register_person(actor, op, profile, now())?;
        Ok(Json(PersonRegistered {
            person: id.to_string(),
            receipt: receipt_view(&receipt),
        }))
    })
}

pub(crate) async fn register_agent(
    State(state): State<Shared>,
    headers: HeaderMap,
    Json(body): Json<AgentRegistration>,
) -> Result<Json<AgentRegistered>, ServerError> {
    if !headers.contains_key(axum::http::header::AUTHORIZATION) {
        let actor = signed_in(&state, &headers)?;
        crate::routes::administrator(&state, &actor)?;
        let (op, profile) = (
            operation(&body.operation)?,
            Profile::new(&body.display_name)?,
        );
        return with_directory(&state, |directory| {
            let projection = directory.projection()?;
            let own = match crate::caller_admission::active_caller(projection, &actor)? {
                IdentityId::Person(person) => person,
                IdentityId::Agent(_) | IdentityId::ServiceAccount(_) => {
                    return Err(ServerError::NotAdmitted {
                        reason: "an agent registration requires a person or an admitted service account",
                    });
                }
            };
            let target = registration_target(own, body.answers_to.as_deref(), true)?;
            crate::agent_policy_api::with_policies(&state, |policies| {
                let answer =
                    directory.register_reporting_agent(actor, op, target, profile, now())?;
                policies.ensure_default(&answer.agent.to_string())?;
                crate::reporting_api::registered(&answer).map(Json)
            })
        });
    }
    let (op, profile) = (
        operation(&body.operation)?,
        Profile::new(&body.display_name)?,
    );
    crate::grants::with_directory_grants(
        &state,
        |mut judged| {
            let caller = crate::service_account_grants::caller(&state, &headers, &judged)?;
            crate::service_account_grants::admit(&mut judged, caller, "agents")?;
            let (actor, responsible) = crate::service_account_grants::actor(&judged, caller)?;
            let target = registration_target(responsible, body.answers_to.as_deref(), false)?;
            Ok((actor, target))
        },
        |directory, (actor, target)| {
            crate::agent_policy_api::with_policies(&state, |policies| {
                let answer =
                    directory.register_reporting_agent(actor, op, target, profile, now())?;
                policies.ensure_default(&answer.agent.to_string())?;
                crate::reporting_api::registered(&answer).map(Json)
            })
        },
    )
}

pub(crate) async fn list(
    State(state): State<Shared>,
    headers: HeaderMap,
) -> Result<Json<IdentitiesView>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    crate::routes::administrator(&state, &actor)?;
    with_directory(&state, |directory| {
        let identities = directory
            .projection()?
            .records()
            .map(|(id, record)| record_view(*id, record))
            .collect::<Vec<_>>();
        Ok(Json(IdentitiesView { identities }))
    })
}

pub(crate) async fn read(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<IdentityRecordView>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    crate::routes::administrator(&state, &actor)?;
    let id = identity_id(&id)?;
    with_directory(&state, |directory| {
        let record =
            directory
                .record(id)?
                .ok_or_else(|| lys_identity::IdentityError::IdentityUnknown {
                    identity: id.to_string(),
                })?;
        Ok(Json(record_view(id, &record)))
    })
}

pub(crate) async fn change_profile(
    State(state): State<Shared>,
    headers: HeaderMap,
    OriginalUri(uri): OriginalUri,
    Path(id): Path<String>,
    principal: Option<axum::Extension<crate::agent_signature::TokenPrincipal>>,
    body: Result<Bytes, BytesRejection>,
) -> Result<Json<ReceiptAnswer>, ServerError> {
    let bytes = body.map_err(|error| malformed(error.body_text()))?;
    let actor = people_giving::actor(
        &state,
        &headers,
        ("POST", uri.path(), &bytes),
        principal.as_ref().map(|value| &value.0),
    )?;
    if actor.provenance().agent().is_none() {
        crate::routes::administrator(&state, &actor)?;
    }
    let body = people_giving::body(headers, bytes).await?;
    let (id, op, profile) = (
        identity_id(&id)?,
        operation(&body.operation)?,
        Profile::new(&body.display_name)?,
    );
    if actor.provenance().agent().is_some() {
        return people_giving::profile(&state, &actor, id, op, profile).map(Json);
    }
    with_directory(&state, |directory| {
        let receipt = directory.change_profile(actor, op, id, profile, now())?;
        Ok(Json(ReceiptAnswer {
            receipt: receipt_view(&receipt),
        }))
    })
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = TransitionBody)]
pub(crate) struct Moved {
    operation: String,
    transition: String,
    #[serde(default)]
    reason: String,
}

pub(crate) async fn transition(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<Moved>,
) -> Result<Json<ReceiptAnswer>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    crate::routes::administrator(&state, &actor)?;
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
        if matches!(moved, Transition::Suspend | Transition::Retire) {
            let projection = directory.projection()?;
            state.sessions.end_matching(|actor| {
                projection
                    .person_for(actor.binding())
                    .map(IdentityId::Person)
                    == Some(id)
                    || projection.agent_for(actor.binding()).map(IdentityId::Agent) == Some(id)
                    || actor.provenance().agent().map(IdentityId::Agent) == Some(id)
            })?;
        }
        Ok(Json(ReceiptAnswer {
            receipt: receipt_view(&receipt),
        }))
    })
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = BindLoginBody)]
pub(crate) struct Bound {
    operation: String,
    issuer: String,
    subject: String,
}

pub(crate) async fn bind_login(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<Bound>,
) -> Result<Json<ReceiptAnswer>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    crate::routes::administrator(&state, &actor)?;
    let person = PersonId::from_str(&id)?;
    let (op, binding) = (
        operation(&body.operation)?,
        LoginBinding::new(&body.issuer, &body.subject)?,
    );
    with_directory(&state, |directory| {
        let receipt = directory.bind_login(actor, op, person, binding, now())?;
        Ok(Json(ReceiptAnswer {
            receipt: receipt_view(&receipt),
        }))
    })
}
