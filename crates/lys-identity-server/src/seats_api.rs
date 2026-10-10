//! The seat routes (AGENTS-002): a seat is a Lys record, started through
//! the server's own start path, seen with its runner's knowledge of it,
//! stopped and restarted under rights, sent messages as user turns, and
//! attached to.
//!
//! - `POST /seats` `{operation, name, agent, profile_version, machine,
//!   working_folder?, account?}`: add a seat
//! - `GET /seats`: every seat with its state, and whether the runners were read
//! - `GET /seats/unregistered`: the sessions runners hold for no seat
//! - `GET /seats/{name}`: one seat
//! - `POST /seats/{name}/start` `{operation}`, `/stop` `{operation, force?}`,
//!   `/restart` `{operation, force?}`, `/send` `{operation, text}`,
//!   `/type` `{operation, text}`, `/attach` `{cursor?, follow?}`: in
//!   `seats_run.rs` and `seats_send.rs`
//!
//! A seat is added by the administrator or the person responsible for its
//! agent, as that agent's start command is given. Its agent must be held
//! by the directory, its profile version reviewed and declaring a harness,
//! and its machine must name a runner.

use std::collections::{BTreeMap, BTreeSet};
use std::str::FromStr;
use std::sync::{Arc, Mutex};

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::routing::{get, post};
use axum::{Json, Router};
use lys_identity::{AgentId, IdentityId, OperationId};

use crate::error::ServerError;
use crate::error_seat::SeatError;
use crate::network_api::with_network;
use crate::openapi_table::{GET, POST};
use crate::openapi_types::Entry;
use crate::provisioning_api::with_provisioning;
use crate::routes::{AppState, signed_in, with_directory};
use crate::seats_acts::{Live, liveness};
use crate::seats_state::{Added, Line, Seat};
use crate::seats_store::SeatStore;
use crate::seats_views::{
    Reading, SeatAddBody, SeatList, SeatRunnerUnread, SeatUnregistered, SeatView,
    UnregisteredSession, view,
};
use crate::session::now;

/// The longest seat name.
const NAME_MAX: usize = 64;

/// The seat routes.
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/seats", get(list).post(add))
        .route("/seats/unregistered", get(unregistered))
        .route("/seats/{name}", get(one))
        .route("/seats/{name}/start", post(crate::seats_run::start))
        .route("/seats/{name}/stop", post(crate::seats_run::stop))
        .route("/seats/{name}/restart", post(crate::seats_run::restart))
        .route("/seats/{name}/send", post(crate::seats_send::send))
        .route("/seats/{name}/type", post(crate::seats_send::typed))
        .route("/seats/{name}/attach", post(crate::seats_send::attach))
}

/// The seat routes' schemas, for the document.
pub(crate) fn types(api: &mut lys_openapi::Api) -> Vec<Entry> {
    use crate::seats_views::{
        SeatAttachBody, SeatAttached, SeatOperationBody, SeatRestarted, SeatSendBody, SeatSent,
        SeatStarted, SeatStopBody, SeatStopped,
    };
    let seat = api.schema::<SeatView>();
    let stop = api.schema::<SeatStopBody>();
    let send = api.schema::<SeatSendBody>();
    let sent = api.schema::<SeatSent>();
    vec![
        (GET, "/seats", None, Some(api.schema::<SeatList>())),
        (POST, "/seats", Some(api.schema::<SeatAddBody>()), Some(seat.clone())),
        (
            GET,
            "/seats/unregistered",
            None,
            Some(api.schema::<SeatUnregistered>()),
        ),
        (GET, "/seats/{name}", None, Some(seat)),
        (
            POST,
            "/seats/{name}/start",
            Some(api.schema::<SeatOperationBody>()),
            Some(api.schema::<SeatStarted>()),
        ),
        (
            POST,
            "/seats/{name}/stop",
            Some(stop.clone()),
            Some(api.schema::<SeatStopped>()),
        ),
        (
            POST,
            "/seats/{name}/restart",
            Some(stop),
            Some(api.schema::<SeatRestarted>()),
        ),
        (POST, "/seats/{name}/send", Some(send.clone()), Some(sent.clone())),
        (POST, "/seats/{name}/type", Some(send), Some(sent)),
        (
            POST,
            "/seats/{name}/attach",
            Some(api.schema::<SeatAttachBody>()),
            Some(api.schema::<SeatAttached>()),
        ),
    ]
}

