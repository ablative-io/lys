//! OAuth grants: neither token prints, a refresh keeps the client and the
//! selected subject, the broker reseals a refreshed grant so the next call
//! opens the new token, and a revocation upstream is recorded as confirmed
//! or not.

use lys_core::Ed25519Identity;
use lys_secrets::{
    Admitted, AuditKind, Broker, BrokerPaths, Holder, LocalGrants, OAuthGrant, Presentation,
    Provenance, Secret, SecretRelation, new_operation_id,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn grant(expires_ms: i64) -> OAuthGrant {
    OAuthGrant::new(
        Provenance {
            token_endpoint: "https://provider.test/token".to_owned(),
            revocation_endpoint: Some("https://provider.test/revoke".to_owned()),
            client_id: "lys-client".to_owned(),
            provider_subject: "service-account-2".to_owned(),
            scopes: vec!["repo".to_owned()],
        },
        None,
        Secret::from_slice(b"access-one"),
        expires_ms,
        Secret::from_slice(b"refresh-one"),
    )
}

#[test]
fn neither_token_prints_and_a_refresh_keeps_the_client_and_subject() -> TestResult {
    let mut held = grant(1_000);
    let printed = format!("{held:?}");
    assert!(!printed.contains("access-one") && !printed.contains("refresh-one"));
    assert!(held.needs_refresh(1_000));
    assert!(!grant(1_000_000).needs_refresh(1_000));
    held.apply_refresh(
        br#"{"access_token":"access-two","expires_in":3600,"refresh_token":"refresh-two"}"#,
        1_000,
    )?;
    assert_eq!(held.access_token().expose(), b"access-two");
    assert!(!held.needs_refresh(2_000));
    assert_eq!(held.provenance().client_id, "lys-client");
    assert_eq!(held.provenance().provider_subject, "service-account-2");
    let form = held.refresh_form();
    assert!(
        form.iter()
            .any(|(name, value)| *name == "refresh_token" && value.expose() == b"refresh-two")
    );
    Ok(())
}

#[test]
fn a_refreshed_grant_is_resealed_and_revocation_is_recorded() -> TestResult {
    let root = tempfile::tempdir()?;
    let keys = root.path().join("keys");
    std::fs::create_dir_all(&keys)?;
    let paths = BrokerPaths {
        store_dir: root.path().join("store"),
        log_dir: root.path().join("log"),
        store_key: keys.join("store.key"),
        audit_key: keys.join("audit.key"),
        anchor: keys.join("audit.anchor"),
    };
    let grants = LocalGrants::new();
    grants.grant(SecretRelation {
        identity: "agent:noor".to_owned(),
        secret: "github".to_owned(),
        granted_by: Some("person:tom".to_owned()),
    });
    let mut broker = Broker::create(&paths, grants, Box::new(|| 1_000))?;
    broker.seal_oauth("github", "person:tom", &grant(1_000))?;
    let agent = Ed25519Identity::load_or_generate(&keys.join("agent.key"))?;
    let holder = Holder {
        identity: "agent:noor".to_owned(),
        key: agent.public_key_bytes(),
    };
    let issued = broker.issue(&holder, "github", 5, 60_000)?;
    let sign = || Presentation::sign(&issued.id, &new_operation_id()?, 1_000, [5; 32], &agent);
    let Admitted::Fresh(ticket) = broker.admit_use(&issued.token, &sign()?, 0)? else {
        return Err("retried".into());
    };
    let mut opened = ticket.oauth()?.ok_or("not an OAuth grant")?;
    opened.apply_refresh(br#"{"access_token":"access-two","expires_in":3600}"#, 1_000)?;
    broker.refreshed(&ticket, &opened)?;
    broker.settle(ticket, 0)?;
    let Admitted::Fresh(next) = broker.admit_use(&issued.token, &sign()?, 0)? else {
        return Err("retried".into());
    };
    let reopened = next.oauth()?.ok_or("not an OAuth grant")?;
    assert_eq!(reopened.access_token().expose(), b"access-two");
    broker.settle(next, 0)?;
    broker.drop_handle(&issued.id)?;
    assert!(broker.oauth_grant_of(&issued.id)?.is_some());
    broker.record_upstream_revocation(&issued.id, false)?;
    let outcomes: Vec<String> = broker
        .audit()
        .replay()?
        .into_iter()
        .filter(|recorded| recorded.line.kind == AuditKind::Refresh)
        .map(|recorded| recorded.line.outcome)
        .collect();
    assert_eq!(outcomes, ["refreshed", "revocation_unconfirmed"]);
    Ok(())
}
