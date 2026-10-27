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
//! of the relation asked, and the people who may give it with no source, by
//! the grants' own rules for that gift: a root issued to a person who asks,
//! or the administrator's standing given to an agent who asks, as Give access
//! gives it. An agent asking for a relation no agent may hold is refused when
//! it asks, in Give access's words, and nothing is kept. The root authority
//! decides any request it is shown. A request is shown to the one who asks,
//! to the person who answers for them, to those people and to the root
//! authority, and to anyone else it is refused exactly as a request that is
//! not kept.
//!
//! A request asked again with the operation id it was kept under is answered
//! as it was kept, whatever the time is by then: the end it asks for is
//! checked against now only when the request is new.
//!
//! The two decisions are in `requests_decide`.

use std::collections::BTreeSet;
use std::ops::Bound;
use std::str::FromStr;
use std::sync::Arc;

use axum::extract::State;
use axum::extract::rejection::JsonRejection;
use axum::http::HeaderMap;
use axum::routing::{get, post};
use axum::{Json, Router};
use lys_identity::grants::admission::refuse_withheld;
use lys_identity::grants::{Action, GrantError, GrantId, RecipientKind, Relation, Resource};
use lys_identity::{IdentityId, OperationId, PersonId};
use serde::Deserialize;

use crate::error::ServerError;
use crate::grant_sight::is_root;
use crate::grants::{Judged, with_grants};
use crate::read_api::{person_record, person_summary};
use crate::requests_store::{Asked, Decided, RequestStore};
use crate::requests_views::{DecisionView, RequestList, RequestView};
use crate::reviews_api::stands;
use crate::routes::{AppState, identity_id};

/// The access request routes.
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/requests", get(list).post(ask))
        .route(
            "/requests/{id}/approve",
            post(crate::requests_decide::approve),
        )
        .route(
            "/requests/{id}/decline",
            post(crate::requests_decide::decline),
        )
        .route(
            "/requests/{id}/reconcile",
            post(crate::requests_decide::settle),
        )
}

pub(crate) fn malformed(reason: impl Into<String>) -> ServerError {
    ServerError::RequestMalformed {
        reason: reason.into(),
    }
}

/// The body a route takes, or `RequestMalformed` in the service's own form.
pub(crate) fn taken<T>(body: Result<Json<T>, JsonRejection>) -> Result<T, ServerError> {
    body.map(|Json(body)| body)
        .map_err(|refused| malformed(refused.body_text()))
}

pub(crate) fn words(name: &str, text: &str) -> Result<String, ServerError> {
    let text = text.trim();
    if text.is_empty() {
        return Err(malformed(format!("{name} is empty")));
    }
    Ok(text.to_owned())
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = RequestResourceBody)]
pub(crate) struct ResourceBody {
    kind: String,
    id: String,
}

/// A request for access. Every member is required; `ends_at` may be null.
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = RequestAskBody)]
pub(crate) struct AskBody {
    operation: String,
    resource: ResourceBody,
    relation: String,
    ends_at: Option<u64>,
    why: String,
}

pub(crate) fn with_requests<T>(
    state: &AppState,
    act: impl FnOnce(&mut RequestStore) -> Result<T, ServerError>,
) -> Result<T, ServerError> {
    let store = state
        .requests
        .as_ref()
        .ok_or_else(|| ServerError::RequestsUnavailable {
            reason: "the configuration names no requests_dir".to_owned(),
        })?;
    let mut store = store
        .lock()
        .map_err(|error| ServerError::RequestsUnavailable {
            reason: format!("the requests lock is poisoned: {error}"),
        })?;
    store.settle()?;
    act(&mut store)
}

/// Whether the model as it stands now defines the relation a request asks
/// for on its kind (ACCESS-011 R1). A request is asked against the model of
/// its day; an app's schema replaced since, or Lys's own model raised, can
/// leave the relation undefined, and the request is then shown as asked,
/// not approved, and declinable, and never refuses the queue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Standing {
    /// The relation resolves to actions under the model as it stands.
    Grantable,
    /// The model (the kind's version named) defines no such relation.
    RelationUnknown { model_version: u64 },
}

