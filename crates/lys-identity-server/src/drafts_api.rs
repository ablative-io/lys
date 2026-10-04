//! Draft admission records immutable words and decisions without executing them.

use std::str::FromStr;
use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, Uri, header};
use axum::routing::{get, post};
use axum::{Json, Router};
use lys_identity::draft_event::{
    Approved, Correction, Created, DraftEvent, Refused, RequestEvidence, RequestSignature, Target,
};
use lys_identity::projection::draft::DraftRecord;
use lys_identity::{Actor, AuthMethod, Directory, IdentityId, OperationId, PersonId, Provenance};
use lys_log_store::FileLeafStore;
use serde::{Deserialize, Serialize};

use crate::error::ServerError;
use crate::routes::{AppState, hex, signed_in, with_directory};

/// The resource and action the prepared mutation names.
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct DraftTarget {
    kind: String,
    id: String,
    action: String,
}

/// The exact prepared request body is text, retained without reserialization.
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct DraftBody {
    operation: String,
    target: DraftTarget,
    method: String,
    path: String,
    body: String,
    note: String,
}

/// An approval reserves an application operation but performs no application.
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct DraftApproveBody {
    operation: String,
    creation_hash: String,
    application: String,
}

/// A refusal names the immutable creation and why it is refused.
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct DraftRefuseBody {
    operation: String,
    creation_hash: String,
    reason: String,
}

/// The responsible person's own replacement is saved beside the refusal.
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct DraftCorrectBody {
    operation: String,
    creation_hash: String,
    reason: String,
    replacement: DraftBody,
}

/// The log coordinate acknowledges a recorded decision, never an applied action.
#[derive(Serialize, utoipa::ToSchema)]
pub struct DraftAnswer {
    draft: String,
    operation: String,
    creation_hash: String,
    index: u64,
    tree_size: u64,
    leaf_hash: String,
    replacement: Option<String>,
}

/// The four draft mutations and the list of drafts a person may decide.
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/drafts", get(crate::drafts_list::list).post(create))
        .route("/drafts/{id}/approve", post(approve))
        .route("/drafts/{id}/refuse", post(refuse))
        .route("/drafts/{id}/correct", post(correct))
}

pub(crate) fn types(api: &mut lys_openapi::Api) -> Vec<crate::openapi_types::Entry> {
    let answer = api.schema::<DraftAnswer>();
    vec![
        (
            lys_openapi::Method::Get,
            "/drafts",
            Some(api.schema::<crate::drafts_list::DraftsQuery>()),
            Some(api.schema::<crate::drafts_list::DraftList>()),
        ),
        (
            lys_openapi::Method::Post,
            "/drafts",
            Some(api.schema::<DraftBody>()),
            Some(answer.clone()),
        ),
        (
            lys_openapi::Method::Post,
            "/drafts/{id}/approve",
            Some(api.schema::<DraftApproveBody>()),
            Some(answer.clone()),
        ),
        (
            lys_openapi::Method::Post,
            "/drafts/{id}/refuse",
            Some(api.schema::<DraftRefuseBody>()),
            Some(answer.clone()),
        ),
        (
            lys_openapi::Method::Post,
            "/drafts/{id}/correct",
            Some(api.schema::<DraftCorrectBody>()),
            Some(answer),
        ),
    ]
}

fn malformed(reason: impl Into<String>) -> ServerError {
    ServerError::RequestMalformed {
        reason: reason.into(),
    }
}

fn signature_boundary(headers: &HeaderMap) -> Result<(), ServerError> {
    if headers.contains_key(crate::agent_signature::HEADER)
        && (headers.contains_key(header::COOKIE)
            || headers
                .get_all(crate::agent_signature::HEADER)
                .iter()
                .count()
                != 1)
    {
        return Err(ServerError::AgentSignatureRefused {
            reason: "a signature must occur once and cannot accompany a cookie",
        });
    }
    Ok(())
}

fn unhex(text: &str) -> Result<Vec<u8>, ServerError> {
    if text.len() % 2 != 0 || !text.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(malformed("the supplied digest or signature is not hex"));
    }
    (0..text.len())
        .step_by(2)
        .map(|at| {
            u8::from_str_radix(&text[at..at + 2], 16)
                .map_err(|error| malformed(format!("the hex value does not read: {error}")))
        })
        .collect()
}

fn creation_hash(text: &str) -> Result<[u8; 32], ServerError> {
    if text.len() != 64 {
        return Err(malformed(
            "creation_hash must be a 32-byte SHA-256 digest in hex",
        ));
    }
    unhex(text)?.try_into().map_err(|bytes: Vec<u8>| {
        malformed(format!(
            "creation_hash has {} bytes, expected 32",
            bytes.len()
        ))
    })
}

