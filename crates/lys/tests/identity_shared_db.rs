//! ID001_SHARED_DB: Rauthy and `SpiceDB` both write to, and reopen against,
//! the one `PostgreSQL` database; their migrations land in their own schemas
//! without colliding; neither service role can read the other's schema; and
//! after the database and the documented key, config and cache dependencies
//! are restored into fresh volumes, readiness is reached again with the data
//! intact.
//!
//! The `SpiceDB` schema and relationship written here are test fixtures that
//! prove the shared database, not grants: nothing in `lys` writes to `SpiceDB`.
//!
//! Container-backed: declared `test = false`, run only on the identity leg of
//! `.land/gates.sh`.

pub mod identity_support;

use std::process::Command;

use identity_support::fixtures::output_text;
use identity_support::{Stack, TestResult, server};

const FIXTURE_SCHEMA: &str = "definition test_user {}\n\ndefinition test_document {\n    relation reader: test_user\n}\n";

fn spicedb_post(stack: &Stack, path: &str, body: &str) -> TestResult<server::Reply> {
    let bearer = stack.fixture.spicedb_bearer()?;
    server::request(
        &stack.fixture.spicedb(),
        "POST",
        path,
        &[("Authorization", bearer.as_str())],
        Some(body),
    )
}

fn fixture_relationship_present(stack: &Stack) -> TestResult<bool> {
    let reply = spicedb_post(
        stack,
        "/v1/relationships/read",
        r#"{"consistency":{"fullyConsistent":true},"relationshipFilter":{"resourceType":"test_document"}}"#,
    )?;
    Ok(reply.status == 200 && reply.body.contains("\"objectId\":\"fixture-subject\""))
}

fn clients(stack: &Stack) -> TestResult<String> {
    let reply = stack.rauthy_admin("GET", "/auth/v1/clients")?;
    let mut ids: Vec<String> = serde_json::from_str::<Vec<serde_json::Value>>(&reply.body)?
        .iter()
        .filter_map(|client| client["id"].as_str().map(str::to_string))
        .collect();
    ids.sort();
    Ok(ids.join(","))
}

fn psql_ok(stack: &Stack, role: &str, sql: &str) -> TestResult<String> {
    let output = stack.compose.psql(role, sql)?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(format!("psql as {role} failed: {}", output_text(&output)).into())
    }
}

/// Run a throwaway container that mounts the stack's Rauthy cache volume and
/// a host directory, to archive or unpack the cache.
fn cache_volume(stack: &Stack, backup: &std::path::Path, script: &str) -> TestResult {
    let volume = format!("{}_rauthy-data:/data", stack.compose.project);
    let mount = format!("{}:/backup", backup.display());
    let image = "public.ecr.aws/docker/library/postgres:17.11-bookworm@sha256:639ab7ceb90e13123085b741fb31ef493fba25463002f6da665352e7b534b652";
    let output = Command::new("docker")
        .args(["run", "--rm", "-v", volume.as_str(), "-v", mount.as_str(), "--entrypoint", "sh", image, "-c", script])
        .output()?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!("cache volume step failed: {}", output_text(&output)).into())
    }
}

