//! `ID001_DEPLOY`'s restart line: a restart, and a full stop and start of the
//! compose project, preserve the issuer's identity (its issuer URL and
//! signing keys) and the database's contents. Container-backed: runs only on
//! the identity leg of .land/gates.sh.

pub mod identity_support;

use std::collections::BTreeSet;

use identity_support::compose::{self, require_runtime};
use identity_support::fixtures::{Deployment, TestResult, succeeded};
use identity_support::server::{rauthy_json, request};

/// What must survive a restart: the issuer, its key ids, and every client.
#[derive(Debug, PartialEq)]
struct Identity {
    issuer: String,
    key_ids: BTreeSet<String>,
    clients: serde_json::Value,
}

fn observe(deployment: &Deployment) -> TestResult<Identity> {
    let address = deployment.rauthy_address();
    let (status, discovery) = request(
        &address,
        "GET",
        "/auth/v1/.well-known/openid-configuration",
        &[],
        None,
    )?;
    assert_eq!(status, 200, "{discovery}");
    let discovery: serde_json::Value = serde_json::from_str(&discovery)?;
    let issuer = discovery["issuer"].as_str().ok_or("no issuer")?.to_string();
    let (status, certs) = request(&address, "GET", "/auth/v1/oidc/certs", &[], None)?;
    assert_eq!(status, 200, "{certs}");
    let certs: serde_json::Value = serde_json::from_str(&certs)?;
    let key_ids = certs["keys"]
        .as_array()
        .ok_or("no keys")?
        .iter()
        .filter_map(|key| key["kid"].as_str().map(str::to_string))
        .collect();
    let clients = rauthy_json(deployment, "GET", "/auth/v1/clients")?;
    Ok(Identity {
        issuer,
        key_ids,
        clients,
    })
}

#[test]
fn id001_deploy_restart_preserves_issuer_identity_and_database_contents() -> TestResult {
    require_runtime()?;
    let deployment = Deployment::new("restart", |text| text)?;
    compose::render(&deployment)?;
    compose::up(&deployment, &[])?;
    compose::wait_ready(&deployment)?;
    succeeded(&deployment.lys("configure")?, "configure")?;
    let before = observe(&deployment)?;
    assert_eq!(
        before.issuer,
        format!("http://localhost:{}/auth/v1", deployment.ports.rauthy)
    );
    assert!(!before.key_ids.is_empty(), "the issuer has no signing keys");

    succeeded(
        &compose::compose(&deployment, &["restart"])?,
        "docker compose restart",
    )?;
    compose::wait_ready(&deployment)?;
    assert_eq!(
        observe(&deployment)?,
        before,
        "a restart changed the issuer or the database"
    );

    succeeded(
        &compose::compose(&deployment, &["down"])?,
        "docker compose down",
    )?;
    compose::up(&deployment, &[])?;
    compose::wait_ready(&deployment)?;
    assert_eq!(
        observe(&deployment)?,
        before,
        "a stop and start changed the issuer or the database"
    );
    Ok(())
}
