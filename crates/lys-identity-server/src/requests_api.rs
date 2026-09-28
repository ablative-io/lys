//! The access request routes: ask for access, see what is asked, and approve
//! or decline.
//!
//! Anyone in the directory, person or agent, asks for a relation on a
//! resource, for themselves, until an end they name, and says why. A request
//! gives no access. Approving one issues a grant through the grants' one
//! authority owner, exactly as `POST /grants` or `POST /grants/roots` would,
//! so a request can be approved only by a caller the grants would let give
//! that access, and the grant ends when the request said.
//!
//! The people who could approve a request are the people holding a standing
//! grant on the resource that may be passed on to the one who asks for every action
//! of the relation asked, and the root authority when a person asks. A
//! request is shown to the one who asks, to the person who answers for them, to
//! those people and to the root authority, and to anyone else it is refused
//! exactly as a request that is not kept.
//!
//! A request asked again with the operation id it was kept under is answered
//! as it was kept, whatever the time is by then: the end it asks for is
//! checked against now only when the request is new.
//!
//! A decision is kept after the grant is committed. If the service stops
//! between the two, the request still waits, and approving it again with the
//! same operation id answers the same grant and keeps the decision.

use std::str::FromStr;
use std::sync::{Arc, PoisonError};

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::routing::{get, post};
use axum::{Json, Router};
use lys_identity::grants::{
    DelegateRequest, GrantId, PassOn, RecipientKind, Relation, Resource, RootRequest, Window,
};
use lys_identity::{IdentityId, OperationId, PersonId};
use serde::Deserialize;

use crate::error::ServerError;
use crate::grant_contract::{RouteWire, grant_id};
use crate::grant_sight::{as_seen_by, is_root};
use crate::grants::{Judged, caller, with_grants};
use crate::read_api::{person_record, person_summary};
use crate::requests_store::{Asked, Decided, RequestStore};
use crate::requests_views::{DecisionView, RequestList, RequestView};
use crate::reviews_api::stands;
use crate::routes::{AppState, identity_id};
use crate::session::now;

/// The most characters a request's reason or a decision's note carries.
const WORDS_MAX: usize = 500;

/// The access request routes.
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/requests", get(list).post(ask))
        .route("/requests/{id}/approve", post(approve))
        .route("/requests/{id}/decline", post(decline))
}

fn malformed(reason: impl Into<String>) -> ServerError {
    ServerError::RequestMalformed {
        reason: reason.into(),
    }
}

/// The body a route takes, or `RequestMalformed` in the service's own form.
fn taken<T>(body: Result<Json<T>, JsonRejection>) -> Result<T, ServerError> {
    body.map(|Json(body)| body)
        .map_err(|refused| malformed(refused.body_text()))
}

fn words(name: &str, text: &str) -> Result<String, ServerError> {
    let text = text.trim();
    if text.is_empty() {
        return Err(malformed(format!("{name} is empty")));
    }
    if text.chars().count() > WORDS_MAX {
        return Err(malformed(format!(
            "{name} is longer than {WORDS_MAX} characters"
        )));
    }
    Ok(text.to_owned())
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ResourceBody {
    kind: String,
    id: String,
}

/// A request for access. Every member is required; `ends_at` may be null.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AskBody {
    operation: String,
    resource: ResourceBody,
    relation: String,
    ends_at: Option<u64>,
    why: String,
}

/// An approval. `source` is the grant the access is lent from, or null for
/// the root authority to issue it to a person.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ApproveBody {
    operation: String,
    route: RouteWire,
    source: Option<String>,
    note: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DeclineBody {
    note: String,
}

fn with_requests<T>(
    state: &AppState,
    act: impl FnOnce(&mut RequestStore) -> Result<T, ServerError>,
) -> Result<T, ServerError> {
    let store = state
        .requests
        .as_ref()
        .ok_or_else(|| ServerError::RequestsUnavailable {
            reason: "the configuration names no requests_dir".to_owned(),
        })?;
    let mut store = store.lock().unwrap_or_else(PoisonError::into_inner);
    store.settle()?;
    act(&mut store)
}

/// A request read against the directory and the grants as they stand.
struct Weighed {
    seeker: IdentityId,
    responsible: PersonId,
    resource: Resource,
    relation: Relation,
    approvers: Vec<PersonId>,
    lenders: Vec<(PersonId, GrantId)>,
}

