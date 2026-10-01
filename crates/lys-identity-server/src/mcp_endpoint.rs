//! Stateless MCP transport over the existing admitted HTTP router.
use axum::body::{Body, to_bytes};
use axum::extract::{MatchedPath, OriginalUri, Request, State};
use axum::http::{HeaderMap, Method, StatusCode, Uri, header};
use axum::response::{IntoResponse, Response};
use axum::{Json, Router, routing::post};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::Arc;
use tower::ServiceExt;

use crate::error::ServerError;

#[cfg(test)]
#[path = "mcp_endpoint_tests.rs"]
mod tests;

const BODY_LIMIT: usize = 2 * 1024 * 1024;

#[derive(Deserialize, Serialize, utoipa::ToSchema)]
#[serde(transparent)]
#[schema(value_type = Object)]
pub(crate) struct Envelope(Value);
const VERSIONS: [&str; 3] = ["2025-03-26", "2025-06-18", "2025-11-25"];
const TOOLS: [(&str, &str); 4] = [
    (
        "what-can-I-do",
        "Ask the existing HTTP routes what the caller may do. Admission is decided by those routes.",
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
    router: Router,
    origin: String,
}

pub(crate) fn routes(router: Router, origin: &str) -> Result<Router, ServerError> {
    let origin = reqwest::Url::parse(origin).map_err(|error| ServerError::ConfigInvalid {
        reason: format!("the MCP origin is invalid: {error}"),
    })?;
    let endpoint = Arc::new(Endpoint {
        router,
        origin: origin.origin().ascii_serialization(),
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

async fn message(State(endpoint): State<Arc<Endpoint>>, request: Request) -> Response {
    if let Err((status, reason)) = transport(request.headers(), &endpoint) {
        return fault(&Value::Null, status, -32600, reason);
    }
    if !accepts(request.headers(), "application/json")
        || !accepts(request.headers(), "text/event-stream")
    {
        return fault(
            &Value::Null,
            StatusCode::NOT_ACCEPTABLE,
            -32600,
            "MCP requires Accept: application/json, text/event-stream",
        );
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
        return fault(
            &Value::Null,
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            -32600,
            "MCP requires Content-Type: application/json",
        );
    }
    let (parts, body) = request.into_parts();
    let bytes = match to_bytes(body, BODY_LIMIT).await {
        Ok(bytes) => bytes,
        Err(error) => {
            return fault(
                &Value::Null,
                StatusCode::PAYLOAD_TOO_LARGE,
                -32600,
                format!("the MCP body could not be read within its limit: {error}"),
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
    let result = match method {
        "initialize" => initialize(&value["params"]),
        "ping" => Ok(json!({})),
        "tools/list" => tools(&value["params"]),
        "tools/call" => call(&endpoint.router, parts, &value["params"]).await,
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
        json!({"protocolVersion":version, "capabilities":{"tools":{}}, "serverInfo":{"name":"lys", "version":env!("CARGO_PKG_VERSION")}}),
    )
}

fn tools(params: &Value) -> ResultValue {
    if !params.is_null() && (!params.is_object() || params.get("cursor").is_some()) {
        return Err((
            -32602,
            "this tool list has no continuation cursor".to_owned(),
        ));
    }
    let schema = json!({"type":"object","properties":{"method":{"type":"string","enum":["GET","POST","PUT","PATCH","DELETE"]},"path":{"type":"string","description":"An absolute local HTTP path, including its query."},"body":{}},"required":["method","path"],"additionalProperties":false});
    Ok(
        json!({"tools":TOOLS.map(|(name, description)| json!({"name":name,"description":description,"inputSchema":schema}))}),
    )
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Call {
    name: String,
    arguments: Arguments,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Arguments {
    method: String,
    path: String,
    body: Option<Value>,
}

async fn call(
    router: &Router,
    mut parts: axum::http::request::Parts,
    params: &Value,
) -> ResultValue {
    let call: Call = serde_json::from_value(params.clone())
        .map_err(|error| (-32602, format!("invalid tool call: {error}")))?;
    if !TOOLS.iter().any(|(name, _)| *name == call.name) {
        return Err((-32602, format!("unknown tool {}", call.name)));
    }
    if !["GET", "POST", "PUT", "PATCH", "DELETE"].contains(&call.arguments.method.as_str()) {
        return Err((
            -32602,
            "the tool call requires GET, POST, PUT, PATCH or DELETE".to_owned(),
        ));
    }
    let uri: Uri = call
        .arguments
        .path
        .parse()
        .map_err(|error| (-32602, format!("invalid route path: {error}")))?;
    if uri.scheme().is_some()
        || uri.authority().is_some()
        || !call.arguments.path.starts_with('/')
        || call.arguments.path.starts_with("//")
    {
        return Err((
            -32602,
            "the tool path must be local to the HTTP router".to_owned(),
        ));
    }
    parts.method = Method::from_bytes(call.arguments.method.as_bytes())
        .map_err(|error| (-32602, error.to_string()))?;
    parts.uri = uri;
    parts.extensions.remove::<MatchedPath>();
    parts.extensions.remove::<OriginalUri>();
    for name in [header::CONTENT_LENGTH, header::CONTENT_TYPE, header::ACCEPT] {
        parts.headers.remove(name);
    }
    parts.headers.insert(
        header::ACCEPT,
        axum::http::HeaderValue::from_static("application/json"),
    );
    let body = if let Some(body) = call.arguments.body {
        parts.headers.insert(
            header::CONTENT_TYPE,
            axum::http::HeaderValue::from_static("application/json"),
        );
        Body::from(serde_json::to_vec(&body).map_err(|error| (-32603, error.to_string()))?)
    } else {
        Body::empty()
    };
    let response = match router
        .clone()
        .oneshot(Request::from_parts(parts, body))
        .await
    {
        Ok(response) => response,
        Err(error) => match error {},
    };
    let status = response.status();
    let bytes = to_bytes(response.into_body(), BODY_LIMIT)
        .await
        .map_err(|error| {
            (
                -32603,
                format!("the HTTP route response exceeded the MCP limit or failed: {error}"),
            )
        })?;
    let body = match serde_json::from_slice::<Value>(&bytes) {
        Ok(body) => body,
        Err(_) => Value::String(String::from_utf8(bytes.to_vec()).map_err(|error| {
            (
                -32603,
                format!("the HTTP route response is not JSON or text: {error}"),
            )
        })?),
    };
    let answer = json!({"status":status.as_u16(),"body":body});
    Ok(
        json!({"isError":!status.is_success(),"content":[{"type":"text","text":answer.to_string()}],"structuredContent":answer}),
    )
}
