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

use std::path::Path;
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
    Grammars, LaunchRecords, Owners, StartError, give, give_again, state_of, withdraw,
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

/// What the start route answers from.
pub struct StartService {
    owners: StartOwners,
    callers: Box<dyn Callers>,
    launches: Mutex<LaunchRecords>,
    clock: fn() -> u64,
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
    let Some(caller) = service.callers.caller(headers) else {
        return named(
            StatusCode::UNAUTHORIZED,
            "not_signed_in",
            "sign in to start an agent",
        );
    };
    let task = tokio::task::spawn_blocking(move || {
        let mut launches = service.launches();
        act(&service, &mut launches, &caller)
    });
    match task.await {
        Ok(Ok(body)) => json(StatusCode::OK, body),
        Ok(Err(error)) => json(status(&error), error.to_json()),
        Err(failed) => named(
            StatusCode::INTERNAL_SERVER_ERROR,
            "start_task_failed",
            &failed.to_string(),
        ),
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
    answer(service, &headers, move |service, launches, caller| {
        give(
            launches,
            &service.owners(),
            caller,
            members,
            (service.clock)(),
        )
        .map(|given| given.to_json())
    })
    .await
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
        caller == self.caller_of(self.0.admission.administrator_login())
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
    Ok(Arc::new(StartService::new(
        owners,
        Box::new(Directory(Arc::clone(state))),
        launches,
        crate::session::now,
    )))
}
