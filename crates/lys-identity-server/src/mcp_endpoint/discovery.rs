//! Discover authority without exercising an act or requiring a target route's scope.

use std::collections::{BTreeMap, BTreeSet};

use axum::Json;
use axum::http::HeaderMap;
use axum::response::IntoResponse;
use lys_identity::grants::{Action, ExerciseRequest, Resource, Route};
use lys_identity::{AgentId, IdentityId, LifecycleState};
use serde_json::{Value, json};

use super::{Endpoint, ResultValue};
use crate::error::ServerError;
use crate::grants::{Decision, Judged, allowed, decide};
use crate::routes::AppState;

pub(super) async fn answer(
    endpoint: &Endpoint,
    headers: &HeaderMap,
    signer: Option<AgentId>,
    app: Option<&crate::mcp_oauth::Connection>,
) -> ResultValue {
    let state = endpoint.state.as_ref().ok_or_else(|| {
        (
            -32603,
            "the MCP connection has no authority state for discovery".to_owned(),
        )
    })?;
    let token = if headers.contains_key(crate::grant_tokens::HEADER) {
        let principal = crate::grant_tokens::header(headers)
            .and_then(|token| crate::grant_tokens::discovery_principal(state, token));
        match principal {
            Ok(principal) => Some(principal),
            Err(error) => return crate::mcp_callers::rendered(error.into_response()).await,
        }
    } else {
        None
    };
    let started = std::time::Instant::now();
    let reading = async {
        let holder = token
            .map(|principal| principal.holder)
            .or(signer)
            .or_else(|| app.map(|app| app.agent))
            .or(crate::agent_pass::holder(state, headers)?);
        let identity = |judged: &Judged<'_>| -> Result<(IdentityId, bool), ServerError> {
            let (holder, administrator) = if let Some(agent) = holder {
                (IdentityId::Agent(agent), false)
            } else {
                let actor = crate::routes::signed_in(state, headers)?;
                (
                    IdentityId::Person(crate::read_api::own_person(judged.directory, &actor)?),
                    state.admission.is_administrator(judged.directory, &actor)?,
                )
            };
            let refused = || ServerError::NotAdmitted {
                reason: "the connection's holder is not an active directory identity",
            };
            if judged
                .directory
                .record(holder)
                .ok_or_else(&refused)?
                .state()
                != LifecycleState::Active
            {
                return Err(refused());
            }
            Ok((holder, administrator))
        };
        let only = token.map(|principal| principal.grant);
        crate::grants_reads::with_pass_grants(
            state,
            crate::session::now(),
            None,
            true,
            |judged| {
                let (holder, _) = identity(judged)?;
                questions(state, judged, holder, only)
            },
            |mut judged| {
                let (holder, administrator) = identity(&judged)?;
                let placements = judged.apps.held().placements.len();
                inventory(state, &mut judged, holder, only, administrator)
                    .map(|answer| (answer, placements))
            },
        )
    };
    let read = match token {
        Some(principal) => crate::agent_signature::token_scoped(principal, reading).await,
        None => reading.await,
    };
    let response = match read {
        Ok((inventory, placements)) => {
            let answer_bytes = serde_json::to_vec(&inventory)
                .map_err(|error| {
                    (
                        -32603,
                        format!("the discovery answer cannot be encoded: {error}"),
                    )
                })?
                .len();
            tracing::info!(target: "lys_identity_server::passfast", placements, elapsed_ms = started.elapsed().as_millis(), answer_bytes, "MCP discovery");
            Json(inventory).into_response()
        }
        Err(error) => error.into_response(),
    };
    crate::mcp_callers::rendered(response).await
}

fn questions(
    state: &AppState,
    judged: &Judged<'_>,
    holder: IdentityId,
    only: Option<lys_identity::grants::GrantId>,
) -> Result<Vec<ExerciseRequest>, ServerError> {
    let mut candidates: BTreeSet<Resource> = judged
        .grants
        .book()
        .held_by(holder)
        .filter(|record| only.is_none_or(|id| record.grant().id() == id))
        .map(|record| record.grant().resource().clone())
        .collect();
    if !candidates.is_empty() && only.is_none() {
        for placed in &judged.apps.held().placements {
            candidates.insert(Resource::new(&placed.child_kind, &placed.child_id)?);
        }
    }
    let model = state.grant_setup.model()?;
    let mut requests = Vec::new();
    for resource in candidates {
        let actions: BTreeSet<Action> = match model.kinds().get(resource.kind()) {
            Some(kind) => kind.actions.clone(),
            None => model
                .relations_on(resource.kind())
                .flat_map(|(_, actions)| actions.iter().cloned())
                .collect(),
        };
        for action in actions {
            requests.push(ExerciseRequest {
                caller: holder,
                route: Route::Api,
                resource: resource.clone(),
                action,
            });
        }
    }
    Ok(requests)
}

