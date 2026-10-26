//! The start route: gives a start, gives it again from a kept launch record,
//! withdraws it and reads its state, each through the library's start
//! module, with no start logic of its own.
//!
//! - `POST /agents/{id}/start` with `{"profile_version": .., "machine": ..}`
//! - `POST /launch-records/{id}/start-again`
//! - `POST /launch-records/{id}/withdraw`
//! - `GET /launch-records/{id}/state`
//!
//! Every check, refusal, launch record and command the route answers is the
//! library's, in the library's own JSON: a given start answers its command,
//! its working directory and its launch record, and a refused one every
//! refusal by name in words. The request's members are handed over as they
//! came, so the library refuses a member it does not allow. Each owner's
//! record is read through its seam: the credentials check through
//! [`HandleRecords`] alone, whichever client the service is given. Nothing
//! here runs the command, holds a process or puts a credential value in an
//! answer.
//!
//! The service over the directory reads every other record from the store
//! this server already keeps for it, the same facts the start-command route
//! reads: the profile version, named by the operation it was set with as a
//! launch record names it, from the provisioning store; the machines each
//! role and the agent itself may run on, and each machine's egress list,
//! from the network store; and the sessions a launch record's start runs
//! as from the runtime reports.
//!
//! A service given a [`Launcher`] hands each given start to it, and the
//! launcher asks the machine's runner, when the machine names one, to run
//! it; the answer then carries the runner's word as its `runner` member. A
//! machine that names no runner is answered the command as before. Every
//! refusal of a start is the library's, unchanged.

use std::future::Future;
use std::path::Path;
use std::pin::Pin;
use std::str::FromStr;
use std::sync::{Arc, Mutex, MutexGuard};

use axum::Router;
use axum::body::Bytes;
use axum::extract::{Path as UrlPath, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use lys_core::Ed25519Identity;
use lys_identity::AgentId;
use lys_identity::start::active::Lifecycles;
use lys_identity::start::authority::Admission;
use lys_identity::start::credentials::HandleRecords;
use lys_identity::start::egress::{EgressLists, ProfileNeeds};
use lys_identity::start::machine_role::RoleMachines;
use lys_identity::start::profile_command::ProfileVersionRecords;
use lys_identity::start::profile_review::ProfileReviews;
use lys_identity::start::request::AgentRecords;
use lys_identity::start::state::SessionReports;
use lys_identity::start::{Given, Grammars, LaunchRecords, Owners, StartError, state_of, withdraw};
use serde_json::Value;

use crate::error::ServerError;
use crate::routes::AppState;

#[path = "start/run.rs"]
mod run;
#[path = "start/serialization.rs"]
mod serialization;
use run::{start, start_again};

#[path = "start_pass_state.rs"]
mod pass_state;

#[path = "start_directory.rs"]
mod directory;
#[path = "start_records.rs"]
mod records;

#[path = "start/held.rs"]
mod held;

use directory::Directory;

/// Who is asking, as the records a start keeps name them.
pub trait Callers: Send + Sync {
    /// Refuse unavailable admission state before a start reads authority.
    ///
    /// # Errors
    /// Implementations refuse unavailable admission state before authority is read.
    /// The default has no admission store to read and returns success.
    fn check_admission(&self) -> Result<(), ServerError> {
        Ok(())
    }

    /// The caller the request's headers authenticate, or `None`.
    fn caller(&self, headers: &HeaderMap) -> Option<String>;
}

/// Every owner a start reads, each through its own seam, shareable across
/// the route's requests.
pub struct StartOwners {
    /// The agent records DIRECTORY-011 keeps.
    pub agents: Box<dyn AgentRecords + Send + Sync>,
    /// The directory's admission.
    pub admission: Box<dyn Admission + Send + Sync>,
    /// The lifecycle record DIRECTORY-003 keeps.
    pub lifecycles: Box<dyn Lifecycles + Send + Sync>,
    /// The review record the roles card keeps.
    pub reviews: Box<dyn ProfileReviews + Send + Sync>,
    /// The role's machines in the roles card's record.
    pub role_machines: Box<dyn RoleMachines + Send + Sync>,
    /// The handle record SECRETS-002 keeps.
    pub handles: Box<dyn HandleRecords + Send + Sync>,
    /// What each profile version needs.
    pub needs: Box<dyn ProfileNeeds + Send + Sync>,
    /// The machines' egress lists.
    pub egress: Box<dyn EgressLists + Send + Sync>,
    /// The profile version record, read for the command.
    pub profiles: Box<dyn ProfileVersionRecords + Send + Sync>,
    /// The sessions record.
    pub sessions: Box<dyn SessionReports + Send + Sync>,
    /// The id grammars of the assigned values.
    pub grammars: Grammars,
}

impl StartOwners {
    /// The owners as the library reads them.
    #[must_use]
    pub fn owners(&self) -> Owners<'_> {
        Owners {
            agents: &*self.agents,
            admission: &*self.admission,
            lifecycles: &*self.lifecycles,
            reviews: &*self.reviews,
            role_machines: &*self.role_machines,
            handles: &*self.handles,
            needs: &*self.needs,
            egress: &*self.egress,
            profiles: &*self.profiles,
            sessions: &*self.sessions,
            grammars: self.grammars,
        }
    }
}

