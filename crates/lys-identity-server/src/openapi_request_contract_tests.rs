//! Surface mutation fixtures stay equal to the server-derived required fields.
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

type TestResult<T> = Result<T, Box<dyn std::error::Error>>;

fn required(document: &Value, schema: &Value) -> TestResult<BTreeSet<String>> {
    if let Some(reference) = schema.get("$ref").and_then(Value::as_str) {
        return required(
            document,
            document
                .pointer(reference.strip_prefix('#').ok_or("external schema")?)
                .ok_or("missing schema")?,
        );
    }
    let mut fields = BTreeSet::new();
    if let Some(required) = schema.get("required").and_then(Value::as_array) {
        for field in required {
            fields.insert(
                field
                    .as_str()
                    .ok_or("non-string required field")?
                    .to_owned(),
            );
        }
    }
    if let Some(parts) = schema.get("allOf").and_then(Value::as_array) {
        for part in parts {
            fields.extend(required(document, part)?);
        }
    }
    Ok(fields)
}

#[test]
fn surface_required_fields_equal_the_actual_server_request_schemas() -> TestResult<()> {
    let document = super::document()?;
    let mut contracts = BTreeMap::new();
    for (path, route) in document["paths"].as_object().ok_or("no paths")? {
        for (method, operation) in route.as_object().ok_or("no methods")? {
            if !matches!(method.as_str(), "post" | "put" | "patch" | "delete") {
                continue;
            }
            if let Some(schema) = operation.pointer("/requestBody/content/application~1json/schema")
            {
                let fields = required(&document, schema)?;
                if fields.contains("operation") {
                    contracts.insert(format!("{} {path}", method.to_uppercase()), fields);
                }
            }
        }
    }
    let surface: BTreeMap<String, BTreeSet<String>> = serde_json::from_str(include_str!(
        "../../../surface/identity/tests/server-required-fields.json"
    ))?;
    assert_eq!(
        surface, contracts,
        "regenerate surface required fields from the current OpenAPI document"
    );
    Ok(())
}
