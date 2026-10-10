//! Seat supervision (AGENTS-004): a seat started managed is bound to an
//! owner of its own, and the owners every runner holds are listed, live
//! and unreachable, as the runner proved them at the kernel.

use std::collections::BTreeSet;
use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::http::HeaderMap;
use lys_runner::seat_owner::protocol::OwnerBinding;
use lys_runner::{Act, Answer};
use serde::Serialize;

use crate::error::ServerError;
use crate::routes::{AppState, signed_in};
use crate::seats_api::with_seats;

/// The first lease generation of a freshly started owner. A restart of
/// the seat starts a new owner, whose lease begins again: the old owner's
/// record is in its own directory, not reused.
pub const FIRST_GENERATION: u64 = 1;

/// Binds a managed start to an owner when `agent` on `machine` holds a
/// seat: the seat's name, the session being started, the conversation the
/// transport chose and the first generation. A start that is not managed,
/// or for no seat, is returned as it was.
///
/// # Errors
///
/// `seats_unavailable` when the seat store cannot be read.
pub(crate) fn bind_owner(
    state: &AppState,
    agent: &str,
    machine: &str,
    act: Act,
) -> Result<Act, ServerError> {
    let Act::StartManaged {
        mut managed,
        lys_mcp,
        proxy,
    } = act
    else {
        return Ok(act);
    };
    let seat = with_seats(state, |store| {
        Ok(store
            .seats()
            .iter()
            .find(|seat| seat.added.agent == agent && seat.added.machine == machine)
            .map(|seat| seat.added.name.clone()))
    })?;
    if let Some(seat) = seat {
        let binding = OwnerBinding {
            seat,
            session: managed.launch.session.clone(),
            conversation: managed.conversation.clone(),
            generation: FIRST_GENERATION,
        };
        binding.validate()?;
        managed.owner = Some(binding);
    }
    Ok(Act::StartManaged {
        managed,
        lys_mcp,
        proxy,
    })
}

/// One owner a runner started, as the runner proved it.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct OwnedSeatView {
    /// The seat.
    pub seat: String,
    /// The session the owner serves.
    pub session: String,
    /// The conversation the seat is bound to.
    pub conversation: String,
    /// The lease generation.
    pub generation: u64,
    /// The machine whose runner started it.
    pub machine: String,
    /// The build of the owner binary serving it.
    pub build: String,
    /// The owner process.
    pub owner_pid: u32,
    /// The owner process's start identity at the kernel.
    pub owner_start: String,
    /// When the owner became ready, milliseconds since the epoch.
    pub established_at: u64,
    /// Why the owner cannot be reached, when it cannot.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<Object>)]
    pub unreachable: Option<lys_runner::seat_owner::recovery::Unreachable>,
}

/// Every owner, and whether every runner was read.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct OwnedSeats {
    /// The owners, live first within each machine.
    pub owners: Vec<OwnedSeatView>,
    /// Whether every runner answered.
    pub runner: &'static str,
    /// Why one did not, when one did not.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

fn view(machine: &str, seat: &lys_runner::seat_owner::sessions::OwnedSeat) -> OwnedSeatView {
    OwnedSeatView {
        seat: seat.binding.seat.clone(),
        session: seat.binding.session.clone(),
        conversation: seat.binding.conversation.clone(),
        generation: seat.binding.generation,
        machine: machine.to_owned(),
        build: seat.endpoint.build.clone(),
        owner_pid: seat.endpoint.owner.pid,
        owner_start: seat.endpoint.owner.start.0.clone(),
        established_at: seat.established_at,
        unreachable: None,
    }
}

/// GET /seats/owned: every owner each seat's runner started.
pub(crate) async fn owned(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<OwnedSeats>, ServerError> {
    signed_in(&state, &headers)?;
    let machines: BTreeSet<String> = with_seats(&state, |store| {
        Ok(store
            .seats()
            .iter()
            .map(|seat| seat.added.machine.clone())
            .collect())
    })?;
    let mut owners = Vec::new();
    let mut unread = Vec::new();
    for machine in machines {
        let Some(runner) = crate::runner_sessions::machine_runner(&state, &machine)? else {
            unread.push(format!("machine {machine} names no runner"));
            continue;
        };
        match crate::runner_client::ask(&state, &machine, runner, Act::Owned).await {
            Ok(Answer::Owned {
                owners: live,
                unreachable,
            }) => {
                owners.extend(live.iter().map(|seat| view(&machine, seat)));
                owners.extend(unreachable.into_iter().map(|found| OwnedSeatView {
                    unreachable: found.unreachable,
                    ..view(&machine, &found.seat)
                }));
            }
            Ok(other) => {
                return Err(ServerError::AuthorityUnavailable {
                    reason: format!("the runner of machine {machine} answered {other:?} to owned"),
                });
            }
            Err(error) => unread.push(format!(
                "the runner of machine {machine} could not be read: {error}"
            )),
        }
    }
    Ok(Json(OwnedSeats {
        owners,
        runner: if unread.is_empty() { "read" } else { "unknown" },
        reason: (!unread.is_empty()).then(|| unread.join("; ")),
    }))
}