/// What a launcher answers: nothing when the machine names no runner, the
/// runner's word when it ran the start, or a refusal by name.
pub type Launched = Option<Result<Value, ServerError>>;

/// A launcher's answer, once the runner has given it.
pub type LaunchFuture<'a> = Pin<Box<dyn Future<Output = Launched> + Send + 'a>>;

/// Where a given start is run: the runner the machine's record names.
pub trait Launcher: Send + Sync {
    /// Ask the runner of the machine `given` names to run it for `caller`.
    fn launch<'a>(&'a self, given: &'a Given, caller: &'a str) -> LaunchFuture<'a>;
}

/// What the start route answers from.
pub struct StartService {
    owners: StartOwners,
    callers: Box<dyn Callers>,
    launches: Mutex<LaunchRecords>,
    clock: fn() -> u64,
    launcher: Option<Box<dyn Launcher>>,
    pass_state: Option<Arc<Mutex<crate::agent_pass_store::Passes>>>,
    kept: Option<Arc<AppState>>,
    starting: serialization::Starts,
}

impl StartService {
    /// The service over `owners`, knowing callers by `callers`, keeping
    /// launch records in `launches`, and telling the time by `clock`.
    pub fn new(
        owners: StartOwners,
        callers: Box<dyn Callers>,
        launches: LaunchRecords,
        clock: fn() -> u64,
    ) -> Self {
        Self {
            owners,
            callers,
            launches: Mutex::new(launches),
            clock,
            launcher: None,
            pass_state: None,
            kept: None,
            starting: serialization::Starts::default(),
        }
    }

    /// The service handing each given start to `launcher`.
    #[must_use]
    pub fn with_launcher(self, launcher: Box<dyn Launcher>) -> Self {
        Self {
            launcher: Some(launcher),
            ..self
        }
    }

    /// The service reading, before each start's checks, what the server
    /// `state` keeps: the profile version's agent and the caller's handles.
    #[must_use]
    fn with_kept(self, state: Arc<AppState>) -> Self {
        Self {
            kept: Some(state),
            ..self
        }
    }

    /// The owners as the library reads them.
    #[must_use]
    pub fn owners(&self) -> Owners<'_> {
        self.owners.owners()
    }

    /// The owners as the library reads them, reading the handles `broker`
    /// lists in place of the constructed handle record when it lists them.
    fn owners_with<'a>(&'a self, broker: Option<&'a records::Broker>) -> Owners<'a> {
        let mut owners = self.owners();
        if let Some(broker) = broker {
            owners.handles = broker;
        }
        owners
    }

    /// The launch records, one caller at a time.
    ///
    /// # Errors
    /// Refuses a poisoned launch records lock.
    pub fn launches(&self) -> Result<MutexGuard<'_, LaunchRecords>, StartError> {
        self.launches
            .lock()
            .map_err(|error| StartError::Unavailable {
                reason: format!("launch records unavailable: {error}"),
            })
    }
}

