#![cfg(test)]
//! `ID001_SHARED_DB`: Rauthy and `SpiceDB` both write to and reopen the same
//! `PostgreSQL` database; their migrations land in their own schemas without
//! colliding; neither service role can read the other's schema; and a
//! restore of the database with the documented key, config and cache
//! dependencies becomes ready again with everything written before it.
//! The `SpiceDB` schema and relationship written here are test fixtures, not
//! grants. Container-backed: runs only on the identity leg of .land/gates.sh.

pub mod identity_support;

use std::collections::BTreeSet;
use std::process::Command;

use identity_support::compose::{self, require_runtime};
use identity_support::fixtures::{Deployment, TestResult, output_text, succeeded};
use identity_support::server::{rauthy_json, request};

const FIXTURE_SCHEMA: &str = r#"{"schema":"definition fixture/user {}\ndefinition fixture/doc {\n  relation viewer: fixture/user\n}"}"#;
const FIXTURE_RELATIONSHIP: &str = r#"{"updates":[{"operation":"OPERATION_TOUCH","relationship":{"resource":{"objectType":"fixture/doc","objectId":"shared-db"},"relation":"viewer","subject":{"object":{"objectType":"fixture/user","objectId":"reopened"}}}}]}"#;
const FIXTURE_READ: &str = r#"{"consistency":{"fullyConsistent":true},"relationshipFilter":{"resourceType":"fixture/doc"}}"#;

fn spicedb(deployment: &Deployment, path: &str, body: &str) -> TestResult<String> {
    let bearer = format!("Bearer {}", deployment.secret("spicedb-preshared-key")?);
    let (status, text) = request(
        &deployment.spicedb_address(),
        "POST",
        path,
        &[("Authorization", &bearer)],
        Some(body),
    )?;
    if status != 200 {
        return Err(format!("SpiceDB {path} answered {status}: {text}").into());
    }
    Ok(text)
}

fn psql_lines(deployment: &Deployment, role: &str, sql: &str) -> TestResult<Vec<String>> {
    let output = compose::psql(deployment, role, sql)?;
    succeeded(&output, &format!("psql as {role}"))?;
    Ok(String::from_utf8(output.stdout)?
        .lines()
        .map(str::to_string)
        .collect())
}

fn tables_in(deployment: &Deployment, schema: &str) -> TestResult<BTreeSet<String>> {
    Ok(psql_lines(
        deployment,
        "identity_admin",
        &format!(
            "SELECT table_name FROM information_schema.tables WHERE table_schema = '{schema}'"
        ),
    )?
    .into_iter()
    .collect())
}

fn written_state(deployment: &Deployment) -> TestResult<(serde_json::Value, bool)> {
    let clients = rauthy_json(deployment, "GET", "/auth/v1/clients/cambium")?;
    let relationships = spicedb(deployment, "/v1/relationships/read", FIXTURE_READ)?;
    Ok((clients, relationships.contains("\"reopened\"")))
}

fn docker(args: &[&str]) -> TestResult {
    let output = Command::new("docker").args(args).output()?;
    succeeded(&output, &format!("docker {}", args.join(" ")))
}

