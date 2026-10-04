//! The generated document exposes the same limit collections and unavailable figures as the routes.

use std::error::Error;

use lys_identity_server::openapi::document;
use serde_json::Value;

type TestResult = Result<(), Box<dyn Error>>;

fn schema<'a>(document: &'a Value, schema: &'a Value) -> Result<&'a Value, Box<dyn Error>> {
    if let Some(reference) = schema["$ref"].as_str() {
        document
            .pointer(
                reference
                    .strip_prefix('#')
                    .ok_or("nonlocal schema reference")?,
            )
            .ok_or_else(|| "missing referenced schema".into())
    } else {
        Ok(schema)
    }
}

fn body<'a>(
    document: &'a Value,
    path: &str,
    method: &str,
    response: bool,
) -> Result<&'a Value, Box<dyn Error>> {
    let operation = &document["paths"][path][method];
    let value = if response {
        &operation["responses"]["200"]["content"]["application/json"]["schema"]
    } else {
        &operation["requestBody"]["content"]["application/json"]["schema"]
    };
    assert!(
        !value.is_null(),
        "missing {method} {path} schema: {operation}"
    );
    schema(document, value)
}

#[test]
fn budget_put_and_get_describe_one_versioned_collection_and_the_used_invariant() -> TestResult {
    let document = document()?;
    let request = body(&document, "/budgets/{kind}/{id}", "put", false)?;
    for field in ["limits", "warn_at", "version"] {
        assert!(
            !request["properties"][field].is_null(),
            "missing {field}: {request}"
        );
    }
    assert!(request["properties"]["act"].is_null());
    let limit = schema(&document, &request["properties"]["limits"]["items"])?;
    for field in ["unit", "amount", "period", "act", "zone"] {
        assert!(
            !limit["properties"][field].is_null(),
            "missing {field}: {limit}"
        );
    }
    let get = body(&document, "/budgets/{kind}/{id}", "get", true)?;
    assert_eq!(get, body(&document, "/budgets/{kind}/{id}", "put", true)?);
    for field in [
        "holder",
        "limits",
        "warn_at",
        "zone",
        "version",
        "by",
        "at",
        "used",
        "unavailable",
        "within",
    ] {
        assert!(
            !get["properties"][field].is_null(),
            "missing {field}: {get}"
        );
    }
    let used = schema(&document, &get["properties"]["used"]["items"])?;
    let keys = used["properties"]
        .as_object()
        .ok_or("no used row properties")?;
    assert_eq!(keys.len(), 6, "{used}");
    for field in [
        "unit",
        "period",
        "figure",
        "since_ms",
        "unavailable",
        "reported",
    ] {
        assert!(keys.contains_key(field), "{used}");
    }
    Ok(())
}

#[test]
fn team_aliases_share_budget_schemas_and_organisation_put_has_optimistic_version() -> TestResult {
    let document = document()?;
    for (method, response) in [("get", true), ("put", false), ("put", true)] {
        assert_eq!(
            body(&document, "/teams/{id}/budget", method, response)?,
            body(&document, "/budgets/{kind}/{id}", method, response)?
        );
    }
    let request = body(&document, "/configuration", "put", false)?;
    for field in ["zone", "version"] {
        assert!(!request["properties"][field].is_null(), "{request}");
    }
    let answer = body(&document, "/configuration", "put", true)?;
    for field in ["zone", "version", "by", "at"] {
        assert!(!answer["properties"][field].is_null(), "{answer}");
    }
    Ok(())
}

#[test]
fn start_admission_and_limit_boundary_refusals_are_documented() -> TestResult {
    let document = document()?;
    for path in ["/agents/{id}/start-command", "/agents/{id}/start"] {
        let start = document["paths"][path]["post"].to_string();
        assert!(start.contains("BudgetExhausted"), "{start}");
    }
    let put = document["paths"]["/budgets/{kind}/{id}"]["put"].to_string();
    for name in [
        "TeamUnitRefused",
        "BudgetUnitUnavailable",
        "BudgetAmountRefused",
        "BudgetPeriodRefused",
        "BudgetWarningRefused",
        "BudgetLimitsRefused",
        "BudgetZoneRefused",
        "BudgetVersionConflict",
    ] {
        assert!(put.contains(name), "missing {name}: {put}");
    }
    Ok(())
}
