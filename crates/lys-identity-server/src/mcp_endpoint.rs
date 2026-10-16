//! Stateless MCP transport over the existing admitted HTTP router.
use axum::body::to_bytes;
use axum::extract::{Request, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::{Json, Router, routing::post};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::Arc;

use crate::error::ServerError;

mod call;
mod discovery;
mod person;
mod refusal;
#[cfg(test)]
mod refusal_tests;
use call::call;
pub(crate) use call::dispatch_router as dispatch;
pub(crate) use person::reading_actor;

#[path = "mcp_endpoint/run_refusal.rs"]
mod run_refusal;
pub(crate) use run_refusal::run_refusals;

#[cfg(test)]
#[path = "mcp_endpoint_contract_tests.rs"]
mod contract_tests;
#[cfg(test)]
#[path = "mcp_endpoint_receipt_tests.rs"]
mod receipt_tests;
#[cfg(test)]
#[path = "mcp_endpoint_test_support.rs"]
mod test_support;
#[cfg(test)]
#[path = "mcp_endpoint_tests.rs"]
mod tests;

#[derive(Deserialize, Serialize, utoipa::ToSchema)]
#[serde(transparent)]
#[schema(value_type = Object)]
pub(crate) struct Envelope(Value);
const VERSIONS: [&str; 3] = ["2025-03-26", "2025-06-18", "2025-11-25"];
/// The tools Lys offers: four, whatever Lys grows to (DIRECTORY-049 C369:
/// a compact list over the published API). What a call acts on is the route
/// it names, never a tool of its own, so the list never grows with the routes.
const TOOLS: [(&str, &str); 4] = [
    (
        "what-can-I-do",
        "List the routes, resources and actions this connection may reach under its live grants.",
    ),
    (
        "read",
        "Read through an existing HTTP route with the caller's credentials and admission.",
    ),
    (
        "change",
        "Change through an existing HTTP route. Its validation, authority checks and refusals apply unchanged.",
    ),
    (
        "drafts",
        "Ask an existing draft route with the caller's credentials. This transport grants no draft authority.",
    ),
];

struct Endpoint {
    router: Arc<Router>,
    state: Option<Arc<crate::routes::AppState>>,
    origin: String,
    apps: Option<Arc<crate::mcp_oauth::Apps>>,
    refused: crate::mcp_receipts::RefusedAsks,
    reads: std::sync::Mutex<Option<crate::mcp_receipts::Reads>>,
}

#[cfg(test)]
pub(crate) fn routes(router: Router, origin: &str) -> Result<Router, ServerError> {
    registered(router, origin, None, None)
}

pub(crate) fn admitted_routes(
    router: Router,
    origin: &str,
    state: Arc<crate::routes::AppState>,
    apps: Arc<crate::mcp_oauth::Apps>,
) -> Result<Router, ServerError> {
    registered(router, origin, Some(state), Some(apps))
}

fn registered(
    router: Router,
    origin: &str,
    state: Option<Arc<crate::routes::AppState>>,
    apps: Option<Arc<crate::mcp_oauth::Apps>>,
) -> Result<Router, ServerError> {
    let origin = reqwest::Url::parse(origin).map_err(|error| ServerError::ConfigInvalid {
        reason: format!("the MCP origin is invalid: {error}"),
    })?;
    let router = Arc::new(router);
    if let Some(state) = &state {
        state.draft_execution.attach(Arc::downgrade(&router))?;
    }
    let endpoint = Arc::new(Endpoint {
        router,
        state,
        origin: origin.origin().ascii_serialization(),
        apps,
        refused: crate::mcp_receipts::RefusedAsks::new(),
        reads: std::sync::Mutex::new(None),
    });
    Ok(Router::new()
        .route("/mcp", post(message).get(no_stream))
        .with_state(endpoint))
}

fn fault(id: &Value, status: StatusCode, code: i32, reason: impl Into<String>) -> Response {
    (
        status,
        Json(Envelope(
            json!({"jsonrpc":"2.0", "id":id, "error":{"code":code,"message":reason.into()}}),
        )),
    )
        .into_response()
}

fn transport(headers: &HeaderMap, endpoint: &Endpoint) -> Result<(), (StatusCode, &'static str)> {
    if let Some(origin) = headers.get(header::ORIGIN)
        && origin.to_str().ok() != Some(endpoint.origin.as_str())
    {
        return Err((
            StatusCode::FORBIDDEN,
            "the MCP Origin is not the service origin",
        ));
    }
    if let Some(version) = headers.get("mcp-protocol-version")
        && !version
            .to_str()
            .is_ok_and(|version| VERSIONS.contains(&version))
    {
        return Err((
            StatusCode::BAD_REQUEST,
            "the MCP protocol version is not supported",
        ));
    }
    Ok(())
}

async fn no_stream(State(endpoint): State<Arc<Endpoint>>, headers: HeaderMap) -> Response {
    match transport(&headers, &endpoint) {
        Ok(()) => (StatusCode::METHOD_NOT_ALLOWED, [(header::ALLOW, "POST")]).into_response(),
        Err((status, reason)) => fault(&Value::Null, status, -32600, reason),
    }
}

fn accepts(headers: &HeaderMap, media: &str) -> bool {
    headers.get_all(header::ACCEPT).iter().any(|value| {
        value.to_str().is_ok_and(|value| {
            value.split(',').any(|entry| {
                entry
                    .split(';')
                    .next()
                    .is_some_and(|entry| entry.trim() == media)
            })
        })
    })
}

fn transport_refusal(request: &Request, endpoint: &Endpoint) -> Option<Response> {
    if request.headers().contains_key(crate::grant_tokens::HEADER) {
        if let Err(error) = crate::grant_tokens::header(request.headers()) {
            return Some(error.into_response());
        }
        if request
            .headers()
            .contains_key(crate::agent_signature::HEADER)
            || request.headers().contains_key(header::AUTHORIZATION)
        {
            return Some(
                ServerError::AgentSignatureRefused {
                    reason: "a grant token cannot carry another credential",
                }
                .into_response(),
            );
        }
    }
    if request
        .headers()
        .contains_key(crate::agent_signature::HEADER)
        && request
            .headers()
            .get_all(header::COOKIE)
            .iter()
            .any(|value| {
                value.to_str().is_ok_and(|cookies| {
                    cookies.split(';').any(|cookie| {
                        cookie
                            .trim()
                            .split_once('=')
                            .is_some_and(|(name, _)| name == crate::session::COOKIE)
                    })
                })
            })
    {
        return Some(
            ServerError::AgentSignatureRefused {
                reason: "an agent signature cannot carry a session cookie",
            }
            .into_response(),
        );
    }
    if let Err((status, reason)) = transport(request.headers(), endpoint) {
        return Some(fault(&Value::Null, status, -32600, reason));
    }
    if !accepts(request.headers(), "application/json")
        || !accepts(request.headers(), "text/event-stream")
    {
        return Some(fault(
            &Value::Null,
            StatusCode::NOT_ACCEPTABLE,
            -32600,
            "MCP requires Accept: application/json, text/event-stream",
        ));
    }
    if !request
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| {
            value
                .split(';')
                .next()
                .is_some_and(|media| media.trim() == "application/json")
        })
    {
        return Some(fault(
            &Value::Null,
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            -32600,
            "MCP requires Content-Type: application/json",
        ));
    }
    None
}

