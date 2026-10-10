//! Who may import into a seat (AGENTS-003 R4): action `seat.import` on the
//! seat resource in Lys's own app schema, asked of the grant engine live
//! with the caller's identity before anything is reserved. A permit is
//! exercised, so the grant log keeps its use as the import's grant receipt.
//!
//! A confirmation is one signed person's: a caller on an agent's pass, or
//! any caller that is not a person, is refused
//! `import_confirmer_not_person`; a seat whose agent nobody answers for is
//! refused `import_no_responsible`. A person holding no grant of
//! `seat.import` is still admitted when they administer Lys or answer for
//! the seat's agent, as every seat act admits them, and the confirmation
//! says it stood on that. Anyone else is refused `not_permitted`, naming
//! the action and the request URL. The action grants the import alone:
//! never `seat.start`, a secret's move or a source's deletion.
//!
//! A dry run reads and changes nothing, so it asks no grant: it is shown to
//! a signed person who administers Lys or answers for the seat's agent.

use axum::http::{HeaderMap, StatusCode};
use lys_identity::IdentityId;
use lys_identity::grants::{Action, ExerciseRequest, Resource, Route};

use crate::error::ServerError;
use crate::error_seat_import::SeatImportError;
use crate::grants::{Decision, decide, with_grants};
use crate::routes::AppState;
use crate::seat_import_state::Confirmation;
use crate::seats_acts::{KIND, Standing, standing};
use crate::seats_state::Seat;
use crate::session::now;

/// The action that imports into a seat.
pub const IMPORT: &str = "seat.import";

/// The caller as a person answering for `seat`, refused by name when the
/// caller is not a person or nobody answers for the seat's agent.
fn person(state: &AppState, headers: &HeaderMap, seat: &Seat) -> Result<Standing, ServerError> {
    let standing = standing(state, headers, seat)?;
    if standing.pass || !matches!(standing.asker, IdentityId::Person(_)) {
        return Err(SeatImportError::ConfirmerNotPerson {
            caller: standing.asker.to_string(),
        }
        .into());
    }
    if standing.responsible.is_none() {
        return Err(SeatImportError::NoResponsible {
            seat: seat.added.name.clone(),
            agent: seat.added.agent.clone(),
        }
        .into());
    }
    Ok(standing)
}

/// Admit a dry run of `seat` for the caller of `headers`.
pub(crate) fn may_preview(
    state: &AppState,
    headers: &HeaderMap,
    seat: &Seat,
    url: &str,
) -> Result<(), ServerError> {
    let standing = person(state, headers, seat)?;
    if standing.administrator || standing.is_responsible {
        return Ok(());
    }
    Err(ServerError::NotPermitted {
        reason: format!(
            "{} neither answers for the agent of seat {} nor administers Lys: POST {url} is refused",
            standing.asker, seat.added.name
        ),
    })
}

/// Admit the confirmation of an import into `seat` for the caller of
/// `headers`, asking the grant engine live and exercising its permit.
pub(crate) fn confirmed_by(
    state: &AppState,
    headers: &HeaderMap,
    seat: &Seat,
    url: &str,
) -> Result<Confirmation, ServerError> {
    let standing = person(state, headers, seat)?;
    let exercised = with_grants(state, |mut judged| {
        judged.apps.admit_kind(None, KIND)?;
        judged.apps.admit_action(KIND, IMPORT)?;
        let request = ExerciseRequest {
            caller: standing.asker,
            route: Route::Api,
            resource: Resource::new(KIND, &seat.added.name)?,
            action: Action::new(IMPORT)?,
        };
        match decide(&mut judged, &request, now(), None, Decision::Exercise) {
            Ok((permit, _)) => match permit.use_event {
                Some(Ok(index)) => Ok(Some((permit.grant.to_string(), index))),
                Some(Err(error)) => Err(error.into()),
                None => Err(lys_identity::grants::GrantError::LogUnavailable {
                    reason: format!("the exercise of {IMPORT} has no recorded use event"),
                }
                .into()),
            },
            Err(error) if crate::error_status::grant_status(&error) == StatusCode::FORBIDDEN => {
                Ok(None)
            }
            Err(error) => Err(error.into()),
        }
    })?;
    let confirmation = |by: &str, grant: Option<String>, use_event: Option<u64>| Confirmation {
        person: standing.asker.to_string(),
        by: by.to_owned(),
        grant,
        use_event,
    };
    match exercised {
        Some((grant, index)) => Ok(confirmation("grant", Some(grant), Some(index))),
        None if standing.administrator => Ok(confirmation("administrator", None, None)),
        None if standing.is_responsible => Ok(confirmation("responsible", None, None)),
        None => Err(ServerError::NotPermitted {
            reason: format!(
                "{} holds no grant of {IMPORT} on seat {}, and neither answers for its agent nor administers Lys: POST {url} is refused",
                standing.asker, seat.added.name
            ),
        }),
    }
}