impl Standing {
    /// The word on the wire.
    pub(crate) fn word(self) -> &'static str {
        match self {
            Self::Grantable => "grantable",
            Self::RelationUnknown { .. } => "relation_unknown",
        }
    }
}

/// The requests whose outlived relation has been named in the log since
/// start, so each is named once, not on every read of the queue.
static NAMED_OUTLIVED: std::sync::OnceLock<std::sync::Mutex<BTreeSet<String>>> =
    std::sync::OnceLock::new();

fn name_outlived_once(asked: &Asked, model_version: u64) {
    let named = NAMED_OUTLIVED.get_or_init(|| std::sync::Mutex::new(BTreeSet::default()));
    // A poisoned set names the request again rather than never.
    let first = named
        .lock()
        .map_or(true, |mut held| held.insert(asked.id.clone()));
    if first {
        tracing::warn!(
            request = %asked.id,
            kind = %asked.resource_kind,
            relation = %asked.relation,
            model_version,
            "an access request asks for a relation the model as it stands no longer defines on its kind; it is listed as not grantable and can be declined"
        );
    }
}

/// A request read against the directory and the grants as they stand.
pub(crate) struct Weighed {
    pub(crate) seeker: IdentityId,
    pub(crate) responsible: PersonId,
    pub(crate) resource: Resource,
    pub(crate) relation: Relation,
    /// The actions the relation resolves to now; none when it is unknown.
    actions: BTreeSet<Action>,
    pub(crate) standing: Standing,
    approvers: Vec<PersonId>,
    /// The people who may give the access with no source: a root to a
    /// person (ACCESS-009 R2), the administrator's standing to an agent.
    rooters: Vec<PersonId>,
    lenders: Vec<(PersonId, GrantId)>,
}

impl Weighed {
    fn of(judged: &Judged<'_>, asked: &Asked, at: u64) -> Result<Self, ServerError> {
        let seeker = identity_id(&asked.asked_by)?;
        let resource = Resource::new(&asked.resource_kind, &asked.resource_id)?;
        let relation = Relation::new(&asked.relation)?;
        let (actions, standing) = match judged.grants.model().actions_on(resource.kind(), &relation)
        {
            Ok(needed) => (needed.clone(), Standing::Grantable),
            Err(GrantError::RelationUnknown { model_version, .. }) => {
                name_outlived_once(asked, model_version);
                (
                    BTreeSet::default(),
                    Standing::RelationUnknown { model_version },
                )
            }
            Err(other) => return Err(other.into()),
        };
        let kind = RecipientKind::of(seeker);
        let book = judged.grants.book();
        let mut lenders = Vec::new();
        let rooters = issuers(judged, seeker, (&resource, &relation, &actions), at)?;
        let mut approvers = rooters.clone();
        // Nothing lends a relation the model no longer defines: an empty set
        // of needed actions would be within every pass-on.
        if standing == Standing::Grantable {
            for record in book.on_resource(&resource) {
                let grant = record.grant();
                let lends = grant.pass_on().permits(kind)
                    && grant
                        .pass_on()
                        .actions()
                        .is_some_and(|held| actions.is_subset(held));
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
        }
        Ok(Self {
            seeker,
            responsible: PersonId::from_str(&asked.responsible)?,
            resource,
            relation,
            actions,
            standing,
            approvers,
            rooters,
            lenders,
        })
    }

    /// The refusal of an approval of a request whose relation the model no
    /// longer defines (ACCESS-011 R2), or nothing to refuse.
    pub(crate) fn approvable(&self) -> Result<(), ServerError> {
        match self.standing {
            Standing::Grantable => Ok(()),
            Standing::RelationUnknown { model_version } => Err(GrantError::RelationUnknown {
                relation: format!("{} on {}", self.relation, self.resource.kind()),
                model_version,
            }
            .into()),
        }
    }

    /// The refusal of an agent asking for what no agent may be given, in the
    /// words Give access refuses it with, or nothing to refuse.
    fn givable(&self, judged: &Judged<'_>) -> Result<(), ServerError> {
        Ok(withheld(
            judged,
            self.seeker,
            (&self.resource, &self.relation, &self.actions),
        )?)
    }

    /// Whether `caller` may decide the request: a person who could give the
    /// access, or the root authority, which decides any request it is shown.
    fn decided_by(&self, caller: IdentityId, root: PersonId) -> bool {
        matches!(caller, IdentityId::Person(_)) && (self.decides(caller) || is_root(caller, root))
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

    pub(crate) fn view(
        &self,
        judged: &Judged<'_>,
        caller: IdentityId,
        asked: &Asked,
        decided: Option<&Decided>,
        held_by: Option<&str>,
    ) -> Result<RequestView, ServerError> {
        let summary = |person: PersonId| {
            person_record(judged.directory, person).map(|record| person_summary(person, record))
        };
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
            actions: self
                .actions
                .iter()
                .map(|action| action.as_str().to_owned())
                .collect(),
            standing: self.standing.word(),
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
            can_issue_root: self
                .rooters
                .iter()
                .any(|person| IdentityId::Person(*person) == caller),
            can_decide: self.decided_by(caller, judged.root),
            held_by: held_by.map(str::to_owned),
            decision: decided.map(DecisionView::from),
        })
    }
}

