//! `ID001_DEPLOY_REFUSAL`: a missing secret, an invalid issuer or redirect,
//! and an unavailable database each produce a named failure, never a
//! substitute identity or development database. Also the ADR-005 case: a
//! database address is configuration and nothing local replaces it.
//!
//! Container-backed: declared `test = false`, run only on the identity leg of
//! .land/gates.sh, and refused as `container_runtime_missing` without a
//! runtime. Test identities only (CN2).

mod identity_support;

use identity_support::compose::Compose;
use identity_support::fixtures::{Failure, Venue, text};
use identity_support::server::{Stack, require_container_runtime};

/// The documentation address (TEST-NET-1) standing for a database on a
/// network device that does not answer.
const UNREACHABLE_HOST: &str = "192.0.2.10";

/// Runs `lys identity prepare` on the venue and returns its refusal.
fn prepare_refusal(venue: &Venue) -> Result<String, Failure> {
    let config = venue.config();
    let output = venue.lys(&["identity", "prepare", "--config", &config.to_string_lossy()])?;
    if output.status.success() {
        return Err("prepare accepted a config it must refuse".into());
    }
    Ok(text(&output.stderr))
}

/// Invalid issuers and redirect URIs are refused by name before anything
/// is written, and the database address reaches compose unchanged and is
/// refused by name when it does not answer.
#[test]
fn id001_deploy_refusal_config_and_database_address() -> Result<(), Failure> {
    require_container_runtime()?;
    let cases = [
        (
            "rauthy",
            "public_origin",
            "\"http://id.example.net\"",
            "invalid_issuer",
        ),
        (
            "rauthy",
            "public_origin",
            "\"https://id.example.net/auth\"",
            "invalid_issuer",
        ),
        (
            "clients.platform",
            "redirect_uris",
            "[\"http://app.example.net/callback\"]",
            "invalid_redirect_uri",
        ),
        (
            "clients.cambium",
            "redirect_uris",
            "[\"https://app.example.net/*\"]",
            "invalid_redirect_uri",
        ),
    ];
    let mut refused = 0;
    for (section, key, value, failure) in cases {
        let venue = Venue::fresh("refusal", Some(UNREACHABLE_HOST))?;
        venue.set(section, key, value)?;
        let stderr = prepare_refusal(&venue)?;
        assert!(
            stderr.contains(failure),
            "{section}.{key} = {value}: {stderr}"
        );
        assert!(
            !venue.private_dir().exists(),
            "a refused config wrote private files"
        );
        refused += 1;
    }
    assert_eq!(refused, cases.len());

    // The copied example with its database host set to 192.0.2.10, rendered
    // through the identity test support into the environment compose.yaml
    // interpolates.
    let venue = Venue::fresh("address", Some(UNREACHABLE_HOST))?;
    let compose = Compose::new(&venue);
    compose.render()?;
    assert_eq!(compose.environment("rauthy", "PG_HOST")?, UNREACHABLE_HOST);
    let uri = compose.environment("spicedb", "SPICEDB_DATASTORE_CONN_URI")?;
    let host = uri
        .split_once('@')
        .and_then(|(_, rest)| rest.split_once(':'))
        .map(|(host, _)| host.to_string())
        .ok_or("the datastore URI names no host")?;
    assert_eq!(host, UNREACHABLE_HOST);
    let config = compose.config()?;
    assert!(
        config["services"]["postgres"].is_null(),
        "a local database was declared"
    );

    // Readiness fails by the database's name at that address; no default
    // local address is substituted.
    let output = venue.lys(&[
        "identity",
        "health",
        "--config",
        &venue.config().to_string_lossy(),
    ])?;
    let stderr = text(&output.stderr);
    assert!(!output.status.success());
    assert!(stderr.contains("database_unreachable"), "{stderr}");
    assert!(
        stderr.contains(&format!("database at {UNREACHABLE_HOST}:5432")),
        "{stderr}"
    );
    for local in ["127.0.0.1:5432", "localhost:5432"] {
        assert!(
            !stderr.contains(local),
            "a local database address was dialled: {stderr}"
        );
    }

    // A secret missing from the environment compose reads is refused by
    // compose itself, by name.
    let env_file = venue.private_dir().join("identity.env");
    let rendered = std::fs::read_to_string(&env_file)?;
    let without: Vec<&str> = rendered
        .lines()
        .filter(|line| !line.starts_with("LYS_IDENTITY_RAUTHY_API_KEY_SECRET="))
        .collect();
    std::fs::write(&env_file, without.join("\n") + "\n")?;
    let output = compose.run(&["config", "--format", "json"])?;
    let stderr = text(&output.stderr);
    assert!(
        !output.status.success(),
        "compose accepted a missing secret"
    );
    assert!(
        stderr.contains("missing_secret LYS_IDENTITY_RAUTHY_API_KEY_SECRET"),
        "{stderr}"
    );
    Ok(())
}

/// On a running deployment: a missing credential is refused and never
/// regenerated, and an unavailable database is named while the identity
/// the deployment holds is the one it comes back with.
#[test]
fn id001_deploy_refusal_running_deployment() -> Result<(), Failure> {
    let venue = Venue::fresh("refusal-live", None)?;
    let stack = Stack::up(&venue)?;
    let before = stack.wait_ready()?;
    stack.configure()?;

    let secret = venue.private_dir().join("secrets/rauthy_api_key_secret");
    let kept = std::fs::read(&secret)?;
    std::fs::remove_file(&secret)?;
    let mut refused = 0;
    for subcommand in ["prepare", "configure"] {
        let output = stack.identity(subcommand)?;
        let stderr = text(&output.stderr);
        assert!(
            !output.status.success(),
            "{subcommand} ran without its secret"
        );
        assert!(
            stderr.contains("secret_missing: rauthy_api_key_secret"),
            "{subcommand}: {stderr}"
        );
        assert!(
            !secret.exists(),
            "{subcommand} generated a substitute secret"
        );
        refused += 1;
    }
    assert_eq!(refused, 2);
    restore_private(&secret, &kept)?;

    stack.compose().ok(&["stop", "postgres"])?;
    let output = stack.health()?;
    let stderr = text(&output.stderr);
    assert!(!output.status.success());
    assert!(stderr.contains("database_unreachable"), "{stderr}");
    stack.compose().ok(&["start", "postgres"])?;
    let after = stack.wait_ready()?;
    assert_eq!(after["issuer"], before["issuer"], "the issuer changed");
    assert_eq!(
        after["signing_keys"], before["signing_keys"],
        "a substitute identity appeared"
    );
    let report = stack.configure()?;
    let operations = report["operations"].as_array().ok_or("no operations")?;
    assert!(
        operations.iter().all(|op| op["outcome"] == "unchanged"),
        "{report}"
    );
    assert_eq!(operations.len(), 4);
    Ok(())
}

/// Puts a credential back as the owner-only file prepare wrote.
fn restore_private(path: &std::path::Path, bytes: &[u8]) -> Result<(), Failure> {
    use std::os::unix::fs::PermissionsExt;

    std::fs::write(path, bytes)?;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    Ok(())
}
