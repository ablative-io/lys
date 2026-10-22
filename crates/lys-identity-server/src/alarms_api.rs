//! Alarm reads are confined to the caller's agents; a person acknowledges,
//! or an agent given `alarm.acknowledge` on the alarm (AGENTS-019 R4).

use super::alarms::AlarmStore;
use super::alarms_state::{Alarm, AlarmError, unavailable};
use crate::routes::{AppState, signed_in, with_directory};
use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use axum::{
    Json, Router,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};

// Reopening under one lock settles every uncertain append before another read.
static STORE: Mutex<()> = Mutex::new(());

pub(crate) struct ApiError(Box<Response>);

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        *self.0
    }
}

fn response(error: impl IntoResponse) -> ApiError {
    ApiError(Box::new(error.into_response()))
}

impl IntoResponse for AlarmError {
    fn into_response(self) -> Response {
        let (status, refusal) = match &self {
            Self::Unavailable(_) => (StatusCode::SERVICE_UNAVAILABLE, "AlarmsUnavailable"),
            Self::Malformed(_) => (StatusCode::BAD_REQUEST, "AlarmMalformed"),
            Self::Unknown => (StatusCode::NOT_FOUND, "AlarmUnknown"),
        };
        (
            status,
            Json(serde_json::json!({"refusal": refusal, "reason": self.to_string()})),
        )
            .into_response()
    }
}

/// A list and the revision at which it was read.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct AlarmList {
    /// Visible occurrences, open first.
    pub alarms: Vec<Alarm>,
    /// The durable log revision.
    pub revision: u64,
}

/// The dashboard's visible open count and its three newest open alarms.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct AlarmSummary {
    /// Open conditions, including acknowledged conditions still active.
    pub open_count: usize,
    /// At most three newest visible open occurrences.
    pub newest: Vec<Alarm>,
}

/// Acknowledgement carries no caller-controlled fields.
#[derive(Deserialize, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct AlarmAcknowledgement {}

/// Mount the alarm routes beside the goal routes.
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/alarms", get(list))
        .route("/alarms/{id}", get(one))
        .route("/alarms/{id}/acknowledge", post(acknowledge))
}

pub(crate) fn types(api: &mut lys_openapi::Api) -> Vec<crate::openapi_types::Entry> {
    use lys_openapi::Method::{Get, Post};
    vec![
        (Get, "/alarms", None, Some(api.schema::<AlarmList>())),
        (Get, "/alarms/{id}", None, Some(api.schema::<Alarm>())),
        (
            Post,
            "/alarms/{id}/acknowledge",
            Some(api.schema::<AlarmAcknowledgement>()),
            Some(api.schema::<Alarm>()),
        ),
    ]
}

#[cfg(test)]
#[path = "alarms_api_tests.rs"]
mod refusal_tests;

pub(crate) fn with_store<T>(
    state: &AppState,
    act: impl FnOnce(&mut AlarmStore, ()) -> Result<T, AlarmError>,
) -> Result<T, AlarmError> {
    let guard = STORE.lock().map_err(unavailable)?;
    let mut store = AlarmStore::open(&state.estate_plan_file.with_file_name("alarms"))?;
    let answer = act(&mut store, ());
    drop(guard);
    answer
}

fn visible(
    state: &AppState,
    headers: &HeaderMap,
    uri: &Uri,
) -> Result<BTreeSet<String>, crate::error::ServerError> {
    with_directory(state, |directory| {
        let projection = directory.projection()?;
        if let Some(agent) =
            crate::agent_pass::holder(state, headers)?.or(crate::agent_signature::signed_agent(
                state,
                projection,
                headers,
                ("GET", uri.path(), &[]),
            )?)
        {
            return Ok(BTreeSet::from([agent.to_string()]));
        }
        let actor = signed_in(state, headers)?;
        let person = crate::read_api::own_person(projection, &actor)?;
        projection
            .agents_of(person)
            .map(|entry| entry.map(|(id, _)| id.to_string()).map_err(Into::into))
            .collect()
    })
}

pub(crate) fn listed(
    state: &AppState,
    headers: &HeaderMap,
    uri: &Uri,
) -> Result<AlarmList, ApiError> {
    let visible = visible(state, headers, uri).map_err(response)?;
    with_store(state, |store, ()| {
        Ok(AlarmList {
            alarms: store
                .list()
                .into_iter()
                .filter(|alarm| visible.contains(&alarm.agent))
                .collect(),
            revision: store.revision(),
        })
    })
    .map_err(response)
}