/// What a request asks for: the resource, the relation and the actions the
/// relation resolves to.
type Asks<'a> = (&'a Resource, &'a Relation, &'a BTreeSet<Action>);

/// The refusal of `seeker`, when it is an agent, being given what `asks`
/// carries that no agent may hold: the rule, and the words, every gift to an
/// agent is judged by.
fn withheld(judged: &Judged<'_>, seeker: IdentityId, asks: Asks<'_>) -> Result<(), GrantError> {
    let (resource, relation, actions) = asks;
    if RecipientKind::of(seeker) == RecipientKind::Agent {
        refuse_withheld(judged.grants.model(), relation, resource.kind(), actions)
    } else {
        Ok(())
    }
}

/// The people who may give `seeker` what `asks` carries with no source, by
/// the grants' own rules for that gift: whoever may issue a person the root
/// (ACCESS-009 R2); for an agent, whoever may give it from the
/// administrator's standing, as Give access gives it, and nobody while it
/// carries what no agent may hold. Anyone else is lent access from a grant a
/// person holds.
fn issuers(
    judged: &Judged<'_>,
    seeker: IdentityId,
    asks: Asks<'_>,
    at: u64,
) -> Result<Vec<PersonId>, ServerError> {
    Ok(match seeker {
        IdentityId::Person(_) => {
            let organisation = judged.directory.organisation_of(seeker);
            judged.grants.rooters(organisation, asks.0, at)?
        }
        IdentityId::Agent(_) if withheld(judged, seeker, asks).is_ok() => {
            judged.grants.standing_givers(judged.directory, seeker)
        }
        IdentityId::Agent(_)
        | IdentityId::ServiceAccount(_)
        | IdentityId::Connector(_)
        | IdentityId::Machine(_)
        | IdentityId::Organisation(_) => Vec::new(),
    })
}