impl Weighed {
    fn of(judged: &Judged<'_>, asked: &Asked, at: u64) -> Result<Self, ServerError> {
        let seeker = identity_id(&asked.asked_by)?;
        let resource = Resource::new(&asked.resource_kind, &asked.resource_id)?;
        let relation = Relation::new(&asked.relation)?;
        let needed = judged.grants.model().actions(&relation)?;
        let kind = RecipientKind::of(seeker);
        let book = judged.grants.book();
        let mut approvers = Vec::new();
        let mut lenders = Vec::new();
        if kind == RecipientKind::Person {
            approvers.push(judged.root);
        }
        for record in book.on_resource(&resource) {
            let grant = record.grant();
            let lends = grant.pass_on().permits(kind)
                && grant
                    .pass_on()
                    .actions()
                    .is_some_and(|actions| needed.is_subset(actions));
            if let IdentityId::Person(holder) = grant.holder()
                && lends
                && stands(book, record, at)
            {
                lenders.push((holder, grant.id()));
                if !approvers.contains(&holder) {
                    approvers.push(holder);
                }
            }
        }
        Ok(Self {
            seeker,
            responsible: PersonId::from_str(&asked.responsible)?,
            resource,
            relation,
            approvers,
            lenders,
        })
    }

    fn decides(&self, caller: IdentityId) -> bool {
        self.approvers
            .iter()
            .any(|person| IdentityId::Person(*person) == caller)
    }

    fn shows(&self, caller: IdentityId) -> bool {
        caller == self.seeker
            || caller == IdentityId::Person(self.responsible)
            || self.decides(caller)
    }

    fn view(
        &self,
        judged: &Judged<'_>,
        caller: IdentityId,
        asked: &Asked,
        decided: Option<&Decided>,
    ) -> Result<RequestView, ServerError> {
        let summary = |person: PersonId| {
            person_record(judged.directory, person).map(|record| person_summary(person, record))
        };
        let actions = judged.grants.model().actions(&self.relation)?;
        Ok(RequestView {
            id: asked.id.clone(),
            asked_by: asked.asked_by.clone(),
            asked_by_name: judged
                .directory
                .record(self.seeker)
                .map(|record| record.profile().display_name().to_owned()),
            responsible: summary(self.responsible)?,
            resource: crate::grant_contract::ResourceView {
                kind: asked.resource_kind.clone(),
                id: asked.resource_id.clone(),
            },
            relation: asked.relation.clone(),
            actions: actions
                .iter()
                .map(|action| action.as_str().to_owned())
                .collect(),
            ends_at: asked.ends_at,
            why: asked.why.clone(),
            asked_at: asked.asked_at,
            state: match decided {
                None => "waiting",
                Some(decided) if decided.approved => "approved",
                Some(_) => "declined",
            },
            approvers: self
                .approvers
                .iter()
                .map(|person| summary(*person))
                .collect::<Result<_, _>>()?,
            sources: self
                .lenders
                .iter()
                .filter(|(holder, _)| IdentityId::Person(*holder) == caller)
                .map(|(_, grant)| grant.to_string())
                .collect(),
            decision: decided.map(DecisionView::from),
        })
    }
}

/// The request `id` as `caller` may see it, refused as unknown when it is not kept or not theirs to see.
fn seen(
    judged: &Judged<'_>,
    store: &RequestStore,
    caller: IdentityId,
    id: &str,
    at: u64,
) -> Result<(Asked, Option<Decided>, Weighed), ServerError> {
    let (asked, decided) = store.request(id).ok_or(ServerError::RequestUnknown)?;
    let weighed = Weighed::of(judged, asked, at)?;
    if weighed.shows(caller) || is_root(caller, judged.root) {
        Ok((asked.clone(), decided.cloned(), weighed))
    } else {
        Err(ServerError::RequestUnknown)
    }
}

/// The person deciding, or the refusal of a caller who may not decide this request.
fn decider(
    judged: &Judged<'_>,
    caller: IdentityId,
    weighed: &Weighed,
) -> Result<PersonId, ServerError> {
    match caller {
        IdentityId::Person(person) if weighed.decides(caller) || is_root(caller, judged.root) => {
            Ok(person)
        }
        _ => Err(ServerError::NotAdmitted {
            reason: "only a person who could give this access, or the root authority, decides the request",
        }),
    }
}

/// The answer for a request already decided: the same decision again is
/// answered as it was kept, and any other is refused.
fn settled(decided: &Decided, by: PersonId, approved: bool) -> Result<(), ServerError> {
    if decided.approved == approved && decided.by == by.to_string() {
        Ok(())
    } else {
        Err(ServerError::RequestDecided {
            request: decided.id.clone(),
        })
    }
}

