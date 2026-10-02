//! Refusals name eligible authorities along the agent's existing chains.

use std::collections::{BTreeMap, BTreeSet};

use axum::Json;
use axum::response::{IntoResponse, Response};

use lys_identity::grants::{
    Action, ExerciseRequest, GrantError, RecipientKind, Resource, Route, admission::effective,
};
use lys_identity::{AgentId, IdentityError, IdentityId};
use serde::Serialize;

use crate::error::ServerError;
use crate::grants::Judged;
use crate::routes::AppState;

/// An identity the agent can ask for the refused authority.
#[derive(Debug, Serialize)]
pub struct CanGrant {
    /// The directory identity.
    pub id: String,
    /// Its directory kind.
    pub kind: &'static str,
    /// Its current display name.
    pub display_name: String,
}

/// Read eligible chain holders without recording a request or exercising a grant.
///
/// # Errors
/// Returns the named directory, grant or permission-engine refusal when the
/// current authority cannot be read.
pub fn for_agent(
    state: &AppState,
    agent: AgentId,
    resource: &Resource,
    action: &Action,
) -> Result<Vec<CanGrant>, ServerError> {
    crate::grants::with_grants(state, |mut judged| {
        for_judged(&mut judged, agent, resource, action, crate::session::now())
    })
}

fn named(judged: &Judged<'_>, identity: IdentityId) -> Result<CanGrant, ServerError> {
    let record =
        judged
            .directory
            .record(identity)
            .ok_or_else(|| IdentityError::IdentityUnknown {
                identity: identity.to_string(),
            })?;
    Ok(CanGrant {
        id: identity.to_string(),
        kind: match identity {
            IdentityId::Person(_) => "person",
            IdentityId::Agent(_) => "agent",
            IdentityId::ServiceAccount(_) => "service_account",
        },
        display_name: record.profile().display_name().to_owned(),
    })
}

pub(crate) fn for_judged(
    judged: &mut Judged<'_>,
    agent: AgentId,
    resource: &Resource,
    action: &Action,
    at: u64,
) -> Result<Vec<CanGrant>, ServerError> {
    let caller = IdentityId::Agent(agent);
    let root = IdentityId::Person(judged.root);
    let mut chain = Vec::new();
    let mut seen = BTreeSet::from([caller]);
    let mut cursor = caller;
    while cursor != root {
        let record =
            judged
                .directory
                .record(cursor)
                .ok_or_else(|| IdentityError::IdentityUnknown {
                    identity: cursor.to_string(),
                })?;
        let Some(parent) = record.reports_to() else {
            if matches!(cursor, IdentityId::Person(_)) {
                break;
            }
            return Err(IdentityError::NoAccountablePerson {
                chain: seen.iter().map(ToString::to_string).collect(),
            }
            .into());
        };
        if !seen.insert(parent) {
            return Err(IdentityError::AnswersToCycle {
                chain: chain
                    .iter()
                    .map(ToString::to_string)
                    .chain([parent.to_string()])
                    .collect(),
            }
            .into());
        }
        chain.push(parent);
        cursor = parent;
    }

    let book = judged.grants.book();
    let candidates: Vec<_> = book
        .on_resource(resource)
        .map(lys_identity::grants::projection::GrantRecord::grant)
        .collect();
    for grant in &candidates {
        if grant.holder() == caller {
            for ancestor in book.lineage(grant.id())?.path.into_iter().skip(1) {
                let holder = book
                    .grant(ancestor)
                    .ok_or_else(|| lys_identity::grants::GrantError::GrantUnknown {
                        grant: ancestor.to_string(),
                    })?
                    .holder();
                if seen.insert(holder) {
                    chain.push(holder);
                }
                if holder == root {
                    break;
                }
            }
        }
    }
    let mut possible = BTreeMap::<_, Vec<_>>::new();
    for grant in candidates {
        if grant.holder() == caller
            || grant.holder() == root
            || !seen.contains(&grant.holder())
            || !grant.actions().contains(action)
            || !grant.pass_on().permits(RecipientKind::Agent)
            || !grant
                .pass_on()
                .actions()
                .is_some_and(|actions| actions.contains(action))
            || matches!(grant.holder(), IdentityId::ServiceAccount(_))
        {
            continue;
        }
        match effective(book, judged.directory, grant.id(), at) {
            Ok(_) => {}
            Err(error) if crate::grants_batch::unanswered(&error) => return Err(error.into()),
            Err(_) => continue,
        }
        possible.entry(grant.holder()).or_default().push(grant.id());
    }
    let mut eligible = BTreeSet::new();
    if !possible.is_empty() {
        let frame = judged.grants.frame(judged.directory, None)?;
        for (holder, sources) in possible {
            let mut live = false;
            for source in sources {
                match effective(judged.grants.book(), judged.directory, source, at) {
                    Ok(_) => {
                        live = true;
                        break;
                    }
                    Err(error) if crate::grants_batch::unanswered(&error) => {
                        return Err(error.into());
                    }
                    Err(_) => {}
                }
            }
            if !live {
                continue;
            }
            let request = ExerciseRequest {
                caller: holder,
                route: Route::Tool,
                resource: resource.clone(),
                action: action.clone(),
            };
            match judged.grants.explain_in(&frame, &request, at) {
                Ok(_) => {
                    eligible.insert(holder);
                }
                Err(error) if crate::grants_batch::unanswered(&error) => return Err(error.into()),
                Err(_) => {}
            }
        }
    }
    let mut answer = Vec::new();
    for holder in chain {
        if holder != caller && holder != root && eligible.contains(&holder) {
            answer.push(named(judged, holder)?);
        }
    }
    answer.push(named(judged, root)?);
    Ok(answer)
}

/// An authority refusal that may name eligible grantors.
pub enum AuthorityRefusal<'a> {
    /// The grants authority refused the call.
    Grant(GrantError),
    /// The route's responsibility remains with a person.
    ResponsibilityKept {
        /// The refused route.
        route: &'a str,
    },
}

impl From<GrantError> for AuthorityRefusal<'_> {
    fn from(error: GrantError) -> Self {
        Self::Grant(error)
    }
}

/// Add grantor names while retaining the refusal's ordinary HTTP answer.
#[must_use]
pub fn refusal<'a>(error: impl Into<AuthorityRefusal<'a>>, holders: &[CanGrant]) -> Response {
    let error = match error.into() {
        AuthorityRefusal::Grant(error) => error,
        AuthorityRefusal::ResponsibilityKept { route } => {
            return (
                axum::http::StatusCode::FORBIDDEN,
                Json(serde_json::json!({
                    "refusal":"ResponsibilityKept", "reason":"a person keeps this responsibility",
                    "fields":{"route":route}, "can_grant":[],
                })),
            )
                .into_response();
        }
    };
    let error = ServerError::from(error);
    (
        error.status(),
        Json(serde_json::json!({
            "refusal":error.name(), "reason":error.to_string(), "fields":error.fields(),
            "can_grant":holders,
        })),
    )
        .into_response()
}
