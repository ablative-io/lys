//! Repeated document reads do no schema generation work.

use std::cell::Cell;
use std::error::Error;

thread_local! {
    static BUILDS: Cell<usize> = const { Cell::new(0) };
}

pub(super) fn built() {
    BUILDS.set(BUILDS.get() + 1);
}

#[tokio::test]
async fn a_second_document_read_does_not_generate_the_schema_again() -> Result<(), Box<dyn Error>> {
    let _first = super::served().await?;
    let first = BUILDS.get();
    let _second = super::served().await?;
    assert_eq!(BUILDS.get(), first);
    Ok(())
}

#[test]
fn an_invalid_document_is_refused_before_it_can_be_cached() -> Result<(), Box<dyn Error>> {
    let mut document = super::document()?;
    document["paths"]["/apps"]["post"]["requestBody"]["content"]["application/json"]["schema"]["$ref"] =
        serde_json::json!("#/components/schemas/Missing");
    assert!(matches!(
        super::encode(&document),
        Err(crate::error::ServerError::ConfigInvalid { .. })
    ));
    Ok(())
}
