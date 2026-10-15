//! A refused tool call retains the route's decision and names its required authority.

use axum::http::{StatusCode, Uri};
use axum::response::Response;
use lys_identity::AgentId;
use serde_json::{Map, Value, json};

use super::{Endpoint, ResultValue};

pub(super) async fn rendered(
    endpoint: &Endpoint,
    method: &str,
    uri: &Uri,
    agent: Option<AgentId>,
    response: Response,
) -> ResultValue {
    if !matches!(
        response.status(),
        StatusCode::FORBIDDEN | StatusCode::NOT_FOUND
    ) {
        return crate::mcp_callers::rendered(response).await;
    }
    let mut result = crate::mcp_callers::rendered(response).await?;
    let text = result["content"][0]["text"]
        .as_str()
        .ok_or_else(|| (-32603, "the MCP response has no JSON text".to_owned()))?;
    let mut answer: Value = serde_json::from_str(text)
        .map_err(|error| (-32603, format!("the MCP response cannot be read: {error}")))?;
    let status = answer["status"].as_u64();
    let Some(name) = answer["body"]["refusal"].as_str() else {
        return Ok(result);
    };
    if !matches!(status, Some(403 | 404)) || !missing_authority(name) {
        return Ok(result);
    }
    let undeclared = name == "TokenScopeUndeclared";
    let scope = match crate::openapi_table::token_scope(method, uri.path()) {
        Ok(scope) => Some(scope),
        Err(crate::grant_tokens::TokenError::Undeclared) => None,
        Err(error) => {
            return Err((
                -32603,
                format!("the refused route's required scope cannot be read: {error}"),
            ));
        }
    };
    if scope.is_none() && !undeclared && !known_route(method, uri.path()) {
        return Ok(result);
    }
    let body = &mut answer["body"];
    let mut fields = fields(body);
    fields.insert("route".to_owned(), json!(uri.path()));
    fields.insert("method".to_owned(), json!(method));
    let reason = body["reason"].as_str().ok_or_else(|| {
        (
            -32603,
            "the MCP authority refusal names no reason".to_owned(),
        )
    })?;
    if let Some((resource, action)) = scope {
        fields.insert("resource_kind".to_owned(), json!(resource.kind()));
        fields.insert("resource_id".to_owned(), json!(resource.id()));
        fields.insert("action".to_owned(), json!(action.as_str()));
        fields.insert("scope_declared".to_owned(), json!(true));
        body["reason"] = json!(format!(
            "{reason}. Required route scope: {} on {} for {method} {}.",
            action.as_str(),
            resource,
            uri.path()
        ));
        // An obscured identity must not become a directory lookup through its refusal.
        if status == Some(403)
            && body.get("can_grant").is_none()
            && let (Some(state), Some(agent)) = (endpoint.state.as_deref(), agent)
        {
            match crate::who_can_grant::for_agent(state, agent, &resource, &action) {
                Ok(holders) => body["can_grant"] = json!(holders),
                Err(error) => {
                    body["grantor_lookup"] = json!({
                        "refusal":error.name(), "reason":error.to_string()
                    });
                }
            }
        }
    } else {
        // This identifies the undeclared HTTP operation, never a grantable resource.
        fields.insert("resource_kind".to_owned(), json!("http_route"));
        fields.insert("resource_id".to_owned(), json!(uri.path()));
        fields.insert("action".to_owned(), json!(method));
        fields.insert(
            "missing".to_owned(),
            json!("declared_resource_action_scope"),
        );
        body["reason"] = json!(format!(
            "{reason}. {method} {} has no declared resource/action scope; no grant can enable this call through this connection until Lys declares that scope.",
            uri.path()
        ));
        fields.insert("scope_declared".to_owned(), json!(false));
        body["can_grant"] = json!([]);
    }
    body["fields"] = Value::Object(fields);
    result["content"][0]["text"] = json!(answer.to_string());
    Ok(result)
}

fn missing_authority(name: &str) -> bool {
    matches!(
        name,
        "TokenScopeUndeclared"
            | "GrantTokenScopeMismatch"
            | "NotAdmitted"
            | "not_permitted"
            | "AgentNotVisible"
            | "NotHeld"
            | "NotHolder"
            | "PermissionAbsent"
            | "ActionsOutside"
            | "UseOnly"
            | "Revoked"
            | "Expired"
    )
}

fn fields(body: &Value) -> Map<String, Value> {
    match body.get("fields") {
        Some(Value::Object(fields)) => fields.clone(),
        Some(Value::Array(details)) if !details.is_empty() => {
            Map::from_iter([("details".to_owned(), json!(details))])
        }
        Some(value) if !value.is_null() && value != &json!([]) => {
            Map::from_iter([("details".to_owned(), value.clone())])
        }
        _ => Map::new(),
    }
}

fn known_route(method: &str, path: &str) -> bool {
    crate::openapi_table::TABLE.iter().any(|row| {
        if !method.eq_ignore_ascii_case(row.0.word()) {
            return false;
        }
        let mut concrete = path.split('/');
        row.1.split('/').all(|segment| {
            concrete.next().is_some_and(|value| {
                segment == value
                    || (!value.is_empty() && segment.starts_with('{') && segment.ends_with('}'))
            })
        }) && concrete.next().is_none()
    })
}
