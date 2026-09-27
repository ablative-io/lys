#![cfg(test)]
//! `ID001_DEPLOY_REFUSAL`: a missing secret, an invalid issuer or redirect and
//! an unavailable database each produce a named failure, never a substitute
//! identity or development database; and a database on a network device is
//! configuration, carried into both services with no local address
//! substituted. Container-backed: runs only on the identity leg of
//! .land/gates.sh.

pub mod identity_support;

use identity_support::compose::{self, require_runtime};
use identity_support::fixtures::{Deployment, TestResult, output_text, succeeded};

const REMOTE_HOST: &str = "192.0.2.10";

fn remote_database(text: &str) -> String {
    text.lines()
        .map(|line| match line.split_once(" = ").map(|(key, _)| key) {
            Some("bundled") => "bundled = false".to_string(),
            Some("host") => format!("host = \"{REMOTE_HOST}\""),
            Some("check_address") => format!("check_address = \"{REMOTE_HOST}:5432\""),
            _ => line.to_string(),
        })
        .map(|line| line + "\n")
        .collect()
}

#[test]
fn id001_deploy_refusal_missing_secret_is_named() -> TestResult {
    require_runtime()?;
    let provided = Deployment::new("provided", |text| {
        text.replace("source = \"generate\"", "source = \"provided\"")
    })?;
    let output = provided.lys("prepare")?;
    let text = output_text(&output);
    assert!(!output.status.success());
    assert!(text.contains("secret_missing"), "{text}");
    assert!(!provided.state.join("compose.env").exists());

    let deployment = Deployment::new("missing", |text| text)?;
    compose::render(&deployment)?;
    let env_path = deployment.state.join("compose.env");
    let env = std::fs::read_to_string(&env_path)?;
    let mut without = env
        .lines()
        .filter(|line| !line.starts_with("RAUTHY_DB_PASSWORD="))
        .collect::<Vec<_>>()
        .join("\n");
    without.push('\n');
    std::fs::write(&env_path, without)?;
    let refused = compose::compose(&deployment, &["config", "--quiet"])?;
    let text = output_text(&refused);
    assert!(!refused.status.success());
    assert!(text.contains("secret_missing RAUTHY_DB_PASSWORD"), "{text}");
    std::fs::remove_file(deployment.state.join("rauthy-bootstrap-api-secret"))?;
    let configure = deployment.lys("configure")?;
    assert!(!configure.status.success());
    assert!(
        output_text(&configure).contains("secret_missing"),
        "{}",
        output_text(&configure)
    );
    Ok(())
}

#[test]
fn id001_deploy_refusal_invalid_issuer_and_redirect_are_named() -> TestResult {
    require_runtime()?;
    let issuer = Deployment::new("issuer", |text| {
        text.replace(
            "public_origin = \"http://localhost:",
            "public_origin = \"http://id.example.test:",
        )
    })?;
    let output = issuer.lys("prepare")?;
    assert!(!output.status.success());
    assert!(
        output_text(&output).contains("issuer_invalid"),
        "{}",
        output_text(&output)
    );
    assert!(
        !issuer.state.exists(),
        "a refused configuration wrote state"
    );

    let redirect = Deployment::new("redirect", |text| {
        text.replace(
            "http://localhost:8400/auth/oidc/callback",
            "https://cambium.example.test/callback#fragment",
        )
    })?;
    let output = redirect.lys("prepare")?;
    assert!(!output.status.success());
    assert!(
        output_text(&output).contains("redirect_uri_invalid"),
        "{}",
        output_text(&output)
    );
    assert!(
        !redirect.state.exists(),
        "a refused configuration wrote state"
    );
    Ok(())
}

#[test]
fn a_network_database_address_is_carried_into_both_services() -> TestResult {
    require_runtime()?;
    let deployment = Deployment::new("remote", |text| remote_database(&text))?;
    compose::render(&deployment)?;
    let model = compose::config_json(&deployment)?;
    let services = &model["services"];
    assert!(
        services.get("postgres").is_none(),
        "no local database is started"
    );
    assert_eq!(services["rauthy"]["environment"]["PG_HOST"], REMOTE_HOST);
    assert_eq!(services["rauthy"]["environment"]["HIQLITE"], "false");
    for service in ["spicedb", "spicedb-migrate"] {
        let uri = services[service]["environment"]["SPICEDB_DATASTORE_CONN_URI"]
            .as_str()
            .ok_or("no datastore URI")?;
        let host = uri
            .split_once('@')
            .and_then(|(_, rest)| rest.split_once(':'))
            .map(|(host, _)| host)
            .ok_or("no host in the datastore URI")?;
        assert_eq!(host, REMOTE_HOST);
    }
    Ok(())
}

#[test]
fn id001_deploy_refusal_unavailable_database_is_named() -> TestResult {
    require_runtime()?;
    let deployment = Deployment::new("unavailable", |text| remote_database(&text))?;
    compose::render(&deployment)?;
    compose::up(&deployment, &["rauthy"])?;
    std::thread::sleep(std::time::Duration::from_secs(20));
    let health = deployment.lys("health")?;
    let text = output_text(&health);
    assert!(!health.status.success(), "{text}");
    assert!(
        text.lines()
            .any(|line| line.starts_with("postgres unready: database_unreachable")),
        "{text}"
    );
    assert!(
        text.lines()
            .any(|line| line.starts_with("rauthy unready: ")),
        "Rauthy answered ready with no database: {text}"
    );
    let ps = compose::compose(&deployment, &["ps", "--all", "--format", "json"])?;
    succeeded(&ps, "docker compose ps")?;
    assert!(!String::from_utf8_lossy(&ps.stdout).contains("\"Service\":\"postgres\""));
    Ok(())
}