fn matches_path(pattern: &str, path: &str) -> bool {
    let mut actual = path.split('/');
    for segment in pattern.split('/') {
        let Some(given) = actual.next() else {
            return false;
        };
        if segment.starts_with('{') && segment.ends_with('}') {
            if given.is_empty() {
                return false;
            }
        } else if segment != given {
            return false;
        }
    }
    actual.next().is_none()
}

fn creation(
    body: DraftBody,
    actor: Actor,
    reviewer: IdentityId,
    at: u64,
    corrects: Option<OperationId>,
) -> Result<Created, ServerError> {
    if !crate::openapi_table::TABLE.iter().any(|route| {
        let method = match route.0 {
            lys_openapi::Method::Get => "GET",
            lys_openapi::Method::Post => "POST",
            lys_openapi::Method::Put => "PUT",
        };
        method == body.method && method != "GET" && matches_path(route.1, &body.path)
    }) {
        return Err(malformed(
            "the prepared request must name a registered mutation route",
        ));
    }
    Ok(Created {
        operation: OperationId::from_str(&body.operation)?,
        actor,
        recorded_at: at,
        target: Target {
            kind: body.target.kind,
            id: body.target.id,
            action: body.target.action,
        },
        method: body.method,
        path: body.path,
        body: body.body.into_bytes(),
        note: body.note,
        reviewer,
        corrects,
        evidence: None,
        request_signature: None,
    })
}

async fn create(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    uri: Uri,
    bytes: Bytes,
) -> Result<Json<DraftAnswer>, ServerError> {
    signature_boundary(&headers)?;
    let body: DraftBody = crate::signed_json::read(&state, &headers, &bytes)?;
    let path = uri
        .path_and_query()
        .map_or(uri.path(), axum::http::uri::PathAndQuery::as_str);
    with_directory(&state, |directory| {
        let projection = directory.projection()?;
        let agent = crate::agent_signature::signed_agent(
            &state,
            projection,
            &headers,
            ("POST", path, &bytes),
        )?
        .ok_or(ServerError::AgentSignatureRefused {
            reason: "draft creation requires an agent signature",
        })?;
        let person = projection
            .record(IdentityId::Agent(agent))
            .and_then(lys_identity::projection::Record::responsible)
            .ok_or(ServerError::NoPerson)?;
        let binding = projection
            .record(IdentityId::Person(person))
            .and_then(|record| record.bindings().first())
            .ok_or(ServerError::NoPerson)?
            .clone();
        let at = crate::session::now();
        let actor = Actor::new(binding, Provenance::by_agent(agent, at));
        let mut created = creation(body, actor, IdentityId::Person(person), at, None)?;
        let (text, signed_path, signed_body) =
            if let Some(original) = headers.get(crate::agent_signature::HEADER) {
                let text = original.to_str().map_err(|error| {
                    malformed(format!("the signature header is not text: {error}"))
                })?;
                (text.to_owned(), path.to_owned(), bytes.to_vec())
            } else {
                let (header, message) = crate::agent_signature::relayed_signature().ok_or(
                    ServerError::AgentSignatureRefused {
                        reason: "the signature header is missing",
                    },
                )?;
                (
                    header,
                    crate::agent_signature::RELAY_PATH.to_owned(),
                    message.to_vec(),
                )
            };
        let words: Vec<_> = text.split_ascii_whitespace().collect();
        let [author, signed_at, nonce, signature] = words.as_slice() else {
            return Err(malformed(
                "the verified signature header does not contain four fields",
            ));
        };
        if *author != agent.to_string() {
            return Err(malformed(
                "the verified agent differs from the signature header",
            ));
        }
        let signed_at_ms = signed_at
            .parse()
            .map_err(|error| malformed(format!("the signing time does not read: {error}")))?;
        let payload = crate::agent_signature::payload(
            "POST",
            &signed_path,
            &signed_body,
            signed_at_ms,
            nonce,
        );
        created.evidence = Some(RequestEvidence {
            agent,
            method: "POST".to_owned(),
            path: signed_path,
            body: signed_body,
            signed_at_ms,
            nonce: (*nonce).to_owned(),
            cose_sign1: unhex(signature)?,
        });
        created.request_signature = Some(RequestSignature {
            header: text.as_bytes().to_vec(),
            payload,
        });
        let event = DraftEvent::Created(Arc::new(created));
        let draft = event.operation();
        let hash =
            lys_identity::encoding::payload_commitment(&lys_identity::draft_event::encode(&event));
        append(directory, event, draft, hash, None)
    })
}

