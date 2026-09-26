//! `ID001_SHARED_DB`: Rauthy and `SpiceDB` both write and reopen against the
//! same `PostgreSQL` database, their migrations do not collide, and neither
//! service role can read the other's schema. Then the database plus the
//! documented key, config and cache files are backed up, lost, restored,
//! and readiness is repeated (deploy/identity/README.md, "Backup and
//! restore").
//!
//! The schema and relationship written to `SpiceDB` are test fixtures that
//! prove it writes to the shared database; they are not grants, and nothing
//! asks `SpiceDB` for a decision (DIRECTORY-002 R4).
//!
//! Container-backed: declared `test = false`, run only on the identity leg of
//! .land/gates.sh, and refused as `container_runtime_missing` without a
//! runtime. Test identities only (CN2).

mod identity_support;

use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;

use identity_support::fixtures::{Failure, Venue, text};
use identity_support::server::Stack;
use serde_json::{Value, json};

/// The bootstrap superuser the example config names.
const ADMIN: &str = "identity_admin";

/// A fixture schema: one object type holding one relation.
const FIXTURE_SCHEMA: &str =
    "definition fixture_user {}\ndefinition fixture_document {\n  relation viewer: fixture_user\n}";

#[test]
fn id001_shared_db_coexist_isolate_and_restore() -> Result<(), Failure> {
    let venue = Venue::fresh("shared", None)?;
    let stack = Stack::up(&venue)?;
    let before = stack.wait_ready()?;

    // Both services write to the one database.
    stack.configure()?;
    spicedb(
        &venue,
        "/v1/schema/write",
        &json!({ "schema": FIXTURE_SCHEMA }),
    )?;
    spicedb(
        &venue,
        "/v1/relationships/write",
        &json!({ "updates": [{
        "operation": "OPERATION_TOUCH",
        "relationship": {
            "resource": { "objectType": "fixture_document", "objectId": "shared_db" },
            "relation": "viewer",
            "subject": { "object": { "objectType": "fixture_user", "objectId": "id001" } }
        }
    }] }),
    )?;
    coexistence(&stack)?;
    isolation(&stack)?;

    // Both reopen what they wrote.
    stack.compose().ok(&["restart", "rauthy", "spicedb"])?;
    stack.wait_ready()?;
    reads_back(&stack, &venue)?;

    // Backup: the SQL dump, the Hiqlite cache volume, and the private key and
    // config files, with Rauthy and SpiceDB stopped so the copy is consistent.
    let backup_dir = tempfile::tempdir()?;
    let backup = backup_dir.path().to_path_buf();
    stack.compose().ok(&["stop", "rauthy", "spicedb"])?;
    let dump = postgres_tool(&stack, &venue, &["pg_dump", "--create", "-Fc"], None)?;
    assert!(!dump.is_empty(), "the dump is empty");
    let image = stack.compose().config()?["services"]["postgres"]["image"]
        .as_str()
        .ok_or("compose declares no postgres image")?
        .to_string();
    let volume = format!("{}_rauthy-data", venue.project());
    let mount = format!("{}:/backup", backup.display());
    docker(&[
        "run",
        "--rm",
        "-v",
        &format!("{volume}:/data:ro"),
        "-v",
        &mount,
        "--entrypoint",
        "tar",
        &image,
        "-C",
        "/data",
        "-czf",
        "/backup/rauthy-data.tgz",
        ".",
    ])?;
    copy_private(&venue.private_dir(), &backup.join("private"))?;

    // Loss: the database volume, the cache volume and the private files.
    stack.compose().ok(&["down", "-v"])?;
    std::fs::remove_dir_all(venue.private_dir())?;

    // Restore: keys and config, then the cache volume, then the database.
    copy_private(&backup.join("private"), &venue.private_dir())?;
    stack.compose().ok(&["create"])?;
    docker(&[
        "run",
        "--rm",
        "-v",
        &format!("{volume}:/data"),
        "-v",
        &mount,
        "--entrypoint",
        "tar",
        &image,
        "-C",
        "/data",
        "-xzf",
        "/backup/rauthy-data.tgz",
    ])?;
    stack.compose().ok(&["up", "-d", "postgres"])?;
    wait_for_database(&stack)?;
    let restore = [
        "pg_restore",
        "--dbname",
        "postgres",
        "--clean",
        "--create",
        "--if-exists",
        "--exit-on-error",
    ];
    postgres_tool(&stack, &venue, &restore, Some(&dump))?;
    stack.compose().ok(&["up", "-d"])?;

    // Readiness repeated: the same issuer and keys, and both services' data.
    let restored = stack.wait_ready()?;
    assert_eq!(
        restored["issuer"], before["issuer"],
        "the issuer changed across restore"
    );
    assert_eq!(
        restored["signing_keys"], before["signing_keys"],
        "the signing keys changed"
    );
    coexistence(&stack)?;
    isolation(&stack)?;
    reads_back(&stack, &venue)
}

