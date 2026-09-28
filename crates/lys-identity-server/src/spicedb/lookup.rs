//! Who can do a thing: the reverse question behind DIRECTORY-006 R5's
//! explain seam, answered by `SpiceDB`'s `LookupSubjects`.
//!
//! This is the only place a `LookupSubjects` request is built. It computes
//! no verdict of its own and is no second evaluator: an identity is in the
//! answer exactly when `SpiceDB` resolves it for the permission, read at the
//! same revision as the question set's forward checks. When the set has no
//! revision yet, the first lookup reads at least as fresh as the projector's
//! token and fixes the set's revision at the one it answered at; every later
//! lookup of the set reads at that exact snapshot. DIRECTORY-006 R5 keeps the
//! question's visibility and paging.

use std::str::FromStr;

use lys_identity::grants::relationships::{EXERCISE, resource_object};
use lys_identity::grants::{Action, GrantError, Resource};
use lys_identity::{AgentId, IdentityId, PersonId};

use super::check::Evaluator;
use super::explain::QuestionSet;
use super::projector::{Reached, number, object_reference};
use super::wire::authzed::api::v1::consistency::Requirement;
use super::wire::authzed::api::v1::lookup_subjects_request::WildcardOption;
use super::wire::authzed::api::v1::{
    Consistency, LookupPermissionship, LookupSubjectsRequest, LookupSubjectsResponse, ZedToken,
};

/// The subject lookup the client sends.
pub type LookupQuestion = LookupSubjectsRequest;
/// One answer to a subject lookup.
pub type LookupAnswer = LookupSubjectsResponse;

/// The kinds of identity a lookup resolves, in the order they are asked.
const KINDS: [&str; 2] = ["agent", "person"];

fn identity(kind: &str, id: &str) -> Option<IdentityId> {
    match kind {
        "agent" => AgentId::from_str(id).ok().map(IdentityId::Agent),
        "person" => PersonId::from_str(id).ok().map(IdentityId::Person),
        _ => None,
    }
}

/// Every identity `SpiceDB` resolves for `action` on `resource`, read under
/// `set`, with the projector at `reached`.
pub fn who(
    evaluator: &Evaluator,
    reached: &Reached,
    set: &mut QuestionSet,
    resource: &Resource,
    action: &Action,
) -> Result<Vec<IdentityId>, GrantError> {
    let mut found = Vec::new();
    for kind in KINDS {
        let requirement = match (set.revision(), reached.token.as_deref()) {
            (Some(at), _) => Requirement::AtExactSnapshot(ZedToken {
                token: at.to_owned(),
            }),
            (None, Some(projected)) => Requirement::AtLeastAsFresh(ZedToken {
                token: projected.to_owned(),
            }),
            (None, None) => Requirement::FullyConsistent(true),
        };
        let mut fields = std::collections::BTreeMap::new();
        fields.insert("now".to_owned(), number(evaluator.now()));
        let answers = evaluator.client().lookup(LookupSubjectsRequest {
            consistency: Some(Consistency {
                requirement: Some(requirement),
            }),
            resource: Some(object_reference(&resource_object(resource, action))),
            permission: EXERCISE.to_owned(),
            subject_object_type: kind.to_owned(),
            optional_subject_relation: String::new(),
            context: Some(prost_types::Struct { fields }),
            optional_concrete_limit: 0,
            optional_cursor: None,
            wildcard_option: WildcardOption::ExcludeWildcards.into(),
        })?;
        for answer in answers {
            if let Some(looked_up_at) = &answer.looked_up_at {
                set.fix(&looked_up_at.token);
            }
            let Some(subject) = answer.subject else {
                continue;
            };
            let permitted = LookupPermissionship::try_from(subject.permissionship)
                .is_ok_and(|permissionship| permissionship == LookupPermissionship::HasPermission);
            if let Some(identity) = identity(kind, &subject.subject_object_id)
                && permitted
            {
                found.push(identity);
            }
        }
    }
    found.sort_by_key(ToString::to_string);
    found.dedup();
    Ok(found)
}