/// The signed-in person's own session, the only caller who may decide a
/// draft or list the drafts they may decide.
pub(crate) fn personal(state: &AppState, headers: &HeaderMap) -> Result<Actor, ServerError> {
    signature_boundary(headers)?;
    if headers.contains_key(crate::agent_signature::HEADER) {
        return Err(ServerError::AgentSignatureRefused {
            reason: "only the responsible person's session may decide a draft",
        });
    }
    let actor = signed_in(state, headers)?;
    if actor.provenance().method() != AuthMethod::Oidc {
        return Err(ServerError::NotAdmitted {
            reason: "only the responsible person's session may decide a draft",
        });
    }
    Ok(actor)
}

/// Whether `person` may decide `held`: they are its responsible person.
pub(crate) fn decides(held: &DraftRecord, person: PersonId) -> bool {
    held.created.reviewer == IdentityId::Person(person)
}

fn responsible(
    directory: &mut Directory<FileLeafStore>,
    actor: &Actor,
    draft: OperationId,
) -> Result<IdentityId, ServerError> {
    let projection = directory.projection()?;
    let person = crate::read_api::own_person(projection, actor)?;
    let held =
        projection
            .draft(draft)
            .ok_or_else(|| lys_identity::IdentityError::DraftNotFound {
                draft: draft.to_string(),
            })?;
    if !decides(held, person) {
        return Err(ServerError::NotAdmitted {
            reason: "only the draft's responsible person may decide it",
        });
    }
    Ok(IdentityId::Person(person))
}

fn append(
    directory: &mut Directory<FileLeafStore>,
    event: DraftEvent,
    draft: OperationId,
    hash: [u8; 32],
    replacement: Option<OperationId>,
) -> Result<Json<DraftAnswer>, ServerError> {
    let operation = event.operation();
    let coordinate = directory.record_draft(event)?;
    Ok(Json(DraftAnswer {
        draft: draft.to_string(),
        operation: operation.to_string(),
        creation_hash: hex(&hash),
        index: coordinate.index,
        tree_size: coordinate.tree_size,
        leaf_hash: hex(&coordinate.leaf_hash),
        replacement: replacement.map(|id| id.to_string()),
    }))
}

async fn approve(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    bytes: Bytes,
) -> Result<Json<DraftAnswer>, ServerError> {
    let actor = personal(&state, &headers)?;
    let body: DraftApproveBody = crate::signed_json::read(&state, &headers, &bytes)?;
    let draft = OperationId::from_str(&id)?;
    let hash = creation_hash(&body.creation_hash)?;
    with_directory(&state, |directory| {
        responsible(directory, &actor, draft)?;
        let event = DraftEvent::Approved(Arc::new(Approved {
            operation: OperationId::from_str(&body.operation)?,
            actor,
            recorded_at: crate::session::now(),
            draft,
            draft_hash: hash,
            application: OperationId::from_str(&body.application)?,
            evidence: None,
        }));
        append(directory, event, draft, hash, None)
    })
}

async fn refuse(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    bytes: Bytes,
) -> Result<Json<DraftAnswer>, ServerError> {
    let actor = personal(&state, &headers)?;
    let body: DraftRefuseBody = crate::signed_json::read(&state, &headers, &bytes)?;
    let draft = OperationId::from_str(&id)?;
    let hash = creation_hash(&body.creation_hash)?;
    with_directory(&state, |directory| {
        responsible(directory, &actor, draft)?;
        let event = DraftEvent::Refused(Arc::new(Refused {
            operation: OperationId::from_str(&body.operation)?,
            actor,
            recorded_at: crate::session::now(),
            draft,
            draft_hash: hash,
            reason: body.reason,
            evidence: None,
        }));
        append(directory, event, draft, hash, None)
    })
}

async fn correct(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    bytes: Bytes,
) -> Result<Json<DraftAnswer>, ServerError> {
    let actor = personal(&state, &headers)?;
    let body: DraftCorrectBody = crate::signed_json::read(&state, &headers, &bytes)?;
    let draft = OperationId::from_str(&id)?;
    let hash = creation_hash(&body.creation_hash)?;
    with_directory(&state, |directory| {
        let reviewer = responsible(directory, &actor, draft)?;
        let at = crate::session::now();
        let corrected = Arc::new(creation(
            body.replacement,
            actor.clone(),
            reviewer,
            at,
            Some(draft),
        )?);
        let corrected_hash = lys_identity::encoding::payload_commitment(
            &lys_identity::draft_event::encode(&DraftEvent::Created(Arc::clone(&corrected))),
        );
        let replacement = corrected.operation;
        let event = DraftEvent::Correction(Arc::new(Correction {
            operation: OperationId::from_str(&body.operation)?,
            actor,
            recorded_at: at,
            draft,
            draft_hash: hash,
            reason: body.reason,
            corrected,
            corrected_hash,
        }));
        append(directory, event, draft, hash, Some(replacement))
    })
}