async fn list(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<RequestList>, ServerError> {
    with_grants(&state, |judged| {
        let caller = caller(&state, &headers, judged.directory)?;
        let at = now();
        with_requests(&state, |store| {
            let mut requests = Vec::new();
            for (asked, decided) in store.requests() {
                let weighed = Weighed::of(&judged, asked, at)?;
                if weighed.shows(caller) || is_root(caller, judged.root) {
                    requests.push(weighed.view(&judged, caller, asked, decided)?);
                }
            }
            Ok(Json(RequestList { requests }))
        })
    })
}

async fn ask(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Result<Json<AskBody>, JsonRejection>,
) -> Result<Json<RequestView>, ServerError> {
    with_grants(&state, |judged| {
        let caller = caller(&state, &headers, judged.directory)?;
        let body = taken(body)?;
        let at = now();
        let responsible = match caller {
            IdentityId::Person(person) => Some(person),
            IdentityId::Agent(_) => judged
                .directory
                .record(caller)
                .and_then(lys_identity::projection::Record::responsible),
        }
        .ok_or(ServerError::NoPerson)?;
        let asked = Asked {
            id: OperationId::from_str(&body.operation)?.to_string(),
            asked_by: caller.to_string(),
            responsible: responsible.to_string(),
            resource_kind: body.resource.kind,
            resource_id: body.resource.id,
            relation: body.relation,
            ends_at: body.ends_at,
            why: words("why", &body.why)?,
            asked_at: at,
        };
        let weighed = Weighed::of(&judged, &asked, at)?;
        with_requests(&state, |store| {
            let fresh = store.request(&asked.id).is_none();
            if fresh && asked.ends_at.is_some_and(|ends_at| ends_at <= at) {
                return Err(malformed("the end asked for is not after now"));
            }
            store.ask(asked.clone())?;
            let (kept, decided) = store
                .request(&asked.id)
                .ok_or(ServerError::RequestUnknown)?;
            Ok(Json(weighed.view(&judged, caller, kept, decided)?))
        })
    })
}

async fn approve(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    body: Result<Json<ApproveBody>, JsonRejection>,
) -> Result<Json<RequestView>, ServerError> {
    with_grants(&state, |judged| {
        let caller = caller(&state, &headers, judged.directory)?;
        let body = taken(body)?;
        let at = now();
        with_requests(&state, |store| {
            let (asked, decided, weighed) = seen(&judged, store, caller, &id, at)?;
            let by = decider(&judged, caller, &weighed)?;
            if let Some(decided) = &decided {
                settled(decided, by, true)?;
                return Ok(Json(weighed.view(
                    &judged,
                    caller,
                    &asked,
                    Some(decided),
                )?));
            }
            let note = words("note", &body.note)?;
            let operation = OperationId::from_str(&body.operation)?;
            let window = Window::new(asked.asked_at, asked.ends_at)?;
            let recorded = match (&body.source, weighed.seeker) {
                (Some(source), _) => judged.grants.delegate(
                    judged.directory,
                    &DelegateRequest {
                        operation,
                        caller,
                        route: body.route.into(),
                        source: grant_id(source)?,
                        recipient: weighed.seeker,
                        responsible: weighed.responsible,
                        resource: weighed.resource.clone(),
                        relation: weighed.relation.clone(),
                        pass_on: PassOn::UseOnly,
                        window,
                    },
                    at,
                ),
                (None, IdentityId::Person(holder)) => judged.grants.issue_root(
                    judged.directory,
                    &RootRequest {
                        operation,
                        caller,
                        route: body.route.into(),
                        holder,
                        resource: weighed.resource.clone(),
                        relation: weighed.relation.clone(),
                        pass_on: PassOn::UseOnly,
                        window,
                    },
                    at,
                ),
                (None, IdentityId::Agent(_)) => {
                    return Err(malformed(
                        "an agent's access is lent from a grant a person holds: name the source",
                    ));
                }
            }
            .map_err(|error| as_seen_by(&judged, caller, error))?;
            let decided = Decided {
                id: asked.id.clone(),
                by: by.to_string(),
                approved: true,
                note,
                grant: Some(recorded.receipt.grant.to_string()),
                decided_at: at,
            };
            store.decide(decided.clone())?;
            Ok(Json(weighed.view(
                &judged,
                caller,
                &asked,
                Some(&decided),
            )?))
        })
    })
}

async fn decline(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    body: Result<Json<DeclineBody>, JsonRejection>,
) -> Result<Json<RequestView>, ServerError> {
    with_grants(&state, |judged| {
        let caller = caller(&state, &headers, judged.directory)?;
        let body = taken(body)?;
        let at = now();
        with_requests(&state, |store| {
            let (asked, decided, weighed) = seen(&judged, store, caller, &id, at)?;
            let by = decider(&judged, caller, &weighed)?;
            if let Some(decided) = &decided {
                settled(decided, by, false)?;
                return Ok(Json(weighed.view(
                    &judged,
                    caller,
                    &asked,
                    Some(decided),
                )?));
            }
            let decided = Decided {
                id: asked.id.clone(),
                by: by.to_string(),
                approved: false,
                note: words("note", &body.note)?,
                grant: None,
                decided_at: at,
            };
            store.decide(decided.clone())?;
            Ok(Json(weighed.view(
                &judged,
                caller,
                &asked,
                Some(&decided),
            )?))
        })
    })
}
