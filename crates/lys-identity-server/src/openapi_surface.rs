//! Only the contract the surface needs, closed over its referenced schemas.
use crate::error::ServerError;
use axum::body::Bytes;
use axum::http::{HeaderValue, header};
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

#[derive(Serialize, utoipa::ToSchema)]
pub(crate) struct SurfaceContract {
    #[schema(value_type = Object)]
    paths: BTreeMap<String, Value>,
    #[schema(value_type = Object)]
    components: Value,
}

static SURFACE: OnceLock<Result<Bytes, String>> = OnceLock::new();

fn references(value: &Value, pending: &mut Vec<String>) {
    match value {
        Value::Object(members) => {
            if let Some(Value::String(reference)) = members.get("$ref") {
                pending.push(reference.clone());
            }
            for value in members.values() {
                references(value, pending);
            }
        }
        Value::Array(values) => {
            for value in values {
                references(value, pending);
            }
        }
        _ => {}
    }
}

fn project(document: &Value) -> Result<SurfaceContract, String> {
    let mut paths = BTreeMap::new();
    let mut pending = Vec::new();
    for path in ["/agents", "/network/machines/{id}/agents"] {
        let post = document
            .get("paths")
            .and_then(|paths| paths.get(path))
            .and_then(|route| route.get("post"))
            .ok_or_else(|| format!("the surface contract has no POST {path}"))?;
        references(post, &mut pending);
        paths.insert(path.to_owned(), json!({"post": post}));
    }
    let mut seen = BTreeSet::new();
    let mut schemas = BTreeMap::new();
    while let Some(reference) = pending.pop() {
        if !seen.insert(reference.clone()) {
            continue;
        }
        let name = reference
            .strip_prefix("#/components/schemas/")
            .ok_or_else(|| format!("unsupported surface reference {reference}"))?;
        let value = document
            .pointer(&reference[1..])
            .ok_or_else(|| format!("missing surface reference {reference}"))?;
        references(value, &mut pending);
        schemas.insert(name.to_owned(), value.clone());
    }
    Ok(SurfaceContract {
        paths,
        components: json!({"schemas": schemas}),
    })
}

pub(crate) fn prepare() -> Result<&'static Bytes, ServerError> {
    SURFACE
        .get_or_init(|| {
            let document: Value =
                serde_json::from_slice(super::prepare().map_err(|error| error.to_string())?)
                    .map_err(|error| error.to_string())?;
            serde_json::to_vec(&project(&document)?)
                .map(Bytes::from)
                .map_err(|error| error.to_string())
        })
        .as_ref()
        .map_err(|reason| ServerError::ConfigInvalid {
            reason: reason.clone(),
        })
}

pub(super) async fn served() -> Result<Response, ServerError> {
    let mut response = prepare()?.clone().into_response();
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    Ok(response)
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_surface_contract_is_small_complete_and_reuses_cached_bytes()
    -> Result<(), Box<dyn std::error::Error>> {
        let full = super::super::prepare()?;
        let first = super::prepare()?;
        let second = super::prepare()?;
        assert!(std::ptr::eq(first, second));
        assert!(first.len() < full.len() / 2);
        let value: serde_json::Value = serde_json::from_slice(first)?;
        assert_eq!(value["paths"].as_object().ok_or("no paths")?.len(), 2);
        let mut references = Vec::new();
        super::references(&value, &mut references);
        for reference in references {
            assert!(value.pointer(&reference[1..]).is_some(), "{reference}");
        }
        assert!(
            value["components"]["schemas"]["AgentRegistrationBody"]["properties"]
                .get("answers_to")
                .is_some()
        );
        Ok(())
    }
}
