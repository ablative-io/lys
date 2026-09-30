//! Each entry reads independently so a document failure names its source.

use std::error::Error;

use serde_json::Value;
use utoipa::openapi::RefOr;
use utoipa::openapi::path::PathItem;
use utoipa::openapi::schema::Schema;

type TestResult = Result<(), Box<dyn Error>>;

fn unread<T: serde::de::DeserializeOwned>(entries: &Value) -> Result<Vec<String>, Box<dyn Error>> {
    let entries = entries
        .as_object()
        .ok_or("document entries are not an object")?;
    let mut faults = Vec::new();
    for (name, entry) in entries {
        if let Err(error) = serde_json::from_value::<T>(entry.clone()) {
            faults.push(format!("{name}: {error}; entry={entry}"));
        }
    }
    Ok(faults)
}

#[test]
fn every_component_schema_reads_as_openapi_3_1() -> TestResult {
    let document = lys_identity_server::openapi::document()?;
    let faults = unread::<RefOr<Schema>>(&document["components"]["schemas"])?;
    assert!(faults.is_empty(), "unread component schemas: {faults:#?}");
    Ok(())
}

#[test]
fn every_path_entry_reads_as_openapi_3_1() -> TestResult {
    let document = lys_identity_server::openapi::document()?;
    let faults = unread::<PathItem>(&document["paths"])?;
    assert!(faults.is_empty(), "unread path entries: {faults:#?}");
    Ok(())
}