/// Both migration histories exist, each only in its own schema, nothing
/// sits in `public`, each role's search path is its own schema, and each
/// service's rows are there under its own role.
fn coexistence(stack: &Stack<'_>) -> Result<(), Failure> {
    let histories = query(
        stack,
        ADMIN,
        "select table_schema || '.' || table_name from information_schema.tables \
         where table_name in ('refinery_schema_history', 'alembic_version') order by 1",
    )?;
    assert_eq!(
        histories,
        "rauthy.refinery_schema_history\nspicedb.alembic_version"
    );
    let public = query(
        stack,
        ADMIN,
        "select count(*) from pg_tables where schemaname = 'public'",
    )?;
    assert_eq!(public, "0", "a migration wrote to the public schema");
    let search_paths = query(
        stack,
        ADMIN,
        "select r.rolname || ':' || array_to_string(s.setconfig, ',') from pg_db_role_setting s \
         join pg_roles r on r.oid = s.setrole order by 1",
    )?;
    assert_eq!(
        search_paths,
        "rauthy:search_path=rauthy\nspicedb:search_path=spicedb"
    );
    let clients = query(
        stack,
        "rauthy",
        "select string_agg(id, ',' order by id) from clients",
    )?;
    assert_eq!(clients, "cambium,platform,rauthy");
    let tuples = query(stack, "spicedb", "select count(*) from relation_tuple")?;
    assert_ne!(tuples, "0", "SpiceDB's relationship is not in its schema");
    Ok(())
}

/// Neither service role can read the other's schema or write to `public`.
fn isolation(stack: &Stack<'_>) -> Result<(), Failure> {
    let cases = [
        (
            "rauthy",
            "select count(*) from spicedb.relation_tuple",
            "permission denied for schema spicedb",
        ),
        (
            "spicedb",
            "select count(*) from rauthy.clients",
            "permission denied for schema rauthy",
        ),
        (
            "rauthy",
            "create table public.lys_probe (id int)",
            "permission denied for schema public",
        ),
        (
            "spicedb",
            "create table public.lys_probe (id int)",
            "permission denied for schema public",
        ),
    ];
    let mut refused = 0;
    for (user, sql, expected) in cases {
        let output = stack.psql(user, sql)?;
        let stderr = text(&output.stderr);
        assert!(!output.status.success(), "{user} was allowed: {sql}");
        assert!(stderr.contains(expected), "{user} {sql}: {stderr}");
        refused += 1;
    }
    assert_eq!(refused, cases.len());
    Ok(())
}