type Shared = Arc<StartService>;

/// The start route over `service`.
pub fn routes(service: Shared) -> Router {
    Router::new()
        .route("/agents/{id}/start", post(start))
        .route("/launch-records/{id}/start-again", post(start_again))
        .route("/launch-records/{id}/withdraw", post(withdrawn))
        .route("/launch-records/{id}/state", get(state))
        .with_state(service)
}

fn json(status: StatusCode, body: String) -> Response {
    (status, [(header::CONTENT_TYPE, "application/json")], body).into_response()
}

fn named(status: StatusCode, error: &str, words: &str) -> Response {
    json(status, named_body(error, words))
}

fn named_body(error: &str, words: &str) -> String {
    serde_json::json!({ "error": error, "words": words }).to_string()
}

fn status(error: &StartError) -> StatusCode {
    match error {
        StartError::Refused(refused) if refused.names("start_right_missing") => {
            StatusCode::FORBIDDEN
        }
        StartError::Refused(refused) if refused.names("agent_unknown") => StatusCode::NOT_FOUND,
        StartError::Refused(_) => StatusCode::CONFLICT,
        StartError::LaunchRecordUnknown { .. } => StatusCode::NOT_FOUND,
        StartError::Unavailable { .. } => StatusCode::SERVICE_UNAVAILABLE,
    }
}

/// Run `act` for the signed-in caller off the async workers, one caller at
/// a time on the launch records, and answer its JSON.
async fn answer<F>(service: Shared, headers: &HeaderMap, act: F) -> Response
where
    F: FnOnce(&StartService, &mut LaunchRecords, &str) -> Result<String, StartError>
        + Send
        + 'static,
{
    match answer_with(service, headers, act).await {
        Ok(body) => json(StatusCode::OK, body),
        Err((status, body)) => json(status, body),
    }
}

/// Run `act` as [`answer`] does, handing back what it made, or the answer
/// that refuses it.
async fn answer_with<F, T>(
    service: Shared,
    headers: &HeaderMap,
    act: F,
) -> Result<T, (StatusCode, String)>
where
    F: FnOnce(&StartService, &mut LaunchRecords, &str) -> Result<T, StartError> + Send + 'static,
    T: Send + 'static,
{
    service.callers.check_admission().map_err(|error| {
        (
            error.status(),
            named_body(&error.name(), &error.to_string()),
        )
    })?;
    let Some(caller) = service.callers.caller(headers) else {
        return Err((
            StatusCode::UNAUTHORIZED,
            named_body("not_signed_in", "sign in to start an agent"),
        ));
    };
    let task = tokio::task::spawn_blocking(move || {
        let mut launches = service.launches()?;
        act(&service, &mut launches, &caller)
    });
    match task.await {
        Ok(Ok(made)) => Ok(made),
        Ok(Err(error)) => Err((status(&error), error.to_json())),
        Err(failed) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            named_body("start_task_failed", &failed.to_string()),
        )),
    }
}

/// The request's members as they came, each value as its text, then the
/// agent the path names; `None` when the body is not a JSON object.
fn members(agent: &str, body: &Bytes) -> Option<Vec<(String, String)>> {
    let parsed = if body.is_empty() {
        Value::Object(serde_json::Map::new())
    } else {
        serde_json::from_slice(body).ok()?
    };
    let Value::Object(object) = parsed else {
        return None;
    };
    let mut members: Vec<(String, String)> = object
        .into_iter()
        .map(|(name, value)| match value {
            Value::String(text) => (name, text),
            other => (name, other.to_string()),
        })
        .collect();
    members.push(("agent".to_owned(), agent.to_owned()));
    Some(members)
}

