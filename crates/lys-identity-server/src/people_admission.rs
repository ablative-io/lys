//! Admitting a person is judged by the `organisation.admit` grant on the
//! caller's own organisation (ACCESS-009 R1), through the grants engine as
//! a route grant is judged (`route_actions`), never by
//! `routes::administrator`. POST /people/admit asks the question before
//! its first step and POST /people exercises the grant under the same hold
//! as its write, so one act records one use. A body naming another
//! organisation is refused by name before the grants are read.

use std::str::FromStr;

use lys_identity::grants::shipped::ORGANISATION_GRANT;
use lys_identity::grants::{Action, ExerciseRequest, GrantError, Resource, Route};
use lys_identity::{Actor, IdentityId, OrganisationId};
use lys_log_store::LeafStore;

use crate::error::ServerError;
use crate::grants::{Decision, Judged};

/// The act every admission exercises.
pub(crate) const ADMIT: &str = "organisation.admit";

/// The organisation `actor` admits into: their own, once they are active
/// and hold [`ADMIT`] on it. `named` is the organisation the body names,
/// refused `organisation_not_own` unless it is the caller's. `decision`
/// says whether the permit is recorded as a use.
///
/// # Errors
/// Refuses an unknown or inactive caller, one under no organisation, a body
/// naming another organisation, a caller without the grant (`NotAdmitted`,
/// naming the act), or whatever reading the grants refuses.
pub(crate) fn organisation<S: LeafStore>(
    judged: &mut Judged<'_, S>,
    actor: &Actor,
    named: Option<&str>,
    decision: Decision,
) -> Result<OrganisationId, ServerError> {
    // A login bound to no person is not admitted to admit: NotAdmitted, as
    // every mutation route answers a stranger, never the reads' NoPerson.
    let (caller, organisation) = crate::caller_admission::caller_within(judged.directory, actor)
        .map_err(|error| match error {
            ServerError::NoPerson => ServerError::NotAdmitted {
                reason: "the caller's login is bound to no person, so no organisation admits through it",
            },
            other => other,
        })?;
    if let Some(named) = named {
        let asked = OrganisationId::from_str(named)?;
        if asked != organisation {
            return Err(ServerError::OrganisationNotOwn {
                organisation: named.to_owned(),
            });
        }
    }
    let resource = Resource::new("organisation", &organisation.to_string())?;
    judged.apps.admit_kind(None, resource.kind())?;
    // An agent admits only holding the right to give the first grant as well
    // (AGENTS-019 amendment, 22 October 2026), asked first and recorded by
    // the grant it gives; a person's one right stands.
    if matches!(caller, IdentityId::Agent(_)) {
        asked(
            judged,
            (caller, &resource, ORGANISATION_GRANT),
            Decision::Explain,
            "an agent admitting a person needs organisation.admit and organisation.grant on its own organisation",
        )?;
    }
    asked(
        judged,
        (caller, &resource, ADMIT),
        decision,
        "admitting a person needs organisation.admit on the caller's own organisation",
    )?;
    Ok(organisation)
}

/// Decide `act` on `resource` for `caller`, refused `NotAdmitted` for
/// `refused` when no live grant of the caller's carries it.
fn asked<S: LeafStore>(
    judged: &mut Judged<'_, S>,
    (caller, resource, act): (IdentityId, &Resource, &str),
    decision: Decision,
    refused: &'static str,
) -> Result<(), ServerError> {
    judged.apps.admit_action(resource.kind(), act)?;
    let request = ExerciseRequest {
        caller,
        route: Route::Api,
        resource: resource.clone(),
        action: Action::new(act)?,
    };
    let at = crate::session::now();
    match crate::grants::decide(judged, &request, at, None, decision) {
        Ok(_) => Ok(()),
        Err(
            GrantError::NotHeld { .. } | GrantError::Revoked { .. } | GrantError::Expired { .. },
        ) => Err(ServerError::NotAdmitted { reason: refused }),
        Err(error) => Err(error.into()),
    }
}
