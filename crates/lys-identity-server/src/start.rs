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
//! A service given a [`Launcher`] hands each given start to it, and the
//! launcher asks the machine's runner, when the machine names one, to run
//! it; the answer then carries the runner's word as its `runner` member. A
//! machine that names no runner is answered the command as before. Every
//! refusal of a start is the library's, unchanged.

use std::future::Future;
use std::path::Path;
use std::pin::Pin;
use std::str::FromStr;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use axum::Router;
use axum::body::Bytes;
use axum::extract::{Path as UrlPath, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use lys_core::Ed25519Identity;
use lys_identity::start::active::Lifecycles;
use lys_identity::start::authority::Admission;
use lys_identity::start::credentials::HandleRecords;
use lys_identity::start::egress::{EgressLists, ProfileNeeds};
use lys_identity::start::machine_role::{HeldRole, RoleMachines};
use lys_identity::start::profile_command::ProfileVersionRecords;
use lys_identity::start::profile_review::{ProfileReviews, Review};
use lys_identity::start::request::{AgentRecord, AgentRecords};
use lys_identity::start::state::{SessionReport, SessionReports};
use lys_identity::start::{
    Given, Grammars, LaunchRecords, Owners, StartError, give, give_again, state_of, withdraw,
};
use lys_identity::{AgentId, IdentityId, LifecycleState, LoginBinding};
use serde_json::Value;

use crate::error::ServerError;
use crate::routes::{AppState, signed_in, with_directory};

/// Who is asking, as the records a start keeps name them.
pub trait Callers: Send + Sync {
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

    /// The owners as the library reads them.
    pub fn owners(&self) -> Owners<'_> {
        self.owners.owners()
    }

    /// The launch records, one caller at a time.
    pub fn launches(&self) -> MutexGuard<'_, LaunchRecords> {
        self.launches.lock().unwrap_or_else(PoisonError::into_inner)
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
    json(
        status,
        serde_json::json!({ "error": error, "words": words }).to_string(),
    )
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
        Err(refused) => refused,
    }
}

