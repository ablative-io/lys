//! ID001_DEPLOY_REFUSAL: a missing secret, an invalid issuer or redirect, and
//! an unavailable database each produce a named failure, and none of them is
//! answered with a substitute identity or a development database. The
//! database address is configuration (ADR-005): with it set to an
//! unreachable 192.0.2.10, nothing local stands in for it.
//!
//! Container-backed: declared `test = false`, run only on the identity leg of
//! `.land/gates.sh`.

pub mod identity_support;

use identity_support::compose::{self, Compose};
use identity_support::fixtures::{Fixture, output_text};
use identity_support::{TestResult, server};
use serde_json::Value;

/// The host part of a `postgres://user:password@host:port/db` URI.
fn uri_host(uri: &str) -> TestResult<&str> {
    let after = uri.split_once('@').ok_or("the URI has no authority")?.1;
    Ok(after.split([':', '/']).next().unwrap_or_default())
}

struct Teardown<'a>(&'a Compose);

impl Drop for Teardown<'_> {
    fn drop(&mut self) {
        if let Err(error) = self.0.down() {
            eprintln!("warning: removing compose project {} failed: {error}", self.0.project);
        }
    }
}

#[test]
fn an_unreachable_external_database_is_named_and_never_replaced() -> TestResult {
    let fixture = Fixture::new("remote", "192.0.2.10")?;
    let rendered = compose::render(&fixture)?;
    let _teardown = Teardown(&rendered);

    // The resolved compose configuration names 192.0.2.10 for both services
    // and does not include the local database service at all.
    let config: Value = rendered.config()?;
    let services = config["services"].as_object().ok_or("no services")?;
    assert!(!services.contains_key("postgres"), "the local database would start");
    assert_eq!(config["services"]["rauthy"]["environment"]["PG_HOST"], "192.0.2.10");
    let uri = config["services"]["spicedb"]["environment"]["SPICEDB_DATASTORE_CONN_URI"]
        .as_str()
        .ok_or("SpiceDB has no datastore URI")?;
    assert_eq!(uri_host(uri)?, "192.0.2.10");
    let uri = config["services"]["spicedb-migrate"]["environment"]["SPICEDB_DATASTORE_CONN_URI"]
        .as_str()
        .ok_or("the migration has no datastore URI")?;
    assert_eq!(uri_host(uri)?, "192.0.2.10");

    // Bringing it up fails at SpiceDB's migration, so Rauthy never starts.
    let up = rendered.run(&["up", "-d"])?;
    assert!(!up.status.success(), "the stack started without its database");
    assert!(!server::rauthy_healthy(&fixture.rauthy()), "an identity answered without its database");

    // Readiness names the database at its configured address, and only there.
    let health = fixture.identity("health", &["--timeout-secs", "2"])?;
    let text = output_text(&health);
    assert!(!health.status.success(), "{text}");
    assert!(text.contains("service database: unready at 192.0.2.10:5432 (database_unreachable"), "{text}");
    assert!(text.contains("services_unready: database (database_unreachable)"), "{text}");
    let local = format!("127.0.0.1:{}", fixture.ports.postgres);
    assert!(!text.contains(&local), "a local address was substituted: {text}");
    let running = rendered.run_ok(&["ps", "--services", "--status", "running"])?;
    assert!(!running.lines().any(|service| service == "postgres"), "{running}");
    Ok(())
}

#[test]
fn a_missing_secret_is_named_before_anything_starts() -> TestResult {
    let fixture = Fixture::new("secret", "postgres")?;
    let rendered = compose::render(&fixture)?;
    let _teardown = Teardown(&rendered);

    // Compose refuses an environment that lacks a secret, by name.
    let env = std::fs::read_to_string(&rendered.env_file)?;
    let without: String = env
        .lines()
        .filter(|line| !line.starts_with("IDENTITY_RAUTHY_DB_PASSWORD="))
        .map(|line| format!("{line}\n"))
        .collect();
    assert_ne!(without, env);
    std::fs::write(&rendered.env_file, without)?;
    let refused = rendered.run(&["config", "--quiet"])?;
    let text = output_text(&refused);
    assert!(!refused.status.success(), "{text}");
    assert!(text.contains("identity_secret_missing - IDENTITY_RAUTHY_DB_PASSWORD"), "{text}");
    let up = rendered.run(&["up", "-d"])?;
    assert!(!up.status.success());
    assert!(!server::rauthy_healthy(&fixture.rauthy()));

    // configure refuses a missing API key by name, before any request.
    std::fs::remove_file(fixture.state_dir().join("credentials/rauthy_api_key"))?;
    let themes = identity_support::fixtures::repo_root().join("deploy/identity/rauthy-themes.json");
    let configure = fixture.identity("configure", &["--themes", &themes.to_string_lossy()])?;
    let text = output_text(&configure);
    assert!(!configure.status.success(), "{text}");
    assert!(text.contains("secret_missing: rauthy_api_key"), "{text}");
    Ok(())
}

#[test]
fn an_invalid_issuer_or_redirect_is_named_and_writes_nothing() -> TestResult {
    let cases = [
        (
            "public_origin = \"http://localhost:",
            "public_origin = \"http://identity.example.test:",
            "invalid_issuer",
        ),
        (
            "redirect_uris = [\"http://localhost:3000/auth/callback\"]",
            "redirect_uris = [\"http://localhost:3000/auth/callback#x\"]",
            "invalid_redirect_uri",
        ),
        (
            "redirect_uris = [\"http://localhost:4000/auth/callback\"]",
            "redirect_uris = [\"http://cambium.example.test/auth/callback\"]",
            "invalid_redirect_uri",
        ),
    ];
    let mut refused = 0;
    for (from, to, name) in cases {
        let fixture = Fixture::new("invalid", "postgres")?;
        fixture.edit_config(from, to)?;
        let prepared = fixture.identity("prepare", &[])?;
        let text = output_text(&prepared);
        assert!(!prepared.status.success(), "{text}");
        assert!(text.contains(&format!("error: {name}:")), "{name}: {text}");
        assert!(!fixture.state_dir().exists(), "a refused configuration wrote state");
        refused += 1;
    }
    assert_eq!(refused, cases.len());
    Ok(())
}
