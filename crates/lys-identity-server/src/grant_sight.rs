//! What a caller may see of the grants: a grant it holds, issued or answers
//! for, a grant derived from one of those, or any grant as the root
//! authority, and the refusals it may read whole.

use std::collections::HashMap;
use std::str::FromStr;

use lys_identity::grants::admission::effective;
use lys_identity::grants::lineage::MAX_DEPTH;
use lys_identity::grants::{GrantError, GrantId, GrantRecord, Source};
use lys_identity::{IdentityError, IdentityId, PersonId};

use crate::error::ServerError;
use crate::grant_contract::{GrantView, RefusedView, StandingView};
use crate::grants::Judged;

pub(crate) fn is_root(caller: IdentityId, root: PersonId) -> bool {
    caller == IdentityId::Person(root)
}

/// Whether `caller` may see the grant `record`.
pub(crate) fn sees(judged: &Judged<'_>, caller: IdentityId, record: &GrantRecord) -> bool {
    sees_with(judged, caller, record, &mut HashMap::new())
}

/// Whether `caller` may see the grant `record`, reading and filling `known`
/// with the answer for every grant on the chain it walks, so a question over
/// many grants walks each chain once.
pub(crate) fn sees_with(
    judged: &Judged<'_>,
    caller: IdentityId,
    record: &GrantRecord,
    known: &mut HashMap<GrantId, bool>,
) -> bool {
    if is_root(caller, judged.root) {
        return true;
    }
    let book = judged.grants.book();
    let mut walked = Vec::new();
    let mut next = Some(record.grant());
    let mut seen = false;
    while let Some(grant) = next {
        if let Some(answer) = known.get(&grant.id()) {
            seen = *answer;
            break;
        }
        walked.push(grant.id());
        let parts = grant.parts();
        if parts.holder == caller
            || parts.issuer == caller
            || IdentityId::Person(parts.responsible) == caller
        {
            seen = true;
            break;
        }
        next = match parts.source {
            Source::Grant(id) if walked.len() <= MAX_DEPTH => book.grant(id),
            Source::Grant(_) | Source::Root => None,
        };
    }
    for id in walked {
        known.insert(id, seen);
    }
    seen
}

/// Whether `caller` may see `identity`'s records: its own, an agent it answers for, or any as the root authority.
pub(crate) fn sees_identity(judged: &Judged<'_>, caller: IdentityId, identity: IdentityId) -> bool {
    identity == caller
        || is_root(caller, judged.root)
        || judged
            .directory
            .record(identity)
            .and_then(lys_identity::projection::Record::responsible)
            .is_some_and(|person| IdentityId::Person(person) == caller)
}

/// Whether `id` is on the authority path of a grant `caller` holds, where a
/// why-permitted answer already names it.
pub(crate) fn on_callers_path(judged: &Judged<'_>, caller: IdentityId, id: GrantId) -> bool {
    let book = judged.grants.book();
    book.held_by(caller).any(|record| {
        let mut next = Some(record.grant());
        let mut hops = 0;
        while let Some(grant) = next {
            if grant.id() == id {
                return true;
            }
            hops += 1;
            next = match grant.source() {
                Source::Grant(source) if hops <= MAX_DEPTH => book.grant(source),
                Source::Grant(_) | Source::Root => None,
            };
        }
        false
    })
}

/// Whether a refusal may name the grant `text` to `caller`.
pub(crate) fn grant_seen(judged: &Judged<'_>, caller: IdentityId, text: &str) -> bool {
    GrantId::from_str(text).ok().is_some_and(|id| {
        on_callers_path(judged, caller, id)
            || judged
                .grants
                .book()
                .record(id)
                .is_some_and(|record| sees(judged, caller, record))
    })
}

/// Refuse a grant `caller` may not see exactly as a grant the grants do not hold.
pub(crate) fn visible_or(
    judged: &Judged<'_>,
    caller: IdentityId,
    id: GrantId,
    unknown: GrantError,
) -> Result<(), ServerError> {
    match judged.grants.book().record(id) {
        Some(record) if !sees(judged, caller, record) && !on_callers_path(judged, caller, id) => {
            Err(unknown.into())
        }
        Some(_) | None => Ok(()),
    }
}

pub(crate) fn identity_seen(judged: &Judged<'_>, caller: IdentityId, text: &str) -> bool {
    crate::routes::identity_id(text).is_ok_and(|identity| sees_identity(judged, caller, identity))
}

/// The refusal as `caller` may read it: whole when every grant and identity it
/// names is one the caller may see, else its name and the condition alone.
pub(crate) fn as_seen_by(
    judged: &Judged<'_>,
    caller: IdentityId,
    error: GrantError,
) -> ServerError {
    let hidden = match &error {
        GrantError::Revoked { grant }
        | GrantError::Expired { grant, .. }
        | GrantError::NotStarted { grant, .. }
        | GrantError::OperationUnresolved { grant, .. } => !grant_seen(judged, caller, grant),
        GrantError::IdentityNotActive { identity, .. }
        | GrantError::ResponsibleMismatch { identity, .. }
        | GrantError::Identity(IdentityError::IdentityUnknown { identity }) => {
            !identity_seen(judged, caller, identity)
        }
        _ => false,
    };
    if hidden {
        ServerError::Withheld {
            refusal: ServerError::from(error).name(),
        }
    } else {
        error.into()
    }
}

/// The grant a refusal names on the chain it was judged over, if any.
fn named_grant(error: &GrantError) -> Option<String> {
    match error {
        GrantError::Revoked { grant }
        | GrantError::Expired { grant, .. }
        | GrantError::NotStarted { grant, .. }
        | GrantError::OperationUnresolved { grant, .. } => Some(grant.clone()),
        _ => None,
    }
}

/// `record` as `caller` reads it at `at`: whether it stands, judged by
/// admission over its whole chain, and the earliest end on that chain.
pub(crate) fn grant_view(
    judged: &Judged<'_>,
    caller: IdentityId,
    record: &GrantRecord,
    at: u64,
) -> GrantView {
    let id = record.grant().id();
    let book = judged.grants.book();
    let effective_ends_at = book
        .lineage(id)
        .ok()
        .and_then(|lineage| lineage.ends)
        .map(|(ends, _)| ends);
    let standing = match effective(book, judged.directory, id, at) {
        Ok(_) => StandingView {
            stands: true,
            refused: None,
        },
        Err(error) => {
            let named = named_grant(&error);
            let seen = as_seen_by(judged, caller, error);
            let grant = if matches!(seen, ServerError::Withheld { .. }) {
                None
            } else {
                named
            };
            StandingView {
                stands: false,
                refused: Some(RefusedView {
                    refusal: seen.name(),
                    grant,
                    reason: seen.to_string(),
                }),
            }
        }
    };
    GrantView::new(
        record,
        judged.grants.unreported(id),
        standing,
        effective_ends_at,
    )
}