async fn withdrawn(
    State(service): State<Shared>,
    headers: HeaderMap,
    UrlPath(launch_record): UrlPath<String>,
) -> Response {
    answer(service, &headers, move |service, launches, caller| {
        withdraw(
            launches,
            &service.owners(),
            caller,
            &launch_record,
            (service.clock)(),
        )
        .and_then(|state| {
            crate::agent_pass_recovery::withdraw(service.pass_state.as_deref(), &launch_record)?;
            Ok(state.to_json())
        })
    })
    .await
}

async fn state(
    State(service): State<Shared>,
    headers: HeaderMap,
    UrlPath(launch_record): UrlPath<String>,
) -> Response {
    answer(service, &headers, move |service, launches, _| {
        state_of(launches, &launch_record, service.owners().sessions).map(|state| state.to_json())
    })
    .await
}

fn agent_id(text: &str) -> bool {
    AgentId::from_str(text).is_ok()
}

/// The start service over the directory `state` holds, reading the handle
/// record through `handles`, whose credential ids are held to
/// `credential_id`, and keeping launch records in `dir`, signed by `key`.
/// Reviews, profile versions, what each needs, the machines each role may
/// run on, egress lists and session reports are read from the stores
/// `state` keeps.
///
/// # Errors
/// Refuses launch records that cannot be opened and verified.
pub fn directory_service(
    state: &Arc<AppState>,
    handles: Box<dyn HandleRecords + Send + Sync>,
    credential_id: fn(&str) -> bool,
    dir: &Path,
    key: Ed25519Identity,
) -> Result<Shared, ServerError> {
    let launches = LaunchRecords::open(dir, key).map_err(|error| ServerError::ConfigInvalid {
        reason: format!("the launch records could not be opened: {error}"),
    })?;
    (state.say)(&format!(
        "launch-record log {}, holding {} launch records",
        launches.start(),
        launches.record_count()
    ));
    let owners = StartOwners {
        agents: Box::new(Directory(Arc::clone(state))),
        admission: Box::new(Directory(Arc::clone(state))),
        lifecycles: Box::new(Directory(Arc::clone(state))),
        reviews: Box::new(records::Kept(Arc::clone(state))),
        role_machines: Box::new(records::Kept(Arc::clone(state))),
        handles,
        needs: Box::new(records::Kept(Arc::clone(state))),
        egress: Box::new(records::Kept(Arc::clone(state))),
        profiles: Box::new(records::Kept(Arc::clone(state))),
        sessions: Box::new(records::Kept(Arc::clone(state))),
        grammars: Grammars {
            agent_id,
            credential_id,
        },
    };
    let service = StartService::new_with_passes(
        owners,
        Box::new(Directory(Arc::clone(state))),
        launches,
        crate::session::now,
        Arc::clone(&state.agent_passes),
    );
    let runs = crate::runner_sessions::DirectoryLauncher(Arc::clone(state));
    Ok(Arc::new(
        service
            .with_launcher(Box::new(runs))
            .with_kept(Arc::clone(state)),
    ))
}

/// The start route's service over the directory `state` holds. The route
/// reads the handle record through the door's client, [`crate::routes::door_handles::DoorHandles`],
/// as its `HandleRecords`. No configuration field names the door yet, so the
/// client is built without an address and answers that no handle record
/// exists, naming SECRETS-002, until one does. Launch records are kept in
/// `launch-records` beside the directory log.
pub(crate) fn configured_service(
    config: &crate::config::Config,
    state: &crate::routes::Shared,
) -> Result<Arc<StartService>, ServerError> {
    let handles = crate::routes::door_handles::DoorHandles::unconfigured();
    let dir = config.log_dir.with_file_name("launch-records");
    directory_service(
        state,
        Box::new(handles),
        crate::routes::door_handles::credential_id,
        &dir,
        lys_identity::signer::load_service_key(&config.event_key_file)?,
    )
}
