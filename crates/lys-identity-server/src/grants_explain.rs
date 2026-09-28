//! Road step 2 behind DIRECTORY-006 R5's explain seam: why an identity can,
//! or cannot, do a thing, and who can, answered by `SpiceDB`.
//!
//! `POST /grants/explain` asks why of any identity the caller may see: its
//! own, an agent it answers for, or any as the root authority. The answer is
//! the one evaluator's traced verdict with its path or named reason and the
//! policy it was read under, and a body naming a `revision` an earlier answer
//! was read at is read at that exact snapshot, so a screen asks one question
//! set at one revision. The who-can page keeps DIRECTORY-006 R5's visibility
//! and paging over the holders `SpiceDB` resolves. A configuration that
//! names no gRPC address for `SpiceDB` has no explanation, and says so by
//! name.

use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::http::HeaderMap;
use lys_identity::IdentityId;
use lys_identity::grants::permission::resting;
use lys_identity::grants::{Action, GrantError, Permit, Question, Resource};
use serde::{Deserialize, Serialize};

use crate::error::ServerError;
use crate::grant_contract::{HolderView, PermitView, ResourceView, WhoPage};
use crate::grant_sight::{as_seen_by, identity_seen, sees_with};
use crate::grants::{Judged, caller, with_grants};
use crate::routes::{AppState, identity_id};
use crate::spicedb::SpiceDbError;
use crate::spicedb::engine::Engine;
use crate::spicedb::explain::{Explanation, QuestionSet, Verdict, why};
use crate::spicedb::lookup::who;

/// A resource, by kind and id.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ResourceBody {
    kind: String,
    id: String,
}

/// Why an identity can or cannot do a thing.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExplainBody {
    /// The identity asked about; the caller when absent.
    #[serde(default)]
    identity: Option<String>,
    /// The resource.
    resource: ResourceBody,
    /// The action.
    action: String,
    /// A revision an earlier answer of the same question set was read at.
    #[serde(default)]
    revision: Option<String>,
}

/// One identity on the path of a permitted answer.
#[derive(Debug, Clone, Serialize)]
pub struct PathStepView {
    /// The identity.
    pub identity: String,
    /// Whether it is the person responsible.
    pub responsible: bool,
}

/// The policy an answer was made under.
#[derive(Debug, Clone, Serialize)]
pub struct PolicyView {
    /// The SHA-256 of `schema.zed`.
    pub schema_sha256: String,
    /// The revision the answer was read at.
    pub revision: String,
}

/// The answer to why an identity can or cannot do a thing.
#[derive(Debug, Clone, Serialize)]
pub struct ExplainView {
    /// The verdict: the one evaluator's.
    pub permitted: bool,
    /// The identity asked about.
    pub identity: String,
    /// The resource.
    pub resource: ResourceView,
    /// The action.
    pub action: String,
    /// The grant a yes rests on, or the grant a no concerns.
    pub grant: Option<String>,
    /// For a yes, the identities from the identity to the responsible person.
    pub path: Vec<PathStepView>,
    /// For a yes, the person responsible.
    pub responsible: Option<String>,
    /// For a no, the refusal's name.
    pub reason: Option<String>,
    /// The policy the answer was made under.
    pub policy: PolicyView,
}

impl From<&Explanation> for ExplainView {
    fn from(explanation: &Explanation) -> Self {
        let (permitted, grant, path, responsible, reason) = match &explanation.verdict {
            Verdict::Permitted {
                grant,
                path,
                responsible,
            } => {
                let marked = responsible.map(IdentityId::Person);
                let steps = path
                    .iter()
                    .map(|identity| PathStepView {
                        identity: identity.to_string(),
                        responsible: Some(*identity) == marked,
                    })
                    .collect();
                (
                    true,
                    grant.map(|grant| grant.to_string()),
                    steps,
                    responsible.map(|person| person.to_string()),
                    None,
                )
            }
            Verdict::Refused { reason, grant } => {
                (false, grant.clone(), Vec::new(), None, Some(reason.clone()))
            }
        };
        Self {
            permitted,
            identity: explanation.identity.to_string(),
            resource: ResourceView {
                kind: explanation.resource.kind().to_owned(),
                id: explanation.resource.id().to_owned(),
            },
            action: explanation.action.to_string(),
            grant,
            path,
            responsible,
            reason,
            policy: PolicyView {
                schema_sha256: explanation.policy.schema_sha256.clone(),
                revision: explanation.policy.revision.clone(),
            },
        }
    }
}