pub(crate) fn body<T>(body: Result<Json<T>, JsonRejection>) -> Result<T, ServerError> {
    body.map(|Json(body)| body)
        .map_err(|refused| ServerError::RequestMalformed {
            reason: refused.body_text(),
        })
}

/// Run `act` with the seats, one caller at a time, settled first.
pub(crate) fn with_seats<T>(
    state: &AppState,
    act: impl FnOnce(&mut SeatStore) -> Result<T, ServerError>,
) -> Result<T, ServerError> {
    let mut store = lock(&state.seats)?;
    store.settle()?;
    act(&mut store)
}

fn lock(seats: &Mutex<SeatStore>) -> Result<std::sync::MutexGuard<'_, SeatStore>, ServerError> {
    seats.lock().map_err(|error| {
        SeatError::Unavailable {
            reason: format!("the seats lock is poisoned: {error}"),
        }
        .into()
    })
}

/// The seat named `name`, refused `seat_unknown` when none is.
pub(crate) fn seat(state: &AppState, name: &str) -> Result<Seat, ServerError> {
    with_seats(state, |store| store.named(name))
}

/// Refuse a name that is not 1 to 64 lowercase letters, digits and hyphens.
pub(crate) fn checked_name(name: &str) -> Result<(), ServerError> {
    let fits = !name.is_empty()
        && name.len() <= NAME_MAX
        && name
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-');
    if fits {
        Ok(())
    } else {
        Err(SeatError::NameInvalid {
            name: name.to_owned(),
        }
        .into())
    }
}

/// The person responsible for each of `agents`, as the directory holds them now.
pub(crate) fn responsible(
    state: &AppState,
    agents: &BTreeSet<String>,
) -> Result<BTreeMap<String, Option<String>>, ServerError> {
    with_directory(state, |directory| {
        let projection = directory.projection()?;
        agents
            .iter()
            .map(|agent| {
                let id = AgentId::from_str(agent)?;
                let person = projection
                    .record(IdentityId::Agent(id))
                    .and_then(lys_identity::projection::Record::responsible)
                    .map(|person| person.to_string());
                Ok((agent.clone(), person))
            })
            .collect()
    })
}

/// `seat` as shown now: its state read from its machine's runner.
pub(crate) async fn shown(state: &Arc<AppState>, seat: &Seat) -> Result<SeatView, ServerError> {
    let people = responsible(state, &BTreeSet::from([seat.added.agent.clone()]))?;
    let person = people.get(&seat.added.agent).cloned().flatten();
    let Some(session) = seat.session.clone() else {
        return Ok(view(seat, person, &Reading::Read(None)));
    };
    Ok(
        match crate::seats_acts::live_session(state, &seat.added.machine, &session).await {
            Ok(live) => view(seat, person, &Reading::Read(live.as_ref())),
            Err(refused) => view(seat, person, &Reading::Unread(&refused.to_string())),
        },
    )
}

async fn add(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    given: Result<Json<SeatAddBody>, JsonRejection>,
) -> Result<Json<SeatView>, ServerError> {
    let given = body(given)?;
    let actor = signed_in(&state, &headers)?;
    checked_name(&given.name)?;
    OperationId::from_str(&given.operation)?;
    let working_folder = given
        .working_folder
        .map(|folder| crate::provisioning_api::folder("working_folder", folder))
        .transpose()?;
    let by = crate::launch_api::start_caller(&state, &headers, &actor, &given.agent)?;
    let number = given.profile_version;
    let harness = with_provisioning(&state, |store| {
        let version = store
            .version(&given.agent, number)
            .ok_or(ServerError::ProfileVersionUnknown { version: number })?;
        crate::start_checks::reviewed(version)?;
        version
            .settings
            .harness
            .as_ref()
            .map(|harness| harness.name.clone())
            .ok_or(ServerError::HarnessUndeclared { version: number })
    })?;
    with_network(&state, |store| {
        let machine = store
            .machine(&given.machine)
            .ok_or(ServerError::MachineUnknown)?;
        if machine.retired.is_some() {
            return Err(ServerError::MachineRetired);
        }
        store
            .runner(&given.machine)
            .ok_or(ServerError::MachineWithoutRunner)?;
        Ok(())
    })?;
    let line = Line::Added(Added {
        operation: given.operation,
        name: given.name,
        agent: given.agent,
        harness,
        profile_version: number,
        machine: given.machine,
        working_folder,
        account: given.account,
        by,
        at: now(),
    });
    let name = line.name().to_owned();
    let seat = with_seats(&state, |store| {
        store.keep(line)?;
        store.named(&name)
    })?;
    Ok(Json(shown(&state, &seat).await?))
}

