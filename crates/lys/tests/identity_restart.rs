//! `ID001_DEPLOY`, its restart half: stopping and starting the whole
//! deployment preserves the issuer's identity and the database's contents.
//!
//! Container-backed: declared `test = false`, run only on the identity leg of
//! .land/gates.sh, and refused as `container_runtime_missing` without a
//! runtime. Test identities only (CN2).

mod identity_support;

use identity_support::fixtures::{Failure, Venue, text};
use identity_support::server::Stack;

/// Rows the restart must keep, one query per schema.
const CONTENTS: [(&str, &str); 3] = [
    (
        "clients",
        "select string_agg(id, ',' order by id) from rauthy.clients",
    ),
    ("signing keys", "select count(*) from rauthy.jwks"),
    (
        "spicedb migrations",
        "select count(*) from spicedb.alembic_version",
    ),
];

#[test]
fn id001_deploy_restart_preserves_issuer_and_contents() -> Result<(), Failure> {
    let venue = Venue::fresh("restart", None)?;
    let stack = Stack::up(&venue)?;
    stack.wait_ready()?;
    stack.configure()?;
    let before = stack.wait_ready()?;
    let contents_before = contents(&stack)?;
    assert!(
        contents_before[0].contains("platform"),
        "{contents_before:?}"
    );

    // The whole deployment down and up again, its volumes kept.
    stack.compose().ok(&["down"])?;
    stack.compose().ok(&["up", "-d"])?;
    let after = stack.wait_ready()?;

    assert_eq!(
        after["issuer"], before["issuer"],
        "the issuer changed across restart"
    );
    assert_eq!(
        after["signing_keys"], before["signing_keys"],
        "the signing keys changed"
    );
    assert_eq!(
        contents(&stack)?,
        contents_before,
        "the database lost contents"
    );
    let report = stack.configure()?;
    let operations = report["operations"].as_array().ok_or("no operations")?;
    assert_eq!(operations.len(), 4);
    assert!(
        operations.iter().all(|op| op["outcome"] == "unchanged"),
        "{report}"
    );
    Ok(())
}

fn contents(stack: &Stack<'_>) -> Result<Vec<String>, Failure> {
    let mut rows = Vec::with_capacity(CONTENTS.len());
    for (what, sql) in CONTENTS {
        let output = stack.psql("identity_admin", sql)?;
        if !output.status.success() {
            return Err(format!("{what}: {}", text(&output.stderr)).into());
        }
        rows.push(text(&output.stdout).trim().to_string());
    }
    Ok(rows)
}
