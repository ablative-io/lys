#![cfg(test)]
//! A configuration's `SpiceDB` endpoint is a socket address or the
//! configuration is refused at load, so no `SpiceDB` call waits on a name
//! lookup.

use std::error::Error;

use serde_json::{Value, json};

use super::Config;
use crate::error::ServerError;

/// A configuration that validates, its `SpiceDB` endpoint `endpoint`.
fn configured(endpoint: &str) -> Value {
    let issuer = "https://issuer.example.test";
    json!({
        "listen": "127.0.0.1:0",
        "log_dir": "/lys-config-test/log",
        "log_origin": "lys-config-test",
        "event_key_file": "/lys-config-test/service.key",
        "issuer": issuer,
        "client_id": "lys-config-test",
        "client_secret_file": "/lys-config-test/client.secret",
        "redirect_url": "http://127.0.0.1:0/callback",
        "administrator": {"issuer": issuer, "subject": "administrator"},
        "link_audit_source": {"issuer": issuer, "subject": "link-audit"},
        "session_seconds": 600,
        "secure_cookie": false,
        "grant_log_dir": "/lys-config-test/grant-log",
        "grant_log_origin": "lys-config-test-grants",
        "grant_model_file": "/lys-config-test/grant-model.json",
        "spicedb": {"endpoint": endpoint, "key_file": "/lys-config-test/spicedb.key"},
    })
}

fn load(endpoint: &str) -> Result<Result<Config, ServerError>, Box<dyn Error>> {
    let dir = tempfile::TempDir::new()?;
    let path = dir.path().join("config.json");
    std::fs::write(&path, configured(endpoint).to_string())?;
    Ok(Config::load(&path))
}

#[test]
fn an_endpoint_naming_a_host_is_refused_at_load() -> Result<(), Box<dyn Error>> {
    let reason = match load("localhost:58443")? {
        Err(ServerError::ConfigInvalid { reason }) => reason,
        other => return Err(format!("a host name endpoint was not refused: {other:?}").into()),
    };
    assert!(
        reason.contains("the SpiceDB endpoint must be an address, so no name lookup is waited on"),
        "{reason}"
    );
    assert!(reason.contains("localhost:58443"), "{reason}");
    Ok(())
}

#[test]
fn an_endpoint_that_is_an_address_loads() -> Result<(), Box<dyn Error>> {
    let mut loaded = 0;
    for endpoint in ["127.0.0.1:58443", "[::1]:58443"] {
        let config = load(endpoint)??;
        let settings = config.spicedb.ok_or("the loaded configuration lost its SpiceDB")?;
        assert_eq!(settings.endpoint, endpoint);
        loaded += 1;
    }
    assert_eq!(loaded, 2);
    Ok(())
}