async fn one(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(name): Path<String>,
) -> Result<Json<SeatView>, ServerError> {
    signed_in(&state, &headers)?;
    let seat = seat(&state, &name)?;
    Ok(Json(shown(&state, &seat).await?))
}

/// Every seat, each machine's runner asked once for every session it
/// holds; a runner that cannot be read leaves its seats `unknown`, named.
async fn list(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<SeatList>, ServerError> {
    signed_in(&state, &headers)?;
    let seats = with_seats(&state, |store| Ok(store.seats().to_vec()))?;
    let agents = seats.iter().map(|seat| seat.added.agent.clone()).collect();
    let people = responsible(&state, &agents)?;
    let machines: BTreeSet<String> = seats
        .iter()
        .filter(|seat| seat.session.is_some())
        .map(|seat| seat.added.machine.clone())
        .collect();
    let mut read: BTreeMap<String, Result<Vec<Live>, String>> = BTreeMap::new();
    for machine in machines {
        let answer = liveness(&state, &machine, None)
            .await
            .map_err(|refused| refused.to_string());
        read.insert(machine, answer);
    }
    let unread: Vec<String> = read
        .iter()
        .filter_map(|(machine, answer)| {
            answer.as_ref().err().map(|reason| {
                format!("the runner of machine {machine} could not be read: {reason}")
            })
        })
        .collect();
    let views = seats
        .iter()
        .map(|seat| {
            let person = people.get(&seat.added.agent).cloned().flatten();
            let reading = match (read.get(&seat.added.machine), &seat.session) {
                (Some(Ok(held)), Some(session)) => {
                    Reading::Read(held.iter().find(|live| &live.session == session))
                }
                (Some(Err(reason)), Some(_)) => Reading::Unread(reason),
                _ => Reading::Read(None),
            };
            view(seat, person, &reading)
        })
        .collect();
    Ok(Json(SeatList {
        seats: views,
        runner: (if unread.is_empty() { "read" } else { "unknown" }).to_owned(),
        reason: (!unread.is_empty()).then(|| unread.join("; ")),
    }))
}

/// The sessions every machine's runner holds that no seat's latest session
/// is: each runner asked once, and one that cannot be read named beside them.
async fn unregistered(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<SeatUnregistered>, ServerError> {
    signed_in(&state, &headers)?;
    let held: BTreeSet<String> = with_seats(&state, |store| {
        Ok(store
            .seats()
            .iter()
            .filter_map(|seat| seat.session.clone())
            .collect())
    })?;
    let machines: Vec<String> = if state.network.is_none() {
        Vec::new()
    } else {
        with_network(&state, |store| {
            Ok(store
                .machines()
                .iter()
                .filter(|machine| machine.retired.is_none())
                .filter(|machine| store.runner(&machine.id).is_some())
                .map(|machine| machine.id.clone())
                .collect())
        })?
    };
    let mut sessions = Vec::new();
    let mut unanswered = Vec::new();
    for machine in machines {
        match liveness(&state, &machine, None).await {
            Ok(live) => {
                for live in live
                    .into_iter()
                    .filter(|live| live.alive && !live.ended && !held.contains(&live.session))
                {
                    let agent = session_agent(&state, &live.session)?;
                    sessions.push(UnregisteredSession {
                        session: live.session,
                        machine: machine.clone(),
                        agent,
                        pid: live.pid,
                        started_at: live.started_at,
                    });
                }
            }
            Err(refused) => unanswered.push(SeatRunnerUnread {
                machine,
                refusal: refused.name(),
                reason: refused.to_string(),
            }),
        }
    }
    Ok(Json(SeatUnregistered {
        sessions,
        unanswered,
    }))
}

/// The agent the runtime reports keep `session` under; none when they keep
/// no such session, or keep no reports.
fn session_agent(state: &AppState, session: &str) -> Result<Option<String>, ServerError> {
    if state.runtime.is_none() {
        return Ok(None);
    }
    crate::runtime_api::with_runtime(state, |store| {
        Ok(store
            .session(session)
            .and_then(|tracked| tracked.agent.clone()))
    })
}
