//! The HTTP routes mapping requests to the directory, each mutation behind admission.
//! The read-only views the identity screens draw are in `read_api`. The start
//! route and the door's handle records it reads are declared here, in
//! [`start`] and [`door_handles`].

/// The directory routes with work bound to the lifetime of their request.
pub fn router(state: Shared) -> axum::Router {
    crate::routes_table::router(state)
        .layer(axum::middleware::from_fn(crate::wait_request::observe))
}

use std::path::PathBuf;
use std::str::FromStr;
use std::sync::{Arc, Mutex};

use axum::Router;
use axum::http::{HeaderMap, header};
use lys_identity::{Actor, AgentId, Directory, IdentityId, PersonId};
use lys_log_store::FileLeafStore;

use lys_identity::signer::load_service_key;

use crate::admission::Admission;
use crate::config::Config;
use crate::error::ServerError;
use crate::grants::{GrantSetup, GrantState};
use crate::oidc::Oidc;
use crate::reviews_store::ReviewStore;
use crate::service_accounts_store::ServiceAccountStore;
use crate::session::Sessions;

#[path = "people_giving.rs"]
pub(crate) mod people_giving;

#[cfg(test)]
#[path = "people_giving_tests.rs"]
pub(crate) mod people_giving_tests;

/// Everything a request is served from.
pub struct AppState {
    /// In-process admitted draft application.
    pub draft_execution: crate::drafts_apply::Executor,
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
    /// Lys's password policy, when the configuration names it.
    pub password_policy: Option<crate::accounts::PasswordPolicy>,
    /// Lys's model proxy for Anthropic calls, given to each Claude Code run
    /// Lys starts as its base URL; absent, runs reach their provider directly.
    pub model_proxy: Option<String>,
    /// The model proxy address a run is given on a computer whose runner
    /// dials in: Lys's own public origin with the proxy's path, since the
    /// loopback address names nothing on that computer. Absent with no proxy.
    pub dialled_model_proxy: Option<String>,
    /// Where the model proxy on this machine keeps its records, read to
    /// show an agent's calls; absent, they cannot be shown.
    pub proxy_dir: Option<PathBuf>,
    /// Live sessions.
    pub sessions: Sessions,
    /// Who is admitted to what.
    pub admission: Admission,
    /// The grants, opened on first use once the root authority exists.
    pub grants: Mutex<Option<GrantState>>,
    /// Rights snapshots bounded to the active holder/app population.
    pub(crate) rights_cache: Mutex<crate::provider::rights_cache::Cache>,
    /// What the grants are opened from.
    pub grant_setup: GrantSetup,
    /// The secrets broker the secrets screens ask, when one is configured.
    pub secrets: Option<crate::secrets_api::SecretsBroker>,
    /// The sender letters go out through; none when the install has no
    /// mail settings (DIRECTORY-110).
    pub mailer: Option<crate::mail::Mailer>,
    /// The foundings waiting on their proof, kept beside the directory log
    /// (DIRECTORY-111); never held across an await.
    pub foundings: Mutex<crate::foundings_store::Foundings>,
    /// The pairings waiting on a person's decision, kept beside the
    /// directory log (AGENTS-020); never held across an await.
    pub pairings: Mutex<crate::pair_store::PairStore>,
    /// The access requests, when the configuration names their file.
    pub requests: Option<Mutex<crate::requests_store::RequestStore>>,
    /// Requests for declared MCP servers, recorded without changing provisioning.
    pub mcp_requests: Option<Box<Mutex<crate::mcp_requests_store::McpRequestStore>>>,
    /// The machines, when the configuration names their file.
    pub network: Option<Mutex<crate::network_store::NetworkStore>>,
    /// The digests of the computers' connection codes, kept beside the
    /// machines' file when the configuration names it.
    pub joins: Option<Mutex<crate::network_join::JoinStore>>,
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
    /// The cadence written by install or upgrade, validated before serving.
    pub review_period_days: u32,
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
    /// The master off switch, kept beside the directory log: always open,
    /// since every start asks it.
    pub cord: Mutex<crate::cord_store::CordStore>,
    /// Each person's canvas, one file for each beside the directory log,
    /// read when asked and made at the person's first change.
    pub canvas: crate::canvas_store::CanvasStore,
    /// Each person's getting-started answers, one file for each beside the
    /// directory log, read when asked and made at the person's first answer.
    pub getting_started: crate::getting_started_api::GettingStartedStore,
    /// The goals and their reminders, when the configuration names their directory.
    pub goals: Option<crate::goals_store::Goals>,
    /// The words a seat is sent (AGENTS-001 R1), when the configuration names their directory.
    pub words: Option<crate::words_store::WordsKept>,
    /// The agents' and sessions' variables (AGENTS-001 R2), when the configuration names their directory.
    pub variables: Option<crate::variables_store::VariablesKept>,
    /// The schedules (AGENTS-001 R4), when the configuration names their directory.
    pub schedules: Option<crate::schedules_store::SchedulesKept>,
    /// The apps, kept beside the grant log: always open, holding at least
    /// the app `lys`.
    pub apps: Mutex<crate::apps_store::AppStore>,
    /// Cached durable grant-bound credentials.
    pub grant_tokens: Mutex<crate::grant_token_store::Tokens>,
    /// Cached digests for the agent runs this install starts.
    pub agent_passes: Arc<Mutex<crate::agent_pass_store::Passes>>,
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
    /// The channel membership capability's page settings, kept cursors and
    /// work counts (ACCESS-006).
    pub membership: crate::channel_membership_counts::Membership,
    /// The seats (AGENTS-002), kept beside the directory log: always open.
    pub seats: Mutex<crate::seats_store::SeatStore>,
    /// The seats' imports (AGENTS-003), kept beside the directory log:
    /// always open, as the seats are.
    pub seat_imports: crate::seat_import_store::SeatImports,
    /// Each store's loss epoch and each own store still lost
    /// (DIRECTORY-107 R4), said in the refusals they cause.
    pub stores: crate::stores_receipt::Held,
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
///
/// # Errors
/// Refuses invalid configuration, directory and store opening or migration
/// failures, issuer discovery failures or startup admission and reconciliation
/// failures.
pub async fn service(config: &Config) -> Result<Router, ServerError> {
    Box::pin(service_saying(config, Arc::new(|_| {}))).await
}

/// As `service`, saying through `say` how each thing kept was started: the
/// directory log from its snapshot or from every leaf, each store with what
/// it read and how much it holds, and the grant log when the grants are
/// opened on their first use.
///
/// # Errors
/// Refuses invalid configuration, directory and store opening or migration
/// failures, issuer discovery failures or startup admission and reconciliation
/// failures.
pub async fn service_saying(config: &Config, say: Say) -> Result<Router, ServerError> {
    Box::pin(service_saying_with_clock(
        config,
        say,
        lys_core::clock::ClockSource::System,
    ))
    .await
}

/// Construct the actual service with its session and request clock.
///
/// # Errors
/// Propagates clock, configuration and store startup failures.
pub async fn service_with_clock(
    config: &Config,
    clock: lys_core::clock::ClockSource,
) -> Result<Router, ServerError> {
    service_saying_with_clock(config, Arc::new(|_| {}), clock).await
}

/// Construct the actual service with its clock and startup reporter.
///
/// # Errors
/// Propagates clock, configuration and store startup failures.
pub async fn service_saying_with_clock(
    config: &Config,
    say: Say,
    clock: lys_core::clock::ClockSource,
) -> Result<Router, ServerError> {
    Ok(Box::pin(started_saying_with_clock(config, say, clock))
        .await?
        .router)
}

/// The service as it started: its routes, the stores receipt's writer for a
/// clean stop, and the words of a start refused by a lost store
/// (DIRECTORY-107 R2), which then serves its health and screens alone.
///
/// # Errors
/// Propagates clock, configuration and store startup failures.
pub async fn started_saying_with_clock(
    config: &Config,
    say: Say,
    clock: lys_core::clock::ClockSource,
) -> Result<crate::stores_receipt::Started, ServerError> {
    Box::pin(crate::routes_startup::started_with_clock(
        config, say, clock,
    ))
    .await
}

/// Open the directory `config` names, creating its log when the log's
/// directory does not exist.
///
/// # Errors
/// Refuses log creation, an unreadable event key, directory migration or
/// opening failures, or retained directory state the service cannot read.
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
    if let Some(actor) = crate::drafts_apply::actor() {
        return Ok(actor);
    }
    // The person an app's pass admits as, while that one act runs (DIRECTORY-113 R1).
    if let Some(actor) = crate::people_admit_app::actor() {
        return Ok(actor);
    }
    if let Some(actor) = crate::mcp_oauth::native_actor(state, headers)? {
        return Ok(actor);
    }
    if let Some(actor) = crate::provider::personal_actor(state, headers)? {
        return Ok(actor);
    }
    if let Some((agent, provenance)) = crate::agent_pass::verified(state, headers)? {
        return Ok(Actor::new(
            lys_identity::LoginBinding::new(state.oidc.issuer(), &agent.to_string())?,
            provenance,
        ));
    }
    if let Some(actor) = crate::operator::actor(state, headers)? {
        return Ok(actor);
    }
    // An agent whose signed MCP message relayed this call, its route grant judged
    // before the handler (`route_actions::admit`): the agent itself, never a person.
    if let Some(agent) = crate::agent_signature::relayed_agent() {
        return Ok(Actor::new(
            lys_identity::LoginBinding::new(state.oidc.issuer(), &agent.to_string())?,
            lys_identity::Provenance::new(
                lys_identity::AuthMethod::AgentSignature(agent),
                crate::session::now(),
            ),
        ));
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

/// Admit a pass actor, or the agent a signed MCP message relayed this call for, whose
/// route grant was exercised by the guarded router, or the agent approver of a draft
/// Lys is applying, admitted at the approval. A request merely signed, not relayed
/// and judged, cannot borrow this exception to personal admission.
pub(crate) fn admitted_agent(
    directory: &lys_identity::projection::Projection,
    actor: &Actor,
) -> Result<bool, ServerError> {
    let relayed = match actor.provenance().method() {
        lys_identity::AuthMethod::AgentSignature(agent) => {
            crate::agent_signature::relayed_agent() == Some(agent)
                // The approver of a draft, carried by Lys's own application of
                // it (24 October 2026): admitted at the approval on its signed
                // decision, which the approval leaf keeps. Only drafts_apply
                // attaches draft provenance.
                || actor.draft().is_some()
        }
        _ => false,
    };
    if relayed
        || matches!(
            actor.provenance().method(),
            lys_identity::AuthMethod::AgentPass(_)
        )
    {
        crate::caller_admission::active_caller(directory, actor)?;
        return Ok(true);
    }
    Ok(false)
}

/// The agent `actor` is when the guarded router admitted it: its run pass,
/// or its relayed MCP message, with the route's declared act exercised
/// through its own grants before the handler (`route_actions::admit`). A
/// route an agent is given by its scope serves that agent within the act
/// the door judged, as it serves the administrator or the responsible
/// person (AGENTS-019 R4); none for every other caller.
///
/// # Errors
/// Refuses an inactive agent, or a directory that cannot be read.
pub(crate) fn door_agent(state: &AppState, actor: &Actor) -> Result<Option<AgentId>, ServerError> {
    with_directory(state, |directory| {
        Ok(admitted_agent(directory.projection()?, actor)?
            .then(|| actor.provenance().agent())
            .flatten())
    })
}

/// Whether the caller is an administrator, retaining operational refusals.
pub(crate) fn is_administrator(state: &AppState, actor: &Actor) -> Result<bool, ServerError> {
    with_directory(state, |directory| {
        state
            .admission
            .is_administrator(directory.projection()?, actor)
    })
}

/// Personal agent acts admit only that agent's responsible person or the administrator.
pub(crate) fn responsible_or_administrator(
    state: &AppState,
    headers: &HeaderMap,
    agent: &str,
) -> Result<(), ServerError> {
    let actor = signed_in(state, headers)?;
    with_directory(state, |directory| {
        let directory = directory.projection()?;
        if state.admission.is_administrator(directory, &actor)? {
            return Ok(());
        }
        let person = crate::caller_admission::admitted_person(directory, &actor)?;
        let agent = AgentId::from_str(agent)?;
        if directory
            .record(IdentityId::Agent(agent))
            .is_some_and(|record| record.responsible() == Some(person))
        {
            return Ok(());
        }
        Err(ServerError::NotAdmitted {
            reason: "only the responsible person or administrator may do this for the agent",
        })
    })
}

#[cfg(test)]
#[path = "routes/identities_self_tests.rs"]
mod identities_self_tests;

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

/// The identity `text` names, read by its prefix; a prefix that names no
/// kind of identity is refused by name (DIRECTORY-080 R1, box 12.5).
pub(crate) fn identity_id(text: &str) -> Result<IdentityId, ServerError> {
    let identity = match text.split_once('-').map(|(prefix, _)| prefix) {
        Some("op") => {
            lys_identity::ServiceAccountId::from_str(text).map(IdentityId::ServiceAccount)
        }
        Some("agent") => AgentId::from_str(text).map(IdentityId::Agent),
        Some("connector") => lys_identity::ConnectorId::from_str(text).map(IdentityId::Connector),
        Some("machine") => lys_identity::MachineId::from_str(text).map(IdentityId::Machine),
        Some("person") => PersonId::from_str(text).map(IdentityId::Person),
        Some("organisation") => {
            lys_identity::OrganisationId::from_str(text).map(IdentityId::Organisation)
        }
        _ => Err(lys_identity::IdentityError::IdentifierMalformed {
            kind: "person, agent, service account, connector, machine or organisation",
            text: text.to_owned(),
        }),
    };
    identity.map_err(ServerError::from)
}

mod identities;
mod identities_agent;
pub(crate) use identities::{
    AgentRegistration, Bound, Moved, Named, bind_login, change_profile, list, read, register_agent,
    register_person, transition,
};
