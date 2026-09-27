//! ID001_DEPLOY, restart half: removing and recreating every container keeps
//! the issuer identity (issuer, signing keys) and the database contents
//! (clients, their kept secrets), and configure then finds nothing to do.
//!
//! Container-backed: declared `test = false`, run only on the identity leg of
//! `.land/gates.sh`.

pub mod identity_support;

use identity_support::{Stack, TestResult, server};
use serde_json::Value;

/// What must survive a restart.
#[derive(Debug, PartialEq)]
struct Identity {
    issuer: String,
    signing_keys: Vec<String>,
    clients: Vec<String>,
}

fn observe(stack: &Stack) -> TestResult<Identity> {
    let address = stack.fixture.rauthy();
    let discovery: Value = serde_json::from_str(
        &server::request(&address, "GET", "/auth/v1/.well-known/openid-configuration", &[], None)?.body,
    )?;
    let jwks: Value = serde_json::from_str(&server::request(&address, "GET", "/auth/v1/oidc/certs", &[], None)?.body)?;
    let mut signing_keys: Vec<String> = jwks["keys"]
        .as_array()
        .ok_or("the JWKS has no keys")?
        .iter()
        .map(|key| format!("{}:{}", key["kid"], key["alg"]))
        .collect();
    signing_keys.sort();
    let listed: Vec<Value> = serde_json::from_str(&stack.rauthy_admin("GET", "/auth/v1/clients")?.body)?;
    let mut clients: Vec<String> = listed.iter().map(Value::to_string).collect();
    clients.sort();
    Ok(Identity {
        issuer: discovery["issuer"].as_str().ok_or("no issuer")?.to_string(),
        signing_keys,
        clients,
    })
}

#[test]
fn restart_preserves_issuer_identity_and_database_contents() -> TestResult {
    let stack = Stack::up("restart")?;
    stack.configure()?;
    let before = observe(&stack)?;
    assert_eq!(before.issuer, format!("http://localhost:{}/auth/v1/", stack.fixture.ports.rauthy));
    assert!(!before.signing_keys.is_empty());
    assert_eq!(before.clients.len(), 3, "{:?}", before.clients);
    let secrets_before = stack.fixture.secrets()?;

    stack.restart()?;

    let after = observe(&stack)?;
    assert_eq!(after, before, "the restart changed the issuer identity or the database");
    let second = stack.configure()?;
    assert!(!second.contains("created") && !second.contains("updated"), "{second}");
    let mut secrets_after = stack.fixture.secrets()?;
    let mut secrets_expected = secrets_before;
    secrets_after.sort();
    secrets_expected.sort();
    assert_eq!(secrets_after, secrets_expected, "a credential was regenerated");
    Ok(())
}
