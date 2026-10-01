//! MCP tools generated from the HTTP route contract.

use axum::body::Bytes;
use lys_openapi::Method;
use serde_json::{Map, Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

use crate::error::ServerError;
use crate::openapi_table::{E, TABLE};

pub(crate) struct Catalogue {
    encoded: Bytes,
    routes: BTreeMap<String, (Method, &'static str)>,
}

static CATALOGUE: OnceLock<Result<Catalogue, String>> = OnceLock::new();

pub(crate) fn prepare() -> Result<&'static Catalogue, ServerError> {
    CATALOGUE
        .get_or_init(build)
        .as_ref()
        .map_err(|reason| ServerError::ConfigInvalid {
            reason: format!("the MCP tool catalogue is invalid: {reason}"),
        })
}

impl Catalogue {
    pub(crate) fn encoded(&self) -> &Bytes {
        &self.encoded
    }

    pub(crate) fn resolve(&self, name: &str, method: &str, path: &str) -> Result<(), String> {
        let (expected, template) = self
            .routes
            .get(name)
            .ok_or_else(|| format!("unknown tool {name}"))?;
        if !method.eq_ignore_ascii_case(expected.word()) || !matches_path(template, path) {
            return Err(format!(
                "tool route mismatch for {name}: requires {} {template}",
                expected.word().to_ascii_uppercase()
            ));
        }
        Ok(())
    }
}

fn matches_path(template: &str, path: &str) -> bool {
    let mut concrete = path.split('/');
    for segment in template.split('/') {
        let Some(part) = concrete.next() else {
            return false;
        };
        if segment.starts_with('{') && segment.ends_with('}') {
            if part.is_empty() {
                return false;
            }
        } else if segment != part {
            return false;
        }
    }
    concrete.next().is_none()
}

fn build() -> Result<Catalogue, String> {
    let api = crate::openapi::api();
    let document = api.document().map_err(|faults| faults.join("; "))?;
    let schemas = document["components"]["schemas"]
        .as_object()
        .ok_or("the route document has no schemas")?;
    let contracts: BTreeMap<_, _> = api
        .routes()
        .iter()
        .map(|route| ((route.method, route.path), route))
        .collect();
    let mut tools = Vec::with_capacity(TABLE.len());
    let mut routes = BTreeMap::new();
    for E(method, path, summary, ..) in TABLE {
        let contract = contracts
            .get(&(*method, *path))
            .ok_or_else(|| format!("no route contract for {} {path}", method.word()))?;
        let name = document["paths"][path][method.word()]["operationId"]
            .as_str()
            .ok_or_else(|| format!("no operation name for {} {path}", method.word()))?;
        if name.is_empty()
            || name.len() > 128
            || !name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
        {
            return Err(format!("invalid MCP tool name {name}"));
        }
        if routes.insert(name.to_owned(), (*method, *path)).is_some() {
            return Err(format!("duplicate MCP tool name {name}"));
        }
        let request = contract.request.as_deref();
        let response = contract.response.as_deref();
        let mut input = json!({
            "type":"object",
            "properties":{
                "method":{"type":"string","const":method.word().to_ascii_uppercase()},
                "path":{"type":"string","description":format!("A local path matching {path}, including its query.")},
                "body":{"description":"no schema"}
            },
            "required":["method","path"], "additionalProperties":false
        });
        let mut metadata = json!({
            "lys/method":method.word(), "lys/path":path,
            "lys/requestSchema":request.unwrap_or("no schema"),
            "lys/responseSchema":response.unwrap_or("no schema")
        });
        if let Some(request) = request {
            let definitions = definitions(request, schemas)?;
            if *method == Method::Get {
                metadata["lys/querySchema"] = json!({
                    "$ref":format!("#/$defs/{request}"), "$defs":definitions
                });
            } else {
                input["properties"]["body"] = json!({"$ref":format!("#/$defs/{request}")});
                input["$defs"] = Value::Object(definitions);
                input["required"] = json!(["method", "path", "body"]);
            }
        }
        let mut tool = json!({
            "name":name,
            "description":format!("{} {path}: {summary}. Request: {}; response: {}.",
                method.word().to_ascii_uppercase(), request.unwrap_or("no schema"), response.unwrap_or("no schema")),
            "inputSchema":input, "_meta":metadata
        });
        if let Some(response) = response {
            tool["outputSchema"] = json!({
                "type":"object", "properties":{
                    "status":{"type":"integer"},
                    "body":{"$ref":format!("#/$defs/{response}")}
                }, "required":["status","body"],
                "$defs":definitions(response, schemas)?
            });
        }
        tools.push(tool);
    }
    let encoded = serde_json::to_vec(&json!({"tools":tools}))
        .map_err(|error| format!("the MCP tool list could not be encoded: {error}"))?;
    Ok(Catalogue {
        encoded: Bytes::from(encoded),
        routes,
    })
}

fn definitions(root: &str, schemas: &Map<String, Value>) -> Result<Map<String, Value>, String> {
    let mut pending = BTreeSet::from([root.to_owned()]);
    let mut held = Map::new();
    while let Some(name) = pending.pop_first() {
        if held.contains_key(&name) {
            continue;
        }
        let mut schema = schemas
            .get(&name)
            .cloned()
            .ok_or_else(|| format!("the route schema {name} is missing"))?;
        local_references(&mut schema, &mut pending)?;
        held.insert(name, schema);
    }
    Ok(held)
}

fn local_references(value: &mut Value, pending: &mut BTreeSet<String>) -> Result<(), String> {
    match value {
        Value::Object(members) => {
            for (key, member) in members {
                if key == "$ref" {
                    let target = member
                        .as_str()
                        .ok_or("a schema reference is not a string")?;
                    let name = target
                        .strip_prefix("#/components/schemas/")
                        .ok_or_else(|| format!("unsupported route schema reference {target}"))?;
                    pending.insert(name.to_owned());
                    *member = Value::String(format!("#/$defs/{name}"));
                } else {
                    local_references(member, pending)?;
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                local_references(item, pending)?;
            }
        }
        _ => {}
    }
    Ok(())
}

#[cfg(test)]
#[path = "mcp_tools_tests.rs"]
mod tests;
