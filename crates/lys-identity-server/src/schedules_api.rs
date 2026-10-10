//! The schedules routes (AGENTS-001 R4): the administrator, or the person
//! responsible for every agent a schedule names, sets, changes and stops
//! it and reads it with every occurrence and delivery; and the task that
//! fires each occurrence when it falls due, through each recipient's
//! sessions' runners, under the runner's operation ids.

use std::collections::BTreeMap;
use std::str::FromStr;
use std::sync::Arc;

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::routing::{get, post};
use axum::{Json, Router};
use lys_identity::{AgentId, IdentityId, OperationId};
use lys_runner::operations::Operation;
use serde::{Deserialize, Serialize};

use crate::error::ServerError;
use crate::read_api::own_person;
use crate::routes::{AppState, is_administrator, signed_in, with_directory};
use crate::schedules_state::{Change, Changed, Item, Recipient, Schedule, SchedulesError, Source};
use crate::schedules_store::{Deliver, Delivering, SchedulesKept, Undelivered, Worded};
use crate::session::now;
use crate::variables_api::session_agent;
use crate::words_state::Slot;

/// A schedule to set.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[schema(as = ScheduleSetBody)]
#[serde(deny_unknown_fields)]
pub struct SetBody {
    /// The operation id, which names the schedule.
    pub operation: String,
    /// The first instant, in seconds since the Unix epoch, after now.
    pub at: u64,
    /// The instant before which every occurrence falls, exclusive.
    #[serde(default)]
    pub until: Option<u64>,
    /// Seconds between occurrences; none for once.
    #[serde(default)]
    pub interval: Option<u64>,
    /// The most occurrences; none for no limit.
    #[serde(default)]
    pub max_occurrences: Option<u64>,
    /// Who it sends to.
    pub recipients: Vec<Recipient>,
    /// What it sends.
    pub source: Source,
}

/// A change to a schedule.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[schema(as = ScheduleChangeBody)]
#[serde(deny_unknown_fields)]
pub struct ChangeBody {
    /// The operation id naming the change.
    pub operation: String,
    /// The change.
    pub change: Change,
}

/// A stop.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[schema(as = ScheduleStopBody)]
#[serde(deny_unknown_fields)]
pub struct StopBody {
    /// Why, in the caller's words.
    pub words: String,
}

/// Every schedule the caller may see.
#[derive(Debug, Serialize, utoipa::ToSchema)]
#[schema(as = SchedulesView)]
pub struct View {
    /// The schedules, in the order set.
    pub schedules: Vec<Item>,
}

/// The schedules routes.
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/schedules", get(read_all).post(set))
        .route("/schedules/{id}", get(read_one))
        .route("/schedules/{id}/change", post(change))
        .route("/schedules/{id}/stop", post(stop))
}

pub(crate) fn schedules(state: &AppState) -> Result<&SchedulesKept, ServerError> {
    state.schedules.as_ref().ok_or_else(|| {
        SchedulesError::Unavailable {
            reason: "the configuration names no schedules_dir".to_owned(),
        }
        .into()
    })
}

fn body<T>(given: Result<Json<T>, JsonRejection>) -> Result<T, ServerError> {
    given
        .map(|Json(body)| body)
        .map_err(|refused| ServerError::RequestMalformed {
            reason: refused.body_text(),
        })
}

/// The agents `recipients` name, each session resolved to its agent.
fn agents_of(state: &AppState, recipients: &[Recipient]) -> Result<Vec<String>, ServerError> {
    recipients
        .iter()
        .map(|recipient| match recipient {
            Recipient::Agent { id } => {
                AgentId::from_str(id).map_err(|_unread| ServerError::AgentNotVisible)?;
                Ok(id.clone())
            }
            Recipient::Session { id } => session_agent(state, id),
        })
        .collect()
}

/// The caller admitted over every agent in `agents`: the administrator, or
/// the person responsible for each of them; by name.
fn over(state: &AppState, headers: &HeaderMap, agents: &[String]) -> Result<String, ServerError> {
    let actor = signed_in(state, headers)?;
    if is_administrator(state, &actor)? {
        return match with_directory(state, |directory| {
            own_person(directory.projection()?, &actor)
        }) {
            Ok(person) => Ok(person.to_string()),
            Err(ServerError::NoPerson) => Ok("operator".to_owned()),
            Err(error) => Err(error),
        };
    }
    let person = with_directory(state, |directory| {
        own_person(directory.projection()?, &actor)
    })?;
    for agent in agents {
        let agent_id = AgentId::from_str(agent).map_err(|_unread| ServerError::AgentNotVisible)?;
        let responsible = with_directory(state, |directory| {
            let projection = directory.projection()?;
            let record = projection
                .record(IdentityId::Agent(agent_id))
                .ok_or(ServerError::AgentNotVisible)?;
            Ok(record.responsible())
        })?;
        if responsible != Some(person) {
            return Err(ServerError::AgentNotVisible);
        }
    }
    Ok(person.to_string())
}

async fn read_all(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<View>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    let items = crate::schedules_store::items(schedules(&state)?)?;
    if is_administrator(&state, &actor)? {
        return Ok(Json(View { schedules: items }));
    }
    let mut visible = Vec::new();
    for item in items {
        let agents = match agents_of(&state, item.recipients()) {
            Ok(agents) => agents,
            Err(_) => continue,
        };
        if over(&state, &headers, &agents).is_ok() {
            visible.push(item);
        }
    }
    Ok(Json(View { schedules: visible }))
}

