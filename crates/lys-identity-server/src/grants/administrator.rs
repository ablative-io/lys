//! Who gives from standing: the signed-in administrator; or, since 24
//! October 2026, an agent through its own signed MCP message (relayed to
//! this route, or approving a draft of it) whose organisation standing the
//! model judges. A pass holder never gives from standing: a pass
//! authenticates its holder for its audience, and signs nothing.

use super::Judged;
use crate::error::ServerError;
use crate::grant_contract::{DelegateBody, RecordedView};
use crate::routes::{AppState, signed_in};
use axum::http::{HeaderMap, header};
use lys_identity::IdentityId;
use lys_identity::grants::GrantError;

/// The caller giving from standing: the signed-in administrator, or an
/// agent through its own signed message. Anyone else, and any pass holder,
/// is refused by name.
pub(crate) fn giver(
    state: &AppState,
    headers: &HeaderMap,
    judged: &Judged<'_>,
) -> Result<IdentityId, ServerError> {
    // A pass authenticates its holder for its audience, never a standing gift.
    if headers.contains_key(header::AUTHORIZATION) || headers.contains_key("lys-agent-pass") {
        return Err(GrantError::RootAuthorityRefused {
            caller: "a pass holder".to_owned(),
        }
        .into());
    }
    let actor = signed_in(state, headers)?;
    let caller = super::caller(state, headers, judged.directory)?;
    let administrator = actor.provenance().method().is_person_signed_in()
        && caller == IdentityId::Person(judged.root);
    let agent_by_signature = matches!(caller, IdentityId::Agent(_))
        && crate::routes::admitted_agent(judged.directory, &actor)?;
    if !administrator && !agent_by_signature {
        return Err(GrantError::RootAuthorityRefused {
            caller: caller.to_string(),
        }
        .into());
    }
    Ok(caller)
}

pub(super) fn delegate(
    state: &AppState,
    headers: &HeaderMap,
    body: &DelegateBody,
    judged: &mut Judged<'_>,
) -> Result<RecordedView, ServerError> {
    let caller = giver(state, headers, judged)?;
    let request = body.standing_request(caller)?;
    judged.apps.admit_kind(None, request.resource.kind())?;
    super::handlers::refuse_held_on_hot(judged, &request.resource, &request.relation, body.mode())?;
    match judged.grants.delegate_administrator(
        judged.directory,
        &request,
        body.mode(),
        crate::session::now(),
    ) {
        Ok(recorded) => Ok(RecordedView::from(&recorded)),
        Err(error) => Err(crate::grant_sight::as_seen_by(judged, caller, error)),
    }
}