/// The step-2 engine, or the refusal naming why there is none.
pub(crate) fn engine(state: &AppState) -> Result<&Engine, GrantError> {
    state.grant_setup.engine.as_ref().ok_or_else(|| {
        SpiceDbError::GrpcAbsent {
            reason: state.grant_setup.engine_absent.clone(),
        }
        .into()
    })
}

/// `POST /grants/explain`.
pub(crate) async fn explain(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<ExplainBody>,
) -> Result<Json<ExplainView>, ServerError> {
    let resource = Resource::new(&body.resource.kind, &body.resource.id)?;
    let action = Action::new(&body.action)?;
    with_grants(&state, |judged| {
        let caller = caller(&state, &headers, judged.directory)?;
        let identity = match &body.identity {
            Some(text) if !identity_seen(&judged, caller, text) => {
                return Err(ServerError::Withheld {
                    refusal: "IdentityNotVisible".to_owned(),
                });
            }
            Some(text) => identity_id(text)?,
            None => caller,
        };
        let engine = engine(&state)?;
        let reached = engine.catch_up(judged.grants, judged.root)?;
        let mut set = body
            .revision
            .clone()
            .map_or_else(QuestionSet::new, QuestionSet::at);
        let question = Question {
            subject: identity,
            resource: &resource,
            action: &action,
        };
        let book = judged.grants.book();
        match why(
            engine.evaluator(),
            book,
            judged.grants.revision(),
            &reached,
            &mut set,
            &question,
        ) {
            Ok(explanation) => Ok(Json(ExplainView::from(&explanation))),
            Err(error) => Err(as_seen_by(&judged, caller, error)),
        }
    })
}

/// One page of who can perform `action` on `resource`, as `SpiceDB`
/// resolves it, holding only holders with a grant `caller` may see, after
/// `after`, at most `page_size` of them.
pub(crate) fn who_page(
    engine: &Engine,
    judged: &Judged<'_>,
    caller: IdentityId,
    (resource, action): (&Resource, &Action),
    after: Option<&str>,
    page_size: usize,
) -> Result<WhoPage, ServerError> {
    let reached = engine.catch_up(judged.grants, judged.root)?;
    let resolved = who(
        engine.evaluator(),
        &reached,
        &mut QuestionSet::new(),
        resource,
        action,
    )?;
    let book = judged.grants.book();
    let mut known = HashMap::new();
    let holders: BTreeSet<(String, IdentityId)> = resolved
        .into_iter()
        .filter(|holder| {
            book.held_by(*holder).any(|record| {
                record.grant().resource() == resource
                    && record.grant().actions().contains(action)
                    && sees_with(judged, caller, record, &mut known)
            })
        })
        .map(|holder| (holder.to_string(), holder))
        .filter(|(text, _)| after.is_none_or(|after| text.as_str() > after))
        .collect();
    let mut permitted = holders.into_iter().filter_map(|(text, holder)| {
        let question = Question {
            subject: holder,
            resource,
            action,
        };
        let (grant, lineage) = resting(book, &question)?;
        let permit = Permit {
            grant: grant.id(),
            path: lineage.path,
            root_person: lineage.root_person,
            actions: grant.actions().clone(),
            model_version: grant.parts().model_version,
            revision: reached.position,
            use_event: None,
        };
        Some(HolderView {
            holder: text,
            permit: PermitView::from(&permit),
        })
    });
    let page: Vec<HolderView> = permitted.by_ref().take(page_size).collect();
    let more = page.len() == page_size && permitted.next().is_some();
    let next = if more {
        page.last().map(|last| last.holder.clone())
    } else {
        None
    };
    Ok(WhoPage {
        holders: page,
        revision: reached.position,
        complete: next.is_none(),
        next,
    })
}