#[test]
fn id001_shared_db_one_database_two_schemas_restore_and_ready_again() -> TestResult {
    require_runtime()?;
    let deployment = Deployment::new("shared", |text| text)?;
    compose::render(&deployment)?;
    compose::up(&deployment, &[])?;
    compose::wait_ready(&deployment)?;
    succeeded(&deployment.lys("configure")?, "configure")?;
    spicedb(&deployment, "/v1/schema/write", FIXTURE_SCHEMA)?;
    spicedb(&deployment, "/v1/relationships/write", FIXTURE_RELATIONSHIP)?;

    let rauthy_tables = tables_in(&deployment, "rauthy")?;
    let spicedb_tables = tables_in(&deployment, "spicedb")?;
    assert!(rauthy_tables.contains("clients"), "{rauthy_tables:?}");
    assert!(
        rauthy_tables.contains("refinery_schema_history"),
        "{rauthy_tables:?}"
    );
    assert!(
        spicedb_tables.contains("relation_tuple"),
        "{spicedb_tables:?}"
    );
    assert!(
        rauthy_tables.is_disjoint(&spicedb_tables),
        "the migrations collided"
    );
    assert!(
        tables_in(&deployment, "public")?.is_empty(),
        "a migration wrote to public"
    );
    let databases = psql_lines(
        &deployment,
        "identity_admin",
        "SELECT datname FROM pg_database WHERE NOT datistemplate AND datname NOT IN ('postgres')",
    )?;
    assert_eq!(databases, ["identity"], "one database");
    let paths = psql_lines(
        &deployment,
        "identity_admin",
        "SELECT r.rolname || ' ' || array_to_string(s.setconfig, ',') FROM pg_db_role_setting s \
         JOIN pg_roles r ON r.oid = s.setrole ORDER BY 1",
    )?;
    assert_eq!(
        paths,
        ["rauthy search_path=rauthy", "spicedb search_path=spicedb"]
    );

    for (role, foreign) in [
        ("rauthy", "spicedb.relation_tuple"),
        ("spicedb", "rauthy.clients"),
    ] {
        let output = compose::psql(
            &deployment,
            role,
            &format!("SELECT count(*) FROM {foreign}"),
        )?;
        let text = output_text(&output);
        assert!(!output.status.success(), "{role} read {foreign}");
        assert!(text.contains("permission denied for schema"), "{text}");
    }

    let before = written_state(&deployment)?;
    assert!(before.1, "the fixture relationship was not written");
    for service in ["rauthy", "spicedb"] {
        compose::stop(&deployment, service)?;
        compose::start(&deployment, service)?;
    }
    compose::wait_ready(&deployment)?;
    assert_eq!(
        written_state(&deployment)?,
        before,
        "a service lost its writes on reopen"
    );

    let backup = tempfile::tempdir()?;
    let dump = compose::compose(
        &deployment,
        &[
            "exec",
            "-T",
            "postgres",
            "pg_dump",
            "-U",
            "identity_admin",
            "-Fc",
            "identity",
        ],
    )?;
    succeeded(&dump, "pg_dump")?;
    std::fs::write(backup.path().join("identity.dump"), &dump.stdout)?;
    let volume = format!("{}_rauthy-data", deployment.project);
    let postgres_image = "docker.io/library/postgres:17.11@sha256:d74eeac9a635390a49bc21bd49fccd973de707e2a53a76ac49b552b8712ec46f";
    let backup_mount = format!("{}:/backup", backup.path().display());
    let volume_mount = format!("{volume}:/data");
    compose::stop(&deployment, "rauthy")?;
    docker(&[
        "run",
        "--rm",
        "-v",
        &volume_mount,
        "-v",
        &backup_mount,
        postgres_image,
        "tar",
        "-C",
        "/data",
        "-cf",
        "/backup/rauthy-data.tar",
        ".",
    ])?;

    succeeded(
        &compose::compose(&deployment, &["down", "-v"])?,
        "docker compose down -v",
    )?;
    compose::up(&deployment, &["postgres"])?;
    let restore_started = std::time::Instant::now();
    while !compose::psql(&deployment, "identity_admin", "SELECT 1")?
        .status
        .success()
    {
        assert!(
            restore_started.elapsed() < compose::READY_WITHIN,
            "postgres never answered"
        );
        std::thread::sleep(std::time::Duration::from_secs(2));
    }
    let mut restore = Command::new("docker");
    restore
        .args(["compose", "-f"])
        .arg(identity_support::fixtures::repository_root().join("deploy/identity/compose.yaml"))
        .arg("--env-file")
        .arg(deployment.state.join("compose.env"))
        .args(["-p", &deployment.project, "--profile", "bundled-db"])
        .args([
            "exec",
            "-T",
            "postgres",
            "pg_restore",
            "-U",
            "identity_admin",
            "-d",
            "identity",
        ])
        .args(["--clean", "--if-exists", "--exit-on-error"])
        .stdin(std::fs::File::open(backup.path().join("identity.dump"))?);
    succeeded(&restore.output()?, "pg_restore")?;
    succeeded(
        &compose::compose(&deployment, &["create", "--no-recreate", "rauthy"])?,
        "docker compose create rauthy",
    )?;
    docker(&[
        "run",
        "--rm",
        "-v",
        &volume_mount,
        "-v",
        &backup_mount,
        postgres_image,
        "tar",
        "-C",
        "/data",
        "-xpf",
        "/backup/rauthy-data.tar",
    ])?;
    compose::up(&deployment, &[])?;
    compose::wait_ready(&deployment)?;
    assert_eq!(
        written_state(&deployment)?,
        before,
        "the restore lost writes"
    );
    let again = deployment.lys("configure")?;
    succeeded(&again, "configure after restore")?;
    for line in output_text(&again)
        .lines()
        .filter(|line| line.contains("(operation "))
    {
        assert!(line.contains(": unchanged ("), "{line}");
    }
    Ok(())
}