async fn message(State(endpoint): State<Arc<Endpoint>>, request: Request) -> Response {
    if let Some(refused) = transport_refusal(&request, &endpoint) {
        return refused;
    }
    // Read whole: `signed_first.rs` has already held an unverified caller's
    // body to `UNVERIFIED_BODY_LIMIT`, and a judged caller has none.
    let (parts, body) = request.into_parts();
    let bytes = match to_bytes(body, usize::MAX).await {
        Ok(bytes) => bytes,
        Err(error) => {
            return fault(
                &Value::Null,
                StatusCode::BAD_REQUEST,
                -32600,
                format!("the MCP body could not be read: {error}"),
            );
        }
    };
    let value = match serde_json::from_slice::<Envelope>(&bytes) {
        Ok(Envelope(value)) => value,
        Err(error) => {
            return fault(
                &Value::Null,
                StatusCode::BAD_REQUEST,
                -32700,
                format!("the MCP message is not JSON: {error}"),
            );
        }
    };
    let id = value.get("id").cloned().unwrap_or(Value::Null);
    if value["jsonrpc"] != "2.0"
        || !value.is_object()
        || value
            .get("id")
            .is_some_and(|id| !id.is_string() && id.as_i64().is_none() && id.as_u64().is_none())
    {
        return fault(
            &Value::Null,
            StatusCode::BAD_REQUEST,
            -32600,
            "a JSON-RPC 2.0 message with a string or integer id is required",
        );
    }
    let Some(method) = value["method"].as_str() else {
        return fault(
            &id,
            StatusCode::BAD_REQUEST,
            -32600,
            "the MCP message names no method",
        );
    };
    if value.get("id").is_none() {
        return if method == "notifications/initialized" {
            StatusCode::ACCEPTED.into_response()
        } else {
            fault(
                &Value::Null,
                StatusCode::BAD_REQUEST,
                -32601,
                "the MCP notification is not supported",
            )
        };
    }
    let signer = match crate::mcp_callers::signer(endpoint.state.as_ref(), &parts, &bytes) {
        Ok(signer) => signer,
        Err(error) => return error.into_response(),
    };
    let app = match crate::mcp_callers::connected_app(endpoint.apps.as_deref(), &parts) {
        Ok(app) => app,
        Err(error) => return error.into_response(),
    };
    if method == "tools/list" {
        return tools(&id, &value["params"]);
    }
    let result = match method {
        "initialize" => initialize(&value["params"]),
        "ping" => Ok(json!({})),
        "tools/call" => call(&endpoint, parts, &value["params"], (signer, app), &bytes).await,
        _ => Err((-32601, "the MCP method is not supported".to_owned())),
    };
    match result {
        Ok(result) => {
            Json(Envelope(json!({"jsonrpc":"2.0", "id":id, "result":result}))).into_response()
        }
        Err((code, reason)) => fault(&id, StatusCode::OK, code, reason),
    }
}

