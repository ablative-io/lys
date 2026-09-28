#![cfg(test)]
//! DIRECTORY-045 R3: a running service answers `GET /authority` with how
//! access is managed and `build`, the commit it was built from. The commit
//! expected is read from git as a second party to the stamp `build.rs` took.

use std::error::Error;
use std::process::Command;

use identity_contract::harness::Service;
use lys_identity_server::admission::AUTHORITY;

type TestResult = Result<(), Box<dyn Error>>;

/// What git says now about the tree this crate is in: its HEAD commit, or
/// the words when there is no git tree tracking this crate.
fn head() -> String {
    let git = |args: &[&str]| {
        Command::new("git")
            .arg("--no-optional-locks")
            .args(args)
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .output()
            .ok()
            .filter(|output| output.status.success())
            .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
    };
    let tracked = git(&["ls-files", "--error-unmatch", "Cargo.toml"]).is_some();
    match git(&["rev-parse", "HEAD"]).filter(|_| tracked) {
        Some(commit) => commit,
        None => "not built from a git commit".to_string(),
    }
}

#[tokio::test]
async fn the_authority_names_the_build_that_answers() -> TestResult {
    let service = Service::start().await?;
    let (status, body) = service.get("/authority", None).await?;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["authority"], AUTHORITY, "{body}");
    let build = body["build"].as_str().ok_or("no build in the answer")?;
    let expected = head();
    assert!(
        build == expected || build == format!("{expected}; dirty"),
        "the service says it is build `{build}`, git says `{expected}`"
    );
    Ok(())
}