/// The request `id` as `caller` may see it, refused as unknown when it is not kept or not theirs to see.
pub(crate) fn seen(
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
pub(crate) fn decider(
    judged: &Judged<'_>,
    caller: IdentityId,
    weighed: &Weighed,
) -> Result<PersonId, ServerError> {
    match caller {
        IdentityId::Person(person) if weighed.decided_by(caller, judged.root) => Ok(person),
        _ => Err(ServerError::NotAdmitted {
            reason: "only a person who could give this access, or the root authority, decides the request",
        }),
    }
}

/// The answer for a request already decided: the same decision again is
/// answered as it was kept, and any other is refused.
pub(crate) fn settled(decided: &Decided, by: PersonId, approved: bool) -> Result<(), ServerError> {
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
    query: crate::list_page::Input,
) -> Result<Json<RequestList>, ServerError> {
    listed(&state, &headers, query).map(Json)
}

/// What `GET /requests` answers the caller for `query`.
pub(crate) fn listed(
    state: &AppState,
    headers: &HeaderMap,
    query: crate::list_page::Input,
) -> Result<RequestList, ServerError> {
    with_grants(state, |judged| {
        let caller = crate::service_account_grants::caller(state, headers, &judged)?;
        let page = crate::list_page::Page::read(query, "/requests")?;
        let members = page
            .as_ref()
            .map(|page| page.members(state))
            .transpose()?
            .flatten();
        let at = state.sessions.now()?;
        with_requests(state, |store| {
            let (requests, totals) = if let Some(page) = &page {
                let filtered = page.filtered() || !is_root(caller, judged.root);
                let after = if filtered {
                    Bound::Unbounded
                } else {
                    page.after()
                };
                let (requests, totals) = page.select(
                    store.requests_ordered(after),
                    (!filtered).then_some(store.request_count()),
                    |asked| {
                        if !filtered {
                            return Ok(true);
                        }
                        let weighed = Weighed::of(&judged, asked, at)?;
                        let name = judged
                            .directory
                            .record(weighed.seeker)
                            .map_or("", |record| record.profile().display_name());
                        Ok((weighed.shows(caller) || is_root(caller, judged.root))
                            && crate::list_page::member(
                                members.as_ref(),
                                &asked.asked_by,
                                Some(&asked.responsible),
                            )
                            && page.matches([
                                name,
                                asked.relation.as_str(),
                                asked.resource_id.as_str(),
                                asked.why.as_str(),
                            ]))
                    },
                    |asked| &asked.id,
                    |asked| {
                        let weighed = Weighed::of(&judged, asked, at)?;
                        let (_, decided) = store
                            .request(&asked.id)
                            .ok_or(ServerError::RequestUnknown)?;
                        let held_by = store.intent(&asked.id).map(|intent| intent.by.as_str());
                        weighed.view(&judged, caller, asked, decided, held_by)
                    },
                )?;
                (requests, Some(totals))
            } else {
                let mut requests = Vec::new();
                for (asked, decided) in store.requests() {
                    let weighed = Weighed::of(&judged, asked, at)?;
                    if weighed.shows(caller) || is_root(caller, judged.root) {
                        let held_by = store.intent(&asked.id).map(|intent| intent.by.as_str());
                        requests.push(weighed.view(&judged, caller, asked, decided, held_by)?);
                    }
                }
                (requests, None)
            };
            Ok(RequestList {
                requests,
                page: totals,
            })
        })
    })
}

async fn ask(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Result<Json<AskBody>, JsonRejection>,
) -> Result<Json<RequestView>, ServerError> {
    with_grants(&state, |judged| {
        let caller = crate::service_account_grants::caller(&state, &headers, &judged)?;
        let body = taken(body)?;
        let at = state.sessions.now()?;
        let responsible = match caller {
            IdentityId::Person(person) => Some(person),
            IdentityId::Agent(_)
            | IdentityId::ServiceAccount(_)
            | IdentityId::Connector(_)
            | IdentityId::Machine(_)
            | IdentityId::Organisation(_) => judged
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
        // A relation the model does not define is refused when asked for;
        // only a request the model later outlives is kept and listed as
        // not grantable (ACCESS-011).
        weighed.approvable()?;
        // An agent asking for what no agent may be given is refused now, by
        // name and in Give access's words, and nothing is kept.
        weighed.givable(&judged)?;
        with_requests(&state, |store| {
            let fresh = store.request(&asked.id).is_none();
            if fresh && asked.ends_at.is_some_and(|ends_at| ends_at <= at) {
                return Err(malformed("the end asked for is not after now"));
            }
            store.ask(asked.clone())?;
            let (kept, decided) = store
                .request(&asked.id)
                .ok_or(ServerError::RequestUnknown)?;
            let held_by = store.intent(&kept.id).map(|intent| intent.by.as_str());
            Ok(Json(weighed.view(&judged, caller, kept, decided, held_by)?))
        })
    })
}
