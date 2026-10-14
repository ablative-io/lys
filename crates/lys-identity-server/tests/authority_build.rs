#![cfg(test)]
//! DIRECTORY-045 R3: a running service answers `GET /authority` with how
//! access is managed and `build`, the commit it was built from. The commit
//! expected is read from git as a second party to the stamp `build.rs` took.

use identity_contract::harness::Service;
use lys_identity_server::admission::AUTHORITY;
use std::error::Error;

#[path = "support/build_head.rs"]
mod build_head;

type TestResult = Result<(), Box<dyn Error>>;

#[tokio::test]
async fn the_authority_names_the_build_that_answers() -> TestResult {
    let expected = build_head::at_build()?;
    let service = Service::start().await?;
    let (status, body) = service.get("/authority", None).await?;
    build_head::unchanged(&expected)?;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["authority"], AUTHORITY, "{body}");
    let build = body["build"].as_str().ok_or("no build in the answer")?;
    assert!(
        build == expected || build == format!("{expected}; dirty"),
        "the service says it is build `{build}`, git says `{expected}`"
    );
    Ok(())
}