/// Rauthy still holds its clients and `SpiceDB` its relationship.
fn reads_back(stack: &Stack<'_>, venue: &Venue) -> Result<(), Failure> {
    let report = stack.configure()?;
    let operations = report["operations"].as_array().ok_or("no operations")?;
    assert!(
        operations.iter().all(|op| op["outcome"] == "unchanged"),
        "{report}"
    );
    let filter = json!({
        "consistency": { "fullyConsistent": true },
        "relationshipFilter": { "resourceType": "fixture_document" }
    });
    let read = spicedb(venue, "/v1/relationships/read", &filter)?;
    assert!(
        read.contains("\"objectId\":\"id001\""),
        "SpiceDB lost its relationship"
    );
    Ok(())
}

fn query(stack: &Stack<'_>, user: &str, sql: &str) -> Result<String, Failure> {
    let output = stack.psql(user, sql)?;
    if output.status.success() {
        Ok(text(&output.stdout).trim().to_string())
    } else {
        Err(format!("{user}: {sql}: {}", text(&output.stderr)).into())
    }
}

/// Runs a `PostgreSQL` client tool in the database container as the
/// bootstrap superuser, feeding `input` on stdin, and returns its stdout.
fn postgres_tool(
    stack: &Stack<'_>,
    venue: &Venue,
    tool: &[&str],
    input: Option<&[u8]>,
) -> Result<Vec<u8>, Failure> {
    let mut args = vec!["exec", "-T", "-e", "PGPASSWORD", "postgres"];
    args.extend_from_slice(tool);
    args.extend(["--host", "127.0.0.1", "--username", ADMIN]);
    if tool.first() == Some(&"pg_dump") {
        args.extend(["--dbname", "identity"]);
    }
    let mut command = stack.compose().command(&args);
    command
        .env("PGPASSWORD", venue.secret("pg_admin_password")?)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn()?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(input.unwrap_or_default())?;
    }
    let output = child.wait_with_output()?;
    if output.status.success() {
        Ok(output.stdout)
    } else {
        Err(format!("{}: {}", tool.join(" "), text(&output.stderr)).into())
    }
}

fn wait_for_database(stack: &Stack<'_>) -> Result<(), Failure> {
    while !stack.psql(ADMIN, "select 1")?.status.success() {
        std::thread::sleep(Duration::from_secs(1));
    }
    Ok(())
}

fn docker(args: &[&str]) -> Result<(), Failure> {
    let output = Command::new("docker").args(args).output()?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "docker {}: {}",
            args.first().unwrap_or(&""),
            text(&output.stderr)
        )
        .into())
    }
}

/// Copies a private directory tree, keeping it owner-only.
fn copy_private(from: &Path, to: &Path) -> Result<(), Failure> {
    use std::os::unix::fs::PermissionsExt;

    std::fs::create_dir_all(to)?;
    std::fs::set_permissions(to, std::fs::Permissions::from_mode(0o700))?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_private(&entry.path(), &target)?;
        } else {
            std::fs::copy(entry.path(), &target)?;
            std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o600))?;
        }
    }
    Ok(())
}

/// One authenticated request to `SpiceDB`'s HTTP API; the whole response.
fn spicedb(venue: &Venue, path: &str, body: &Value) -> Result<String, Failure> {
    let key = venue.secret("spicedb_preshared_key")?;
    let body = body.to_string();
    let mut stream = TcpStream::connect(("127.0.0.1", venue.ports().spicedb_http))?;
    let head = format!(
        "POST {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\nContent-Type: application/json\r\nContent-Length: {}\r\nAuthorization: Bearer ",
        body.len()
    );
    stream.write_all(head.as_bytes())?;
    stream.write_all(key.as_bytes())?;
    stream.write_all(b"\r\n\r\n")?;
    stream.write_all(body.as_bytes())?;
    let mut response = Vec::new();
    stream.read_to_end(&mut response)?;
    let response = text(&response);
    let status = response.lines().next().unwrap_or_default().to_string();
    if status.starts_with("HTTP/1.1 200") {
        Ok(response)
    } else {
        Err(format!("SpiceDB {path}: {status}").into())
    }
}
