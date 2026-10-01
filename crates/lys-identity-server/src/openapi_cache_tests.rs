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
