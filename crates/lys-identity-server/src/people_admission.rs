//! Admitting a person is judged by the `organisation.admit` grant on the
//! caller's own organisation (ACCESS-009 R1), through the grants engine as
//! a route grant is judged (`route_actions`), never by
//! `routes::administrator`. POST /people/admit asks the question before
//! its first step and POST /people exercises the grant under the same hold
//! as its write, so one act records one use. A body naming another
//! organisation is refused by name before the grants are read.

use std::str::FromStr;

use lys_identity::grants::{Action, ExerciseRequest, GrantError, Resource, Route};
use lys_identity::{Actor, OrganisationId};
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
    let (caller, organisation) = crate::caller_admission::caller_within(judged.directory, actor)?;
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
    judged.apps.admit_action(resource.kind(), ADMIT)?;
    let request = ExerciseRequest {
        caller,
        route: Route::Api,
        resource,
        action: Action::new(ADMIT)?,
    };
    let at = crate::session::now();
    match crate::grants::decide(judged, &request, at, None, decision) {
        Ok(_) => Ok(organisation),
        Err(
            GrantError::NotHeld { .. } | GrantError::Revoked { .. } | GrantError::Expired { .. },
        ) => Err(ServerError::NotAdmitted {
            reason: "admitting a person needs organisation.admit on the caller's own organisation",
        }),
        Err(error) => Err(error.into()),
    }
}