#[test]
fn rauthy_and_spicedb_share_one_database_and_survive_a_restore() -> TestResult {
    let stack = Stack::up("shareddb")?;

    // Both write: Rauthy through configure, SpiceDB through its own API.
    stack.configure()?;
    let schema = spicedb_post(&stack, "/v1/schema/write", &serde_json::json!({ "schema": FIXTURE_SCHEMA }).to_string())?;
    assert_eq!(schema.status, 200, "{}", schema.body);
    let relationship = r#"{"updates":[{"operation":"OPERATION_TOUCH","relationship":{"resource":{"objectType":"test_document","objectId":"fixture-document"},"relation":"reader","subject":{"object":{"objectType":"test_user","objectId":"fixture-subject"}}}}]}"#;
    let written = spicedb_post(&stack, "/v1/relationships/write", relationship)?;
    assert_eq!(written.status, 200, "{}", written.body);

    // One database, two schemas, two migration histories, nothing public.
    let tables = psql_ok(
        &stack,
        "postgres",
        "SELECT table_schema || '.' || table_name FROM information_schema.tables \
         WHERE table_schema NOT IN ('pg_catalog', 'information_schema') ORDER BY 1",
    )?;
    let tables: Vec<&str> = tables.lines().collect();
    assert!(tables.contains(&"rauthy.refinery_schema_history"), "{tables:?}");
    assert!(tables.contains(&"spicedb.alembic_version"), "{tables:?}");
    assert!(tables.contains(&"rauthy.clients"), "{tables:?}");
    assert!(tables.iter().all(|table| table.starts_with("rauthy.") || table.starts_with("spicedb.")), "{tables:?}");
    assert!(!tables.contains(&"spicedb.refinery_schema_history") && !tables.contains(&"rauthy.alembic_version"));
    assert_eq!(psql_ok(&stack, "identity_rauthy", "SHOW search_path")?, "rauthy");
    assert_eq!(psql_ok(&stack, "identity_spicedb", "SHOW search_path")?, "spicedb");

    // Neither service role can read the other's schema.
    let mut denied = 0;
    for (role, table) in [("identity_rauthy", "spicedb.relation_tuple"), ("identity_spicedb", "rauthy.clients")] {
        let output = stack.compose.psql(role, &format!("SELECT count(*) FROM {table}"))?;
        let text = output_text(&output);
        assert!(!output.status.success(), "{role} read {table}");
        assert!(text.contains("permission denied for schema"), "{text}");
        denied += 1;
    }
    assert_eq!(denied, 2);

    // Both reopen: restart the services and read their data back.
    let clients_before = clients(&stack)?;
    stack.compose.run_ok(&["restart", "rauthy", "spicedb"])?;
    stack.wait_ready()?;
    assert_eq!(clients(&stack)?, clients_before);
    assert!(fixture_relationship_present(&stack)?);

    // Back up: the database, and the Rauthy cache volume. The env file and
    // credentials (keys and config) stay in the state directory.
    let jwks_before = server::request(&stack.fixture.rauthy(), "GET", "/auth/v1/oidc/certs", &[], None)?.body;
    let backup = stack.fixture.dir.path().join("backup");
    std::fs::create_dir_all(&backup)?;
    let dump = backup.join("identity.dump");
    stack.compose.dump(&dump)?;
    stack.compose.run_ok(&["stop", "rauthy"])?;
    cache_volume(&stack, &backup, "tar -C /data -czf /backup/rauthy-data.tgz .")?;

    // Lose every volume, then restore into fresh ones.
    stack.compose.down()?;
    stack.compose.run_ok(&["up", "-d", "--wait", "postgres"])?;
    stack.compose.restore(&dump)?;
    stack.compose.run_ok(&["create", "rauthy"])?;
    cache_volume(&stack, &backup, "tar -C /data -xzf /backup/rauthy-data.tgz")?;
    stack.compose.run_ok(&["up", "-d"])?;
    stack.wait_ready()?;

    // Readiness again, with the data intact.
    let health = stack.fixture.identity("health", &[])?;
    assert!(health.status.success(), "{}", output_text(&health));
    assert_eq!(clients(&stack)?, clients_before);
    assert!(fixture_relationship_present(&stack)?);
    let jwks_after = server::request(&stack.fixture.rauthy(), "GET", "/auth/v1/oidc/certs", &[], None)?.body;
    assert_eq!(jwks_after, jwks_before, "the signing keys did not survive the restore");
    let second = stack.configure()?;
    assert!(!second.contains("created") && !second.contains("updated"), "{second}");
    println!("ID001_SHARED_DB: restore of {} bytes repeated readiness", std::fs::metadata(&dump)?.len());
    Ok(())
}
