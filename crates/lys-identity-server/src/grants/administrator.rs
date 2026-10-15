//! Only the signed-in administrator may give from standing.

use super::Judged;
use crate::error::ServerError;
use crate::grant_contract::{DelegateBody, RecordedView};
use crate::routes::{AppState, signed_in};
use axum::http::{HeaderMap, header};
use lys_identity::grants::GrantError;
use lys_identity::{AuthMethod, IdentityId};

pub(super) fn delegate(
    state: &AppState,
    headers: &HeaderMap,
    body: &DelegateBody,
    judged: &mut Judged<'_>,
) -> Result<RecordedView, ServerError> {
    // A pass authenticates its holder for its audience, never a standing gift.
    if headers.contains_key(header::AUTHORIZATION) || headers.contains_key("lys-agent-pass") {
        return Err(GrantError::RootAuthorityRefused {
            caller: "a pass holder".to_owned(),
        }
        .into());
    }
    let actor = signed_in(state, headers)?;
    let caller = super::caller(state, headers, judged.directory)?;
    if actor.provenance().method() != AuthMethod::Oidc || caller != IdentityId::Person(judged.root)
    {
        return Err(GrantError::RootAuthorityRefused {
            caller: caller.to_string(),
        }
        .into());
    }
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
