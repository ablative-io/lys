//! The HTTP routes mapping requests to the directory, each mutation behind admission.
//! The read-only views the identity screens draw are in `read_api`. The start
//! route and the door's handle records it reads are declared here, in
//! [`start`] and [`door_handles`].

use std::str::FromStr;
use std::sync::{Arc, Mutex, PoisonError};

use axum::extract::{Path, State};
use axum::http::{HeaderMap, header};
use axum::routing::{get, post};
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

/// Everything a request is served from.
pub struct AppState {
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

type Shared = Arc<AppState>;

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
    let operator_token = crate::operator::token(config, &*say)?;
    let mut directory = open_directory(config)?;
    say(&format!("directory log {}", directory.log()?.start()));
    let key = Arc::new(load_service_key(&config.event_key_file)?);
    let requests = crate::requests_store::RequestStore::opened(config, Arc::clone(&key), &*say)?;
    let certificates = config
        .certificates_dir
        .as_deref()
        .map(|dir| {
            crate::certificates_store::CertificateStore::opened(dir, Arc::clone(&key), &*say)
        })
        .transpose()?;
    let (network, roles, provisioning) = crate::file_stores::opened(config, &*say)?;
    let runtime = crate::runtime_store::RuntimeStore::configured(config, &say)?;
    let service_accounts = ServiceAccountStore::configured(config, Arc::clone(&key), &say)?;
    let reviews = ReviewStore::configured(config, Arc::clone(&key), &*say)?;
    let teams = crate::teams_store::TeamStore::configured(config, Arc::clone(&key), &say)?;
    let stops = crate::stops_store::StopStore::configured(config, Arc::clone(&key), &say)?;
    let budgets = crate::budgets_store::BudgetStore::configured(config, Arc::clone(&key), &say)?;
    let policies =
        crate::agent_policy_store::PolicyStore::configured(config, Arc::clone(&key), &say)?;
    let goals = crate::goals_store::GoalStore::configured(config, Arc::clone(&key), &say)?;
    let acts = crate::runner_acts::ActStore::open(
        &config.log_dir.with_file_name("runner-acts"),
        Arc::clone(&key),
    )?;
    say(&format!(
        "runner-acts log {}, holding {} acts",
        acts.start(),
        acts.len()
    ));
    let apps = crate::apps_api::opened(config, Arc::clone(&key), &*say)?;
    let model = apps.model()?;
    let state = Arc::new(AppState {
        directory: Mutex::new(directory),
        oidc: Oidc::discover(config).await?,
        sign_in: crate::sign_in::IssuerSignIn::configured(config)?,
        provider: config
            .provider
            .as_ref()
            .map(|settings| {
                crate::provider::OpenIdProvider::open(settings, crate::sign_in::lys_origin(config)?)
            })
            .transpose()?,
        setup: config.setup.clone(),
        setup_lock: tokio::sync::Mutex::new(()),
        sessions: match &config.sessions_file {
            Some(file) => {
                Sessions::open(file.clone(), config.session_seconds, config.secure_cookie)?
            }
            None => Sessions::new(config.session_seconds, config.secure_cookie),
        },
        admission: Admission::new(
            crate::setup::administrator(config)?,
            config.link_audit_binding()?,
        ),
        grants: Mutex::new(None),
        grant_setup: GrantSetup {
            log_dir: config.grant_log_dir.clone(),
            log_origin: config.grant_log_origin.clone(),
            key_file: config.event_key_file.clone(),
            model: std::sync::RwLock::new(model),
            spicedb: config.spicedb.clone(),
        },
        secrets: config
            .secrets
            .as_ref()
            .map(crate::secrets_api::SecretsBroker::open)
            .transpose()?,
        requests: requests.map(Mutex::new),
        network: network.map(Mutex::new),
        roles: roles.map(Mutex::new),
        provisioning: provisioning.map(Mutex::new),
        certificates: certificates.map(Mutex::new),
        runtime: runtime.map(Mutex::new),
        service_accounts: service_accounts.map(Mutex::new),
        reviews: reviews.map(Mutex::new),
        teams: teams.map(Mutex::new),
        stops: stops.map(Mutex::new),
        budgets: budgets.map(Mutex::new),
        policies: policies.map(Mutex::new),
        goals: goals.map(crate::goals_store::Goals::new),
        apps: Mutex::new(apps),
        benches: crate::apps_bench::Benches::new(
            config.apps_dir().with_file_name("benches"),
            &*say,
        )?,
        sign_in_providers: config
            .sign_in_providers
            .as_ref()
            .map(|settings| {
                crate::sign_in_providers::SignInProviders::open(
                    settings,
                    config.provider_origins.clone(),
                )
            })
            .transpose()?,
        agent_nonces: Mutex::default(),
        operator_token,
        operator_upgrade_file: config.operator_upgrade_file.clone(),
        runners: crate::runner_client::Runners::new(key, config.runner_socket.clone()),
        acts: Mutex::new(acts),
        say,
    });
    crate::teams_migration::at_start(&state)?;
    crate::goals_api::remind_from(&state);
    crate::budgets_act::settle_at_start(&state);
    crate::refusals_follow::follow_at_start(&state);
    crate::grants_refusals::hold_at_start(&state);
    let configured = crate::configuration_api::routes(config)
        .merge(crate::message_edges::routes(config)?)
        .merge(crate::memory_api::routes(config))
        .merge(crate::certificates_api::routes())
        .with_state(Arc::clone(&state));
    let starts = start::routes(start_service(config, &state)?);
    let provider_callback = crate::sign_in::callback_routes(Arc::clone(&state))
        .merge(crate::provider::routes(Arc::clone(&state)));
    let api = router(Arc::clone(&state)).merge(configured).merge(starts);
    let served = match &config.surface_dir {
        Some(dir) => crate::surface::serving(dir.clone(), api),
        None => api,
    };
    Ok(served
        .merge(provider_callback)
        .layer(axum::middleware::from_fn_with_state(
            Arc::clone(&state),
            crate::session_admission::guard,
        ))
        .layer(axum::middleware::from_fn_with_state(
            state,
            crate::operator::guard,
        )))
}

