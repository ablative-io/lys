#![cfg(test)]
//! The published cannot-give reasons stay in step with admission's reasons.

use std::error::Error;

use lys_identity::grants::CannotGiveReason;
use lys_identity_server::openapi::document;
use serde_json::json;

#[test]
fn the_published_cannot_give_reason_enum_contains_every_reason() -> Result<(), Box<dyn Error>> {
    let document = document()?;
    let schemas = &document["components"]["schemas"];
    let expected = json!(CannotGiveReason::ALL.map(CannotGiveReason::name));
    assert_eq!(schemas["CannotGiveReasonView"]["enum"], expected);
    assert_eq!(
        schemas["CannotGiveItemView"]["properties"]["reason"]["$ref"],
        "#/components/schemas/CannotGiveReasonView"
    );
    println!(
        "published_reason_schema={}",
        schemas["CannotGiveReasonView"]
    );
    Ok(())
}
