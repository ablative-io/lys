//! A seat's start, stop, restart and message are deliberate acts on the
//! seat resource (AGENTS-002 R3, R6): actions `seat.start`, `seat.stop`,
//! `seat.restart` and `seat.send` in Lys's own app schema, each asked of
//! the grant engine live with the caller's identity before anything is
//! done. A permit is exercised, so the grant log keeps its use as the act's
//! grant receipt, and the answer names the grant and that use.
//!
//! A caller holding no grant for the act is still admitted when it is the
//! administrator or the person responsible for the seat's agent, as every
//! act on an agent's sessions admits them, and the answer says it stood on
//! that, not on a grant. An agent's pass was already judged, and its use
//! recorded, by the guarded router on the route's declared scope, so it is
//! not asked again. Anyone else is refused `not_permitted`, naming the
//! action and the request URL.
//!
//! Each act the runner is asked for is kept as a runtime act receipt naming
//! who asked, whatever the runner answered, as the runner routes keep theirs.

use std::str::FromStr;
use std::sync::Arc;

use axum::http::{HeaderMap, StatusCode};
use lys_identity::grants::{Action, ExerciseRequest, Resource, Route};
use lys_identity::{AgentId, AuthMethod, IdentityId};
use lys_runner::{Act, Answer};
use serde::Serialize;

use crate::error::ServerError;
use crate::grants::{Decision, caller, decide, with_grants};
use crate::routes::{AppState, signed_in, with_directory};
use crate::runner_acts::{ActReceipt, Digested, RunnerAct};
use crate::runner_sessions::{Driven, ended_in, keep_act, kind, record_end};
use crate::seats_state::Seat;
use crate::session::now;

/// The kind of resource a seat is in the grants.
pub const KIND: &str = "seat";
/// The action that starts a seat.
pub const START: &str = "seat.start";
/// The action that stops a seat.
pub const STOP: &str = "seat.stop";
/// The action that restarts a seat.
pub const RESTART: &str = "seat.restart";
/// The action that sends a seat a message.
pub const SEND: &str = "seat.send";

/// What admitted a seat act.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
pub struct SeatAdmission {
    /// The action asked.
    pub action: String,
    /// The identity that asked.
    pub caller: String,
    /// What it stood on: `grant`, `pass`, `administrator` or `responsible`.
    pub by: String,
    /// The grant exercised, when it stood on one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grant: Option<String>,
    /// The grant log's index of the use event recording the exercise.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_event: Option<u64>,
}

/// Who answers for a seat's agent, as the directory holds it now.
pub(crate) struct Standing {
    /// The caller, as the directory names it.
    pub(crate) asker: IdentityId,
    /// Whether the caller is the administrator.
    pub(crate) administrator: bool,
    /// The person responsible for the seat's agent, when one is.
    pub(crate) responsible: Option<String>,
    /// Whether the caller is that person.
    pub(crate) is_responsible: bool,
    /// Whether the caller came on an agent's pass.
    pub(crate) pass: bool,
}

/// The caller of a seat request and how it stands to the seat's agent.
pub(crate) fn standing(
    state: &AppState,
    headers: &HeaderMap,
    seat: &Seat,
) -> Result<Standing, ServerError> {
    let actor = signed_in(state, headers)?;
    let agent = AgentId::from_str(&seat.added.agent)?;
    with_directory(state, |directory| {
        let projection = directory.projection()?;
        let asker = caller(state, headers, projection)?;
        let record = projection
            .record(IdentityId::Agent(agent))
            .ok_or(ServerError::AgentNotVisible)?;
        let responsible = record.responsible();
        Ok(Standing {
            asker,
            administrator: state.admission.is_administrator(projection, &actor)?,
            responsible: responsible.map(|person| person.to_string()),
            is_responsible: matches!(
                asker,
                IdentityId::Person(person) if responsible == Some(person)
            ),
            pass: matches!(actor.provenance().method(), AuthMethod::AgentPass(_)),
        })
    })
}