/// The start route's service over the directory `state` holds. The route
/// reads the handle record through the door's client, [`door_handles::DoorHandles`],
/// as its `HandleRecords`. No configuration field names the door yet, so the
/// client is built without an address and answers that no handle record
/// exists, naming SECRETS-002, until one does. Launch records are kept in
/// `launch-records` beside the directory log.
fn start_service(config: &Config, state: &Shared) -> Result<Arc<start::StartService>, ServerError> {
    let handles = door_handles::DoorHandles::unconfigured();
    let dir = config.log_dir.with_file_name("launch-records");
    start::directory_service(
        state,
        Box::new(handles),
        door_handles::credential_id,
        &dir,
        load_service_key(&config.event_key_file)?,
    )
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
        .route("/callback", get(crate::sign_in_callback::callback))
        .route("/people", post(register_person))
        .route("/agents", post(register_agent))
        .route("/identities", get(list))
        .route("/identities/{id}", get(read))
        .route("/identities/{id}/profile", post(change_profile))
        .route("/identities/{id}/transitions", post(transition))
        .route("/people/{id}/logins", post(bind_login))
        .merge(crate::sign_in::routes())
        .merge(crate::setup::routes())
        .merge(crate::accounts::routes())
        .merge(crate::read_api::routes())
        .merge(crate::grants::routes())
        .merge(crate::receipts_api::routes())
        .merge(crate::reviews_api::routes())
        .merge(crate::roles_api::routes())
        .merge(crate::requests_api::routes())
        .merge(crate::connections_api::routes())
        .merge(crate::sign_in_providers::routes())
        .merge(crate::link_audit_api::routes())
        .merge(crate::network_api::routes())
        .merge(crate::provisioning_api::routes())
        .merge(crate::launch_api::routes())
        .merge(crate::runtime_api::routes())
        .merge(crate::runner_api::routes())
        .merge(crate::stop_api::routes())
        .merge(crate::budgets_api::routes())
        .merge(crate::budgets_act::routes())
        .merge(crate::agent_policy_api::routes())
        .merge(crate::refusals_api::routes())
        .merge(crate::goals_api::routes())
        .merge(crate::service_accounts_api::routes())
        .merge(crate::teams_api::routes())
        .merge(crate::resources_api::routes())
        .merge(crate::secrets_api::routes())
        .merge(crate::sessions_api::routes())
        .merge(crate::apps_api::routes())
        .merge(crate::apps_schema_api::routes())
        .merge(crate::apps_bench::routes())
        .merge(crate::openapi::routes())
        .with_state(state)
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
    if let Some(actor) = crate::operator::actor(state, headers)? {
        return Ok(actor);
    }
    state.sessions.actor(cookie_header(headers))
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

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = NamedBody)]
pub(crate) struct Named {
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
) -> Result<Json<PersonRegistered>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    state.admission.administrator(&actor)?;
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

async fn register_agent(
    State(state): State<Shared>,
    headers: HeaderMap,
    Json(body): Json<Named>,
) -> Result<Json<AgentRegistered>, ServerError> {
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
        Ok(Json(AgentRegistered {
            agent: id.to_string(),
            responsible: responsible.to_string(),
            receipt: receipt_view(&receipt),
        }))
    })
}

async fn list(
    State(state): State<Shared>,
    headers: HeaderMap,
) -> Result<Json<IdentitiesView>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    state.admission.administrator(&actor)?;
    with_directory(&state, |directory| {
        let identities = directory
            .projection()?
            .records()
            .map(|(id, record)| record_view(*id, record))
            .collect::<Vec<_>>();
        Ok(Json(IdentitiesView { identities }))
    })
}

async fn read(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<IdentityRecordView>, ServerError> {
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
        Ok(Json(record_view(id, &record)))
    })
}

async fn change_profile(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<Named>,
) -> Result<Json<ReceiptAnswer>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    state.admission.administrator(&actor)?;
    let (id, op, profile) = (
        identity_id(&id)?,
        operation(&body.operation)?,
        Profile::new(&body.display_name)?,
    );
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

async fn transition(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<Moved>,
) -> Result<Json<ReceiptAnswer>, ServerError> {
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

async fn bind_login(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<Bound>,
) -> Result<Json<ReceiptAnswer>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    state.admission.administrator(&actor)?;
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
