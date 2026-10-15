//! Person consent admits only own readings; decisions never enter a personal route.

use axum::Json;
use axum::body::Body;
use axum::http::{Method, Request, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use lys_identity::{Actor, AuthMethod, IdentityId, LoginBinding, PersonId, Provenance};
use serde_json::{Value, json};
use tower::ServiceExt;

use super::{Endpoint, ResultValue};
use crate::error::ServerError;
use crate::mcp_oauth::Connection;

const VIEWS: [&str; 3] = ["/me", "/dashboard", "/drafts"];

tokio::task_local! {
    static PERSON_READING: Actor;
}

pub(crate) fn reading_actor() -> Option<Actor> {
    PERSON_READING.try_with(Clone::clone).ok()
}

fn actor(endpoint: &Endpoint, connection: &Connection) -> Result<Actor, ServerError> {
    let state = endpoint
        .state
        .as_deref()
        .ok_or_else(|| ServerError::ConfigInvalid {
            reason: "the person-read connection has no directory".to_owned(),
        })?;
    let consent = connection
        .person_reads
        .as_ref()
        .ok_or(ServerError::NoPerson)?;
    let person: PersonId = consent.person.parse()?;
    let actor = Actor::new(
        LoginBinding::new(&consent.issuer, &consent.subject)?,
        Provenance::new(AuthMethod::Oidc, consent.authenticated_at),
    );
    crate::routes::with_directory(state, |directory| {
        let projection = directory.projection()?;
        let caller = Actor::new(
            actor.binding().clone(),
            Provenance::by_pass(
                connection.agent,
                crate::mcp_oauth::LAUNCH,
                &connection.client_id,
            )?,
        );
        crate::caller_admission::active_caller(projection, &caller)?;
        let held = projection
            .record(IdentityId::Agent(connection.agent))
            .ok_or(ServerError::NoPerson)?;
        if held.responsible() != Some(person)
            || crate::read_api::own_person(projection, &actor)? != person
        {
            return Err(ServerError::NotAdmitted {
                reason: "this connection no longer answers to the person who consented",
            });
        }
        Ok(actor)
    })
}

fn receipt(
    endpoint: &Endpoint,
    connection: &Connection,
    method: &str,
    uri: &Uri,
    message: &[u8],
) -> Result<Value, ServerError> {
    let state = endpoint
        .state
        .as_deref()
        .ok_or_else(|| ServerError::ConfigInvalid {
            reason: "the person-read connection has no receipt store".to_owned(),
        })?;
    let mut reads = endpoint
        .reads
        .lock()
        .map_err(|error| ServerError::ConfigInvalid {
            reason: format!("the person-read receipt lock is unavailable: {error}"),
        })?;
    if reads.is_none() {
        *reads = Some(crate::mcp_receipts::Reads::open(state)?);
    }
    reads
        .as_mut()
        .ok_or_else(|| ServerError::ConfigInvalid {
            reason: "the opened person-read receipt store is absent".to_owned(),
        })?
        .before(connection, method, uri, message)
}

fn audit_refusal(error: &ServerError) -> Response {
    (StatusCode::SERVICE_UNAVAILABLE, Json(json!({
        "refusal":"McpReadReceiptUnavailable",
        "reason":format!("your read was not served because its receipt could not be kept: {error}"),
    }))).into_response()
}

pub(super) async fn discovery(
    endpoint: &Endpoint,
    connection: &Connection,
    message: &[u8],
) -> ResultValue {
    if let Err(error) = actor(endpoint, connection) {
        return crate::mcp_callers::rendered(error.into_response()).await;
    }
    let uri = Uri::from_static("/mcp");
    let leaf = match receipt(endpoint, connection, "POST", &uri, message) {
        Ok(leaf) => leaf,
        Err(error) => return crate::mcp_callers::rendered(audit_refusal(&error)).await,
    };
    let person = &connection
        .person_reads
        .as_ref()
        .ok_or_else(|| (-32603, "person consent is missing".to_owned()))?
        .person;
    let mut answer = crate::mcp_callers::rendered(Json(json!({
        "connection":{"id":connection.agent.to_string(),"client":connection.client_id,"person":person,"mode":"person_reads"},
        "message":"Person reads permits only your own identity, agents and directory drafts. Decisions, approvals, giving and signing need you in Lys in the browser. Product-draft reads are not available through this consent yet.",
        "routes":VIEWS.iter().map(|path| json!({"method":"GET","path":path})).collect::<Vec<_>>(),
        "resources":[],
    })).into_response()).await?;
    crate::mcp_receipts::attach(&mut answer, "receipt", leaf)
        .map_err(|error| (-32603, error.to_string()))?;
    Ok(answer)
}

pub(super) struct Reading<'a> {
    pub(super) method: &'a str,
    pub(super) uri: Uri,
    pub(super) has_body: bool,
    pub(super) message: &'a [u8],
}

pub(super) async fn call(
    endpoint: &Endpoint,
    connection: &Connection,
    reading: Reading<'_>,
) -> ResultValue {
    let actor = match actor(endpoint, connection) {
        Ok(actor) => actor,
        Err(error) => return crate::mcp_callers::rendered(error.into_response()).await,
    };
    if reading.method != "GET" {
        return crate::mcp_callers::rendered(presence(endpoint, reading.uri.path())).await;
    }
    if !VIEWS.contains(&reading.uri.path()) || reading.has_body {
        return crate::mcp_callers::rendered((StatusCode::FORBIDDEN, Json(json!({
            "refusal":"PersonReadNotConsented",
            "reason":"this connection may read only your identity, own agents and own directory drafts; product drafts need a callable own-person read helper",
        }))).into_response()).await;
    }
    let leaf = match receipt(
        endpoint,
        connection,
        reading.method,
        &reading.uri,
        reading.message,
    ) {
        Ok(leaf) => leaf,
        Err(error) => return crate::mcp_callers::rendered(audit_refusal(&error)).await,
    };
    let request = Request::builder()
        .method(Method::GET)
        .uri(reading.uri)
        .header("accept", "application/json")
        .body(Body::empty())
        .map_err(|error| {
            (
                -32603,
                format!("the own-view read could not be formed: {error}"),
            )
        })?;
    let response = PERSON_READING
        .scope(actor, endpoint.router.clone().oneshot(request))
        .await;
    let response = match response {
        Ok(response) => response,
        Err(error) => match error {},
    };
    let made = response.status().is_success();
    let mut result = crate::mcp_callers::rendered(response).await?;
    crate::mcp_receipts::attach(&mut result, if made { "receipt" } else { "asked" }, leaf)
        .map_err(|error| (-32603, error.to_string()))?;
    Ok(result)
}

fn presence(endpoint: &Endpoint, path: &str) -> Response {
    let mut words = "changing this record";
    let mut url = endpoint.origin.clone();
    let parts: Vec<_> = path.split('/').collect();
    if let [
        "",
        "drafts" | "product-drafts",
        id,
        "approve" | "refuse" | "correct",
    ] = parts.as_slice()
        && id.parse::<lys_identity::OperationId>().is_ok()
    {
        words = "deciding a draft";
        url = format!("{}/#/access/drafts?draft={id}", endpoint.origin);
    } else if path.starts_with("/grants") {
        words = "giving access";
        url.push_str("/#/access");
    }
    (StatusCode::FORBIDDEN, Json(json!({
        "refusal":"PersonPresenceRequired", "reason":format!("{words} needs you in Lys: {url}"), "url":url,
    }))).into_response()
}