pub(crate) fn summary(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<AlarmSummary, super::Refusal> {
    let uri = Uri::from_static("/alarms");
    let visible = visible(state, headers, &uri).map_err(super::Refusal::from)?;
    with_store(state, |store, ()| {
        let open: Vec<_> = store
            .list()
            .into_iter()
            .filter(|alarm| alarm.cleared_at.is_none() && visible.contains(&alarm.agent))
            .collect();
        Ok(AlarmSummary {
            open_count: open.len(),
            newest: open.into_iter().take(3).collect(),
        })
    })
    .map_err(|error| super::Refusal {
        refusal: "AlarmsUnavailable".to_owned(),
        reason: error.to_string(),
    })
}

async fn list(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    uri: Uri,
) -> Result<Json<AlarmList>, ApiError> {
    listed(&state, &headers, &uri).map(Json)
}

async fn one(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    uri: Uri,
    Path(id): Path<String>,
) -> Result<Json<Alarm>, ApiError> {
    listed(&state, &headers, &uri)?
        .alarms
        .into_iter()
        .find(|alarm| alarm.id == id)
        .map(Json)
        .ok_or_else(|| response(AlarmError::Unknown))
}

async fn acknowledge(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    body: Bytes,
) -> Result<Json<Alarm>, ApiError> {
    if !body.is_empty() {
        let input: serde_json::Value = serde_json::from_slice(&body)
            .map_err(|error| response(AlarmError::Malformed(error.to_string())))?;
        if !input.as_object().is_some_and(serde_json::Map::is_empty) {
            return Err(response(AlarmError::Malformed(
                "acknowledgement takes an empty object; the authenticated person is recorded"
                    .to_owned(),
            )));
        }
    }
    // An agent the door admitted holds alarm.acknowledge on this alarm by its
    // own grant (AGENTS-019 R4), recorded by its own name; a person
    // acknowledges an alarm of their own agents.
    let actor = signed_in(&state, &headers).map_err(response)?;
    let given = crate::routes::door_agent(&state, &actor).map_err(response)?;
    let by = match given {
        Some(agent) => agent.to_string(),
        None => with_directory(&state, |directory| {
            Ok(crate::read_api::own_person(directory.projection()?, &actor)?.to_string())
        })
        .map_err(response)?,
    };
    let visible = match given {
        Some(_) => None,
        None => Some(visible(&state, &headers, &Uri::from_static("/alarms")).map_err(response)?),
    };
    let alarm = with_store(&state, |store, ()| {
        let held = store.get(&id)?;
        if visible
            .as_ref()
            .is_some_and(|visible| !visible.contains(&held.agent))
        {
            return Err(AlarmError::Unknown);
        }
        store.acknowledge(&id, &by, crate::session::now())
    })
    .map_err(response)?;
    state.changes.signal().map_err(response)?;
    Ok(Json(alarm))
}

#[cfg(test)]
mod tests {
    use super::super::alarms::{Observation, ProxyEvidence};
    use super::super::alarms_state::Kind;
    use super::with_store;
    use crate::budgets_feed::entry_tests::fixture;
    use std::collections::BTreeMap;

    #[tokio::test]
    async fn alarm_profile_store_ignores_missing_invalid_and_different_organisation_thresholds()
    -> Result<(), Box<dyn std::error::Error>> {
        let table = fixture().await?;
        let path = table
            .state
            .estate_plan_file
            .with_file_name("organisation")
            .join("configuration.json");
        std::fs::create_dir_all(path.parent().ok_or("configuration parent missing")?)?;
        let configurations: [Option<&[u8]>; 4] = [
            None,
            Some(b"invalid configuration"),
            Some(br#"{"alarms":{"stall_after_seconds":0}}"#),
            Some(br#"{"alarms":{"stall_after_seconds":500}}"#),
        ];
        for (index, configuration) in configurations.into_iter().enumerate() {
            if let Some(bytes) = configuration {
                std::fs::write(&path, bytes)?;
            } else if path.exists() {
                std::fs::remove_file(&path)?;
            }
            let observations: Vec<_> = ["short", "long"]
                .into_iter()
                .map(|name| Observation {
                    session: format!("alarm-profile-{index}-{name}"),
                    agent: table.agent.clone(),
                    machine: "machine".to_owned(),
                    live: true,
                    started_at_ms: 1_000,
                    last_signal_at_ms: Some(1_000),
                    proxy: ProxyEvidence::Direct,
                })
                .collect();
            let thresholds = BTreeMap::from([
                (observations[0].session.clone(), 10),
                (observations[1].session.clone(), 20),
            ]);
            with_store(&table.state, |store, ()| {
                assert!(!store.observe_profiles(&observations, 11_000, &thresholds)?);
                assert!(store.observe_profiles(&observations, 12_000, &thresholds)?);
                let alarms = store.list();
                assert!(alarms.iter().any(|alarm| {
                    alarm.session == observations[0].session && alarm.kind == Kind::SeatStalled
                }));
                assert!(
                    alarms
                        .iter()
                        .all(|alarm| alarm.session != observations[1].session)
                );
                Ok(())
            })?;
            if let Some(bytes) = configuration {
                assert_eq!(std::fs::read(&path)?, bytes);
            } else {
                assert!(
                    !path.exists(),
                    "profile evaluation created organisation configuration"
                );
            }
        }
        table.finish().await
    }
}