type ResultValue = Result<Value, (i32, String)>;

fn initialize(params: &Value) -> ResultValue {
    let version = params["protocolVersion"]
        .as_str()
        .ok_or_else(|| (-32602, "initialize requires protocolVersion".to_owned()))?;
    if !params["capabilities"].is_object()
        || !params["clientInfo"]["name"].is_string()
        || !params["clientInfo"]["version"].is_string()
    {
        return Err((
            -32602,
            "initialize requires capabilities and clientInfo name/version".to_owned(),
        ));
    }
    let version = if VERSIONS.contains(&version) {
        version
    } else {
        "2025-11-25"
    };
    Ok(
        json!({"protocolVersion":version, "capabilities":{"tools":{}}, "serverInfo":{"name":"lys", "version":env!("CARGO_PKG_VERSION")},
            "instructions":"Call what-can-I-do first with {} to read this connection's consent, declared routes and live grants. Use read for a permitted view, change for an admitted agent change, and drafts for directory draft routes. Person reads permits only the consenting person's own identity, agents, directory drafts and product drafts, with receipts. Decisions, approvals, giving and signing as that person need Lys in the browser. Agent-mode calls retain each HTTP route's authority and validation; a listed route alone never grants authority."}),
    )
}

fn tools(id: &Value, params: &Value) -> Response {
    if !params.is_null() && (!params.is_object() || params.get("cursor").is_some()) {
        return fault(
            id,
            StatusCode::OK,
            -32602,
            "this tool list has no continuation cursor",
        );
    }
    let schemas = [
        json!({"type":"object","properties":{},"required":[],"additionalProperties":false}),
        json!({"type":"object","properties":{"method":{"type":"string","enum":["GET","POST"],"default":"GET"},"path":{"type":"string","description":"An absolute local read route, including its query."},"body":{}},"required":["path"],"additionalProperties":false}),
        json!({"type":"object","properties":{"method":{"type":"string","enum":["POST","PUT","PATCH","DELETE"]},"path":{"type":"string","description":"An absolute local change route."},"body":{}},"required":["method","path"],"additionalProperties":false}),
        json!({"type":"object","properties":{"method":{"type":"string","enum":["GET","POST"],"default":"GET"},"path":{"type":"string","pattern":"^/drafts(?:/|\\?|$)","description":"A declared draft route."},"body":{}},"required":["path"],"additionalProperties":false}),
    ];
    let tools: Vec<_> = TOOLS.into_iter().zip(schemas).map(|((name, description), schema)| json!({"name":name, "description":description, "inputSchema":schema})).collect();
    Json(Envelope(
        json!({"jsonrpc":"2.0", "id":id, "result":{"tools":tools}}),
    ))
    .into_response()
}