/// Admit `action` on `seat` for the caller of `headers`, asking the grant
/// engine live and exercising its permit; refused `not_permitted` naming
/// the action and `url` when nothing admits it.
pub(crate) fn admitted(
    state: &AppState,
    headers: &HeaderMap,
    seat: &Seat,
    action: &str,
    url: &str,
) -> Result<(SeatAdmission, Standing), ServerError> {
    let standing = standing(state, headers, seat)?;
    let admission = |by: &str, grant: Option<String>, use_event: Option<u64>| SeatAdmission {
        action: action.to_owned(),
        caller: standing.asker.to_string(),
        by: by.to_owned(),
        grant,
        use_event,
    };
    if standing.pass {
        let admission = admission("pass", None, None);
        return Ok((admission, standing));
    }
    let exercised = with_grants(state, |mut judged| {
        judged.apps.admit_kind(None, KIND)?;
        judged.apps.admit_action(KIND, action)?;
        let request = ExerciseRequest {
            caller: standing.asker,
            route: Route::Api,
            resource: Resource::new(KIND, &seat.added.name)?,
            action: Action::new(action)?,
        };
        match decide(&mut judged, &request, now(), None, Decision::Exercise) {
            Ok((permit, _)) => match permit.use_event {
                Some(Ok(index)) => Ok(Some((permit.grant.to_string(), index))),
                Some(Err(error)) => Err(error.into()),
                None => Err(lys_identity::grants::GrantError::LogUnavailable {
                    reason: format!("the exercise of {action} has no recorded use event"),
                }
                .into()),
            },
            Err(error) if crate::error_status::grant_status(&error) == StatusCode::FORBIDDEN => {
                Ok(None)
            }
            Err(error) => Err(error.into()),
        }
    })?;
    let admission = match exercised {
        Some((grant, index)) => admission("grant", Some(grant), Some(index)),
        None if standing.administrator => admission("administrator", None, None),
        None if standing.is_responsible => admission("responsible", None, None),
        None => {
            return Err(ServerError::NotPermitted {
                reason: format!(
                    "{} holds no grant of {action} on seat {}, and neither answers for its agent nor administers Lys: POST {url} is refused",
                    standing.asker, seat.added.name
                ),
            });
        }
    };
    Ok((admission, standing))
}

/// One session of the runner's own knowledge (the contract's `LivenessView`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Live {
    pub(crate) session: String,
    pub(crate) pid: Option<u32>,
    pub(crate) alive: bool,
    pub(crate) managed: bool,
    pub(crate) harness_session: Option<String>,
    pub(crate) turn_active: bool,
    pub(crate) last_signal_at: Option<u64>,
    pub(crate) started_at: u64,
    pub(crate) ended: bool,
}

/// What `machine`'s runner knows of `session`, or of every session it
/// holds when none is named.
pub(crate) async fn liveness(
    state: &Arc<AppState>,
    machine: &str,
    session: Option<String>,
) -> Result<Vec<Live>, ServerError> {
    let runner = crate::runner_sessions::machine_runner(state, machine)?.ok_or_else(|| {
        ServerError::RunnerAbsent {
            machine: machine.to_owned(),
        }
    })?;
    match crate::runner_client::ask(state, machine, runner, Act::Liveness { session }).await? {
        Answer::Liveness { sessions } => Ok(sessions
            .into_iter()
            .map(|view| Live {
                session: view.session,
                pid: view.pid,
                alive: view.alive,
                managed: view.managed,
                harness_session: view.harness_session,
                turn_active: view.turn_active,
                last_signal_at: view.last_signal_at,
                started_at: view.started_at,
                ended: view.ended,
            })
            .collect()),
        other => Err(ServerError::Runner {
            refusal: "runner_reply_malformed".to_owned(),
            words: format!("a liveness read was answered {}", kind(&other)),
        }),
    }
}

/// What the runner knows of the one session `session` on `machine`; none
/// when it holds no such session.
pub(crate) async fn live_session(
    state: &Arc<AppState>,
    machine: &str,
    session: &str,
) -> Result<Option<Live>, ServerError> {
    Ok(liveness(state, machine, Some(session.to_owned()))
        .await?
        .into_iter()
        .find(|live| live.session == session))
}

/// Ask `act` of `driven`'s runner for `caller`, keep its receipt under
/// `name` whatever the runner answered, and answer the runner's answer
/// beside the receipt. Input and operations are asked as the caller.
pub(crate) async fn act(
    state: &Arc<AppState>,
    (driven, caller, name): (&Driven, &str, &str),
    text: Option<Digested>,
    act: Act,
) -> Result<(Answer, ActReceipt), ServerError> {
    let act = if matches!(act, Act::Input { .. } | Act::Operate { .. }) {
        Act::AsCaller {
            caller: caller.to_owned(),
            done: Box::new(act),
        }
    } else {
        act
    };
    let answered =
        crate::runner_client::ask(state, &driven.machine, driven.runner.clone(), act).await;
    if let Some(ended) = answered.as_ref().ok().and_then(ended_in) {
        record_end(state, driven, ended)?;
    }
    let outcome = answered
        .as_ref()
        .map_or_else(ServerError::name, |answer| kind(answer).to_owned());
    let receipt = keep_act(
        state,
        RunnerAct {
            act: name.to_owned(),
            caller: caller.to_owned(),
            session: driven.session.clone(),
            agent: driven.agent.clone(),
            machine: driven.machine.clone(),
            at: now(),
            text,
            keys: Vec::new(),
            outcome,
        },
    )?;
    Ok((answered?, receipt))
}