async fn read_one(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<Item>, ServerError> {
    let item = crate::schedules_store::item(schedules(&state)?, &id)?;
    let agents = agents_of(&state, item.recipients())?;
    over(&state, &headers, &agents).map_err(|_refused| SchedulesError::Unknown)?;
    Ok(Json(item))
}

async fn set(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    given: Result<Json<SetBody>, JsonRejection>,
) -> Result<Json<Item>, ServerError> {
    let body = body(given)?;
    let operation = OperationId::from_str(&body.operation)?;
    let agents = agents_of(&state, &body.recipients)?;
    let author = over(&state, &headers, &agents)?;
    let schedule = Schedule {
        id: operation.to_string(),
        at: body.at,
        until: body.until,
        interval: body.interval,
        max_occurrences: body.max_occurrences,
        recipients: body.recipients,
        source: body.source,
        author,
        set_at: now(),
    };
    let kept = schedules(&state)?;
    let item = crate::schedules_store::set(kept, schedule)?;
    Ok(Json(item))
}

async fn change(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    given: Result<Json<ChangeBody>, JsonRejection>,
) -> Result<Json<Item>, ServerError> {
    let body = body(given)?;
    let operation = OperationId::from_str(&body.operation)?;
    let kept = schedules(&state)?;
    let item = crate::schedules_store::item(kept, &id)?;
    let mut agents = agents_of(&state, item.recipients())?;
    if let Change::Recipients { recipients } = &body.change {
        agents.extend(agents_of(&state, recipients)?);
    }
    let by = over(&state, &headers, &agents).map_err(|_refused| SchedulesError::Unknown)?;
    let item = crate::schedules_store::change(
        kept,
        Changed {
            operation: operation.to_string(),
            schedule: id,
            change: body.change,
            by,
            at: now(),
        },
    )?;
    Ok(Json(item))
}

async fn stop(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    given: Result<Json<StopBody>, JsonRejection>,
) -> Result<Json<Item>, ServerError> {
    let body = body(given)?;
    let kept = schedules(&state)?;
    let item = crate::schedules_store::item(kept, &id)?;
    let agents = agents_of(&state, item.recipients())?;
    let by = over(&state, &headers, &agents).map_err(|_refused| SchedulesError::Unknown)?;
    let words = body.words.trim();
    let item = crate::schedules_store::stop(kept, &id, format!("stopped_by: {by}: {words}"))?;
    Ok(Json(item))
}

/// Occurrences delivered through the runners the service reaches.
struct Live<'a>(&'a Arc<AppState>);

impl Deliver for Live<'_> {
    fn sessions(&self, recipient: &Recipient) -> Result<Vec<(String, Option<String>)>, String> {
        match recipient {
            Recipient::Agent { id } => crate::runner_api::open_sessions(self.0, id)
                .map(|driven| {
                    driven
                        .into_iter()
                        .map(|driven| (driven.session, Some(driven.agent)))
                        .collect()
                })
                .map_err(|error| error.name()),
            Recipient::Session { id } => {
                let agent = session_agent(self.0, id).map_err(|error| error.name())?;
                Ok(vec![(id.clone(), Some(agent))])
            }
        }
    }

    fn worded(
        &self,
        source: &Source,
        agent: Option<&str>,
        session: &str,
    ) -> Result<Worded, String> {
        let numbers = BTreeMap::new();
        let worded = match source {
            Source::Slot { slot } => {
                let delivered = crate::words_api::deliverable(
                    self.0,
                    *slot,
                    agent,
                    Some(session),
                    numbers,
                    None,
                )
                .map_err(|error| error.to_string())?;
                Worded {
                    text: delivered.text,
                    contributed: delivered.contributed,
                    missing: delivered.missing,
                }
            }
            Source::Text { text } => {
                let mut numbers = numbers;
                numbers.insert("text".to_owned(), text.clone());
                let (rendered, contributed) =
                    crate::words_api::rendered(self.0, text, agent, Some(session), numbers, None)
                        .map_err(|error| error.to_string())?;
                Worded {
                    text: rendered.text,
                    contributed,
                    missing: rendered.missing,
                }
            }
        };
        let _ = Slot::ScheduledReminder;
        Ok(worded)
    }

    fn operate(&self, operation: Operation) -> Delivering<'_> {
        Box::pin(async move {
            crate::runner_operate::operate(self.0, operation)
                .await
                .map_err(|undelivered| match undelivered {
                    Undelivered::Refused(words) => Undelivered::Refused(words),
                    Undelivered::Unknown(words) => Undelivered::Unknown(words),
                })
        })
    }
}

/// Start the task that fires each occurrence when it falls due, for as
/// long as `state` is served.
pub fn fire_from(state: &Arc<AppState>) {
    let Some(schedules) = state.schedules.as_ref() else {
        return;
    };
    let changed = Arc::clone(&schedules.changed);
    let served = Arc::downgrade(state);
    tokio::spawn(async move {
        loop {
            let Some(state) = served.upgrade() else {
                return;
            };
            let next = match pass(&state).await {
                Ok(next) => next,
                Err(error) => {
                    (state.say)(&format!("schedules: a pass failed: {error}"));
                    None
                }
            };
            drop(state);
            match next {
                Some(due) => {
                    let wait = std::time::Duration::from_secs(due.saturating_sub(now()));
                    tokio::select! {
                        () = tokio::time::sleep(wait) => {}
                        () = changed.notified() => {}
                    }
                }
                None => changed.notified().await,
            }
        }
    });
}

async fn pass(state: &Arc<AppState>) -> Result<Option<u64>, ServerError> {
    let kept = schedules(state)?;
    crate::schedules_store::pass(kept, &Live(state), now()).await
}
