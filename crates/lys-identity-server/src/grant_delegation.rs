//! The delegation form question, using the same locked grant authority and visibility.

use crate::error::ServerError;
use crate::grant_contract::{CannotGiveAnswer, CannotGiveBody};
use crate::grant_sight::{as_seen_by, sees_identity};
use crate::grants::{Judged, caller, with_grants};
use crate::routes::AppState;
use crate::session::now;
use axum::Json;
use axum::extract::{Query, State};
use axum::http::HeaderMap;
use lys_identity::{IdentityError, IdentityId};
use std::sync::Arc;

/// Whether `caller` may name `recipient` on the delegation form: a person the
/// directory records, since the policy admits people as recipients, or an
/// agent the caller may see.
fn names_recipient(judged: &Judged<'_>, caller: IdentityId, recipient: IdentityId) -> bool {
    judged.directory.record(recipient).is_some()
        && (matches!(recipient, IdentityId::Person(_)) || sees_identity(judged, caller, recipient))
}

/// What the caller cannot give the recipient from the source grant, each
/// item with its one reason, as the grants' one authority owner lists it.
/// The question is a read, asked in the query: `route`, `source` and `recipient`.
pub(crate) async fn cannot_give(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(body): Query<CannotGiveBody>,
) -> Result<Json<CannotGiveAnswer>, ServerError> {
    with_grants(&state, |judged| {
        let caller = caller(&state, &headers, judged.directory)?;
        let request = body.request(caller)?;
        judged
            .grants
            .book()
            .grant(request.source)
            .filter(|source| source.holder() == caller)
            .ok_or(ServerError::GrantNotVisible)?;
        if !names_recipient(&judged, caller, request.recipient) {
            let unknown = ServerError::from(IdentityError::IdentityUnknown {
                identity: request.recipient.to_string(),
            });
            return Err(ServerError::Withheld {
                refusal: unknown.name(),
            });
        }
        let at = now();
        match judged.grants.cannot_give(judged.directory, &request, at) {
            Ok(list) => Ok(Json(CannotGiveAnswer::from(&list))),
            Err(error) => Err(as_seen_by(&judged, caller, error)),
        }
    })
}