/// Run `act` as [`answer`] does, handing back what it made, or the answer
/// that refuses it.
async fn answer_with<F, T>(service: Shared, headers: &HeaderMap, act: F) -> Result<T, Response>
where
    F: FnOnce(&StartService, &mut LaunchRecords, &str) -> Result<T, StartError> + Send + 'static,
    T: Send + 'static,
{
    let Some(caller) = service.callers.caller(headers) else {
        return Err(named(
            StatusCode::UNAUTHORIZED,
            "not_signed_in",
            "sign in to start an agent",
        ));
    };
    let task = tokio::task::spawn_blocking(move || {
        let mut launches = service.launches();
        act(&service, &mut launches, &caller)
    });
    match task.await {
        Ok(Ok(made)) => Ok(made),
        Ok(Err(error)) => Err(json(status(&error), error.to_json())),
        Err(failed) => Err(named(
            StatusCode::INTERNAL_SERVER_ERROR,
            "start_task_failed",
            &failed.to_string(),
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

async fn start(
    State(service): State<Shared>,
    headers: HeaderMap,
    UrlPath(agent): UrlPath<String>,
    body: Bytes,
) -> Response {
    let Some(members) = members(&agent, &body) else {
        return named(
            StatusCode::BAD_REQUEST,
            "request_malformed",
            "a start request is a JSON object naming the profile version and the machine",
        );
    };
    if service.launcher.is_none() {
        return answer(service, &headers, move |service, launches, caller| {
            give(
                launches,
                &service.owners(),
                caller,
                members,
                (service.clock)(),
            )
            .map(|given| given.to_json())
        })
        .await;
    }
    let given = answer_with(
        Arc::clone(&service),
        &headers,
        move |service, launches, caller| {
            give(
                launches,
                &service.owners(),
                caller,
                members,
                (service.clock)(),
            )
            .map(|given| (given, caller.to_owned()))
        },
    )
    .await;
    let (given, caller) = match given {
        Ok(given) => given,
        Err(refused) => return refused,
    };
    let body = given.to_json();
    let Some(launcher) = service.launcher.as_ref() else {
        return json(StatusCode::OK, body);
    };
    match launcher.launch(&given, &caller).await {
        None => json(StatusCode::OK, body),
        Some(ran) => ran_answer(&body, ran),
    }
}

/// The given start's answer with the runner's word beside it: its
/// `runner` member when it ran, or the runner's refusal by name, the start
/// that was given beside it.
fn ran_answer(body: &str, ran: Result<Value, ServerError>) -> Response {
    let Ok(mut given) = serde_json::from_str::<Value>(body) else {
        return named(
            StatusCode::INTERNAL_SERVER_ERROR,
            "start_unreadable",
            "the given start does not read as JSON",
        );
    };
    match ran {
        Ok(runner) => {
            given["runner"] = runner;
            json(StatusCode::OK, given.to_string())
        }
        Err(refused) => {
            let status = refused.status();
            let body = serde_json::json!({
                "error": refused.name(),
                "words": refused.to_string(),
                "given": given,
            });
            json(status, body.to_string())
        }
    }
}

async fn start_again(
    State(service): State<Shared>,
    headers: HeaderMap,
    UrlPath(source): UrlPath<String>,
) -> Response {
    answer(service, &headers, move |service, launches, caller| {
        give_again(
            launches,
            &service.owners(),
            caller,
            &source,
            (service.clock)(),
        )
        .map(|given| given.to_json())
    })
    .await
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
        .map(|state| state.to_json())
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

/// The directory's own records, read for a start: the caller, the
/// administrator's admission, each agent's record and its lifecycle state.
struct Directory(Arc<AppState>);

impl Directory {
    /// The person bound to `binding`, or the login itself when none is.
    fn caller_of(&self, binding: &LoginBinding) -> String {
        let login = || format!("login {} {}", binding.issuer(), binding.subject());
        match with_directory(&self.0, |directory| {
            Ok(directory.projection()?.person_for(binding))
        }) {
            Ok(Some(person)) => person.to_string(),
            Ok(None) => login(),
            Err(error) => {
                tracing::error!("the directory could not be read for the caller: {error}");
                login()
            }
        }
    }

    fn record(&self, agent: &str) -> Option<lys_identity::projection::Record> {
        let id = AgentId::from_str(agent).ok()?;
        match with_directory(&self.0, |directory| {
            Ok(directory.record(IdentityId::Agent(id))?)
        }) {
            Ok(record) => record,
            Err(error) => {
                tracing::error!(agent, "the directory could not be read: {error}");
                None
            }
        }
    }
}

impl Callers for Directory {
    fn caller(&self, headers: &HeaderMap) -> Option<String> {
        let actor = signed_in(&self.0, headers).ok()?;
        Some(self.caller_of(actor.binding()))
    }
}

impl Admission for Directory {
    fn is_administrator(&self, caller: &str) -> bool {
        self.0
            .admission
            .administrator_login()
            .is_some_and(|login| caller == self.caller_of(&login))
    }

    /// Step 1 admits the configured administrator alone.
    fn admits(&self, caller: &str) -> bool {
        self.is_administrator(caller)
    }
}

impl AgentRecords for Directory {
    fn agent(&self, agent: &str) -> Option<AgentRecord> {
        let record = self.record(agent)?;
        Some(AgentRecord {
            id: agent.to_owned(),
            responsible: record.responsible().map(|person| person.to_string()),
        })
    }
}

impl Lifecycles for Directory {
    fn state(&self, agent: &str) -> Option<LifecycleState> {
        self.record(agent).map(|record| record.state())
    }
}

/// An owner's record a start reads that has not landed in this tree: every
/// read answers that the record does not exist, so the check it feeds is
/// refused by name, naming the card that makes it.
struct NotLanded(&'static str);

impl NotLanded {
    fn absent<T>(&self, asked: &str) -> Option<T> {
        tracing::info!(
            owner = self.0,
            asked,
            "the record a start reads does not exist in this tree"
        );
        None
    }
}

impl ProfileReviews for NotLanded {
    fn review(&self, profile_version: &str) -> Option<Review> {
        self.absent(profile_version)
    }
}

impl RoleMachines for NotLanded {
    fn roles(&self, agent: &str) -> Option<Vec<HeldRole>> {
        self.absent(agent)
    }
}

impl ProfileNeeds for NotLanded {
    fn needs(&self, profile_version: &str) -> Option<Vec<String>> {
        self.absent(profile_version)
    }
}

impl EgressLists for NotLanded {
    fn egress(&self, machine: &str) -> Option<Vec<String>> {
        self.absent(machine)
    }
}

impl ProfileVersionRecords for NotLanded {
    fn executable(&self, profile_version: &str) -> Option<String> {
        self.absent(profile_version)
    }

    fn arguments(&self, profile_version: &str) -> Option<Vec<String>> {
        self.absent(profile_version)
    }

    fn working_directory(&self, profile_version: &str) -> Option<String> {
        self.absent(profile_version)
    }
}

impl SessionReports for NotLanded {
    /// No sessions record exists here, so no report names any launch record
    /// and every one reads unconfirmed, never running.
    fn reports(&self, launch_record: &str) -> Vec<SessionReport> {
        self.absent::<Vec<SessionReport>>(launch_record)
            .unwrap_or_default()
    }
}

fn agent_id(text: &str) -> bool {
    AgentId::from_str(text).is_ok()
}

/// The start service over the directory `state` holds, reading the handle
/// record through `handles`, whose credential ids are held to
/// `credential_id`, and keeping launch records in `dir`, signed by `key`.
/// The roles card's, the network's and the sessions brief's records have
/// not landed in this tree, so each is read as not existing.
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
        reviews: Box::new(NotLanded("Ink1H1Os")),
        role_machines: Box::new(NotLanded("Ink1H1Os")),
        handles,
        needs: Box::new(NotLanded("Ink1H1Os")),
        egress: Box::new(NotLanded("network row 8.5")),
        profiles: Box::new(NotLanded("Ink1H1Os")),
        sessions: Box::new(NotLanded("d5055cc1")),
        grammars: Grammars {
            agent_id,
            credential_id,
        },
    };
    let service = StartService::new(
        owners,
        Box::new(Directory(Arc::clone(state))),
        launches,
        crate::session::now,
    );
    let runs = crate::runner_sessions::DirectoryLauncher(Arc::clone(state));
    Ok(Arc::new(service.with_launcher(Box::new(runs))))
}
