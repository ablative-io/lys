//! OAuth grants: neither token prints, a refresh keeps the client and the
//! selected subject, the broker reseals a refreshed grant so the next call
//! opens the new token, a refresh naming a new client is refused until the
//! grant is reconnected by name, and a revocation upstream is recorded as
//! confirmed or not.

use std::path::Path;

use lys_core::Ed25519Identity;
use lys_secrets::{
    Admitted, AuditKind, Broker, BrokerPaths, Holder, LocalGrants, OAuthGrant, Presentation,
    Provenance, RevocationState, Secret, SecretRelation, SecretsError, UpstreamRevocation,
    new_operation_id,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn grant(expires_ms: i64) -> OAuthGrant {
    grant_for("lys-client", expires_ms)
}

fn grant_for(client: &str, expires_ms: i64) -> OAuthGrant {
    OAuthGrant::new(
        Provenance {
            token_endpoint: "https://provider.test/token".to_owned(),
            revocation_endpoint: Some("https://provider.test/revoke".to_owned()),
            client_id: client.to_owned(),
            provider_subject: "service-account-2".to_owned(),
            scopes: vec!["repo".to_owned()],
        },
        None,
        Secret::from_slice(b"access-one"),
        expires_ms,
        Secret::from_slice(b"refresh-one"),
    )
}

/// A broker under `root` at clock 1000 with `agent:noor` granted `github`,
/// and the agent's key.
fn broker_at(root: &Path) -> Result<(Broker<LocalGrants>, Ed25519Identity), SecretsError> {
    let keys = root.join("keys");
    std::fs::create_dir_all(&keys).map_err(|source| SecretsError::Io {
        context: "creating the keys directory".to_owned(),
        source,
    })?;
    let paths = BrokerPaths {
        store_dir: root.join("store"),
        log_dir: root.join("log"),
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
    let broker = Broker::create(&paths, grants, Box::new(|| 1_000))?;
    let agent = Ed25519Identity::load_or_generate(&keys.join("agent.key"))?;
    Ok((broker, agent))
}

fn refusal<T>(result: Result<T, SecretsError>) -> String {
    match result {
        Ok(_) => "admitted".to_owned(),
        Err(error) => error.to_string(),
    }
}

#[test]
fn sec3_oauth_refusals_reconnect_required() -> TestResult {
    let root = tempfile::tempdir()?;
    let (mut broker, agent) = broker_at(root.path())?;
    broker.seal_oauth("github", "person:tom", &grant(1_000))?;
    let holder = Holder {
        identity: "agent:noor".to_owned(),
        key: agent.public_key_bytes(),
    };
    let issued = broker.issue(&holder, "github", 5, 60_000)?;
    let sign = || Presentation::sign(&issued.id, &new_operation_id()?, 1_000, [5; 32], &agent);
    let fresh = |admitted: Admitted| match admitted {
        Admitted::Fresh(ticket) => Ok(ticket),
        Admitted::Retried { outcome } => Err(format!("retried: {outcome}")),
    };

    let ticket = fresh(broker.admit_use(&issued.token, &sign()?, 0)?)?;
    let mut new_client = grant_for("lys-client-2", 1_000);
    new_client.apply_refresh(br#"{"access_token":"access-two","expires_in":3600}"#, 1_000)?;
    let refused = refusal(broker.refreshed(&ticket, &new_client));
    assert!(refused.starts_with("ReconnectRequired: "), "{refused}");
    assert!(
        refused.contains("(act: reconnect the grant by name for the new client)"),
        "{refused}"
    );
    broker.settle(ticket, 0)?;
    let held = fresh(broker.admit_use(&issued.token, &sign()?, 0)?)?;
    let opened = held.oauth()?.ok_or("not an OAuth grant")?;
    assert_eq!(opened.access_token().expose(), b"access-one");
    assert_eq!(opened.provenance().client_id, "lys-client");
    broker.settle(held, 0)?;

    assert!(
        refusal(broker.reconnect_oauth("github", "person:dana", &new_client))
            .starts_with("LendingNotPermitted: ")
    );
    broker.reconnect_oauth("github", "person:tom", &new_client)?;
    let reconnected = fresh(broker.admit_use(&issued.token, &sign()?, 0)?)?;
    let mut refreshed = reconnected.oauth()?.ok_or("not an OAuth grant")?;
    assert_eq!(refreshed.provenance().client_id, "lys-client-2");
    refreshed.apply_refresh(
        br#"{"access_token":"access-three","expires_in":3600}"#,
        1_000,
    )?;
    broker.refreshed(&reconnected, &refreshed)?;
    broker.settle(reconnected, 0)?;
    let outcomes: Vec<String> = broker
        .audit()
        .replay()?
        .into_iter()
        .filter(|recorded| recorded.line.kind == AuditKind::Refresh)
        .map(|recorded| recorded.line.outcome)
        .collect();
    assert_eq!(
        outcomes,
        [
            "ReconnectRequired",
            "reconnected to client lys-client-2",
            "refreshed"
        ]
    );
    Ok(())
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
    assert_eq!(
        broker
            .record_upstream_revocation(&issued.id, Ok(()))
            .map_err(|error| error.name()),
        Err("RevocationBeforeDrop")
    );
    broker.drop_handle(&issued.id)?;
    assert!(broker.oauth_grant_of(&issued.id)?.is_some());
    assert_eq!(
        broker.revocation_state(&issued.id)?,
        RevocationState {
            stopped_here: true,
            upstream: UpstreamRevocation::NotAsked,
        }
    );
    broker.record_upstream_revocation(&issued.id, Err("the provider answered 503".to_owned()))?;
    assert_eq!(
        broker.revocation_state(&issued.id)?,
        RevocationState {
            stopped_here: true,
            upstream: UpstreamRevocation::Unconfirmed("the provider answered 503".to_owned()),
        }
    );
    assert_eq!(
        broker
            .confirm_upstream_revocation(&issued.id, "someone-else")
            .map_err(|error| error.name()),
        Err("ProviderMismatch")
    );
    assert!(broker.confirm_upstream_revocation(&issued.id, "service-account-2")?);
    assert_eq!(
        broker.revocation_state(&issued.id)?.upstream,
        UpstreamRevocation::Confirmed
    );
    assert!(!broker.confirm_upstream_revocation(&issued.id, "service-account-2")?);
    let outcomes: Vec<String> = broker
        .audit()
        .replay()?
        .into_iter()
        .filter(|recorded| recorded.line.kind == AuditKind::Refresh)
        .map(|recorded| recorded.line.outcome)
        .collect();
    assert_eq!(
        outcomes,
        [
            "refreshed",
            "revocation_unconfirmed: the provider answered 503",
            "revoked_upstream"
        ]
    );
    Ok(())
}