fn inventory(
    state: &AppState,
    judged: &mut Judged<'_>,
    holder: IdentityId,
    only: Option<lys_identity::grants::GrantId>,
    administrator: bool,
) -> Result<Value, ServerError> {
    let at = crate::session::now();
    let mut resources: BTreeMap<Resource, BTreeSet<Action>> = BTreeMap::new();
    for request in questions(state, judged, holder, only)? {
        if allowed(decide(judged, &request, at, None, Decision::Explain))? {
            resources
                .entry(request.resource)
                .or_default()
                .insert(request.action);
        }
    }
    let mut routes = BTreeMap::new();
    // An agent is shown no route a person keeps from every agent (AGENTS-019
    // R4); the administrator's routes are hidden from everyone who is not
    // one, since their handlers still ask for the administrator.
    let agent = matches!(holder, IdentityId::Agent(_));
    for row in crate::openapi_table::TABLE {
        let admin_only = row.4.iter().any(|set| {
            *set == crate::openapi_refusals::ADMIN || *set == crate::openapi_refusals::ADMIN_BODY
        }) && !row.4.contains(&crate::openapi_refusals::GRANT_ASKED)
            && !crate::openapi_table::judged_by_handler(row);
        if admin_only && !administrator {
            continue;
        }
        if agent && row.5.is_some() && crate::openapi_table::listed_human_only(row) {
            continue;
        }
        if row.5.is_none() {
            routes.insert((row.0.word(), row.1, String::new(), String::new()), json!({
                "method": row.0.word().to_ascii_uppercase(), "path": row.1,
                "rule": { "description": row.2, "authentication": row.3.iter().map(|auth| format!("{auth:?}")).collect::<Vec<_>>(), "refusals": row.4 },
            }));
            continue;
        }
        for (resource, actions) in &resources {
            for path in paths(row.1, resource.id())? {
                if state.kept_responsibilities.keeps(row.0.word(), &path) {
                    continue;
                }
                let (scoped, action) = match crate::openapi_table::token_scope(row.0.word(), &path)
                {
                    Ok(scope) => scope,
                    Err(crate::grant_tokens::TokenError::Undeclared) => continue,
                    Err(error) => {
                        return Err(ServerError::ConfigInvalid {
                            reason: format!(
                                "the declared MCP route's scope could not be read: {error}"
                            ),
                        });
                    }
                };
                // A route on the organisation names none in its path: it acts
                // on the caller's own, the one its grant is on (POST /people/admit).
                let scoped = if scoped.kind() == "organisation"
                    && scoped.id() == "all"
                    && resource.kind() == "organisation"
                {
                    resource.clone()
                } else {
                    scoped
                };
                if &scoped != resource || !actions.contains(&action) {
                    continue;
                }
                routes.insert((row.0.word(), row.1, resource.kind().to_owned(), resource.id().to_owned()),
                    json!({"method": row.0.word().to_ascii_uppercase(), "path": row.1, "resource": {"kind": resource.kind(), "id": resource.id()}, "action": action.as_str()}));
            }
        }
    }
    let message = if resources.is_empty() {
        "This connection has no resource grants. The listed routes apply their own authentication, role and input rules; listing them grants no authority."
    } else {
        "This connection may use the listed resources and actions."
    };
    Ok(
        json!({"message": message, "routes": routes.into_values().collect::<Vec<_>>(),
        "resources": resources.into_iter().map(|(resource, actions)| json!({
            "kind": resource.kind(), "id": resource.id(), "actions": actions.iter().map(Action::as_str).collect::<Vec<_>>()
        })).collect::<Vec<_>>() }),
    )
}

fn paths(template: &str, id: &str) -> Result<BTreeSet<String>, ServerError> {
    let segments: Vec<_> = template.split('/').collect();
    let parameters = segments
        .iter()
        .filter(|part| part.starts_with('{') && part.ends_with('}'))
        .count();
    let replace = |values: &[&str]| -> Result<String, ServerError> {
        let mut values = values.iter();
        let words = segments
            .iter()
            .map(|part| {
                if part.starts_with('{') && part.ends_with('}') {
                    values
                        .next()
                        .map(|value| crate::provider::encoded(value))
                        .ok_or_else(|| ServerError::ConfigInvalid {
                            reason: "the declared MCP route has more parameters than its resource"
                                .to_owned(),
                        })
                } else {
                    Ok((*part).to_owned())
                }
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(words.join("/"))
    };
    let mut paths = BTreeSet::from([replace(&vec![id; parameters])?]);
    let components: Vec<_> = id.split('.').collect();
    if parameters > 1 && parameters == components.len() {
        paths.insert(replace(&components)?);
    }
    Ok(paths)
}
