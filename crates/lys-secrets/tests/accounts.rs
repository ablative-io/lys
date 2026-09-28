//! A holder moves its secret to the next account with its own handle: the
//! ask is checked as a use is, counts no use, and a refused ask moves
//! nothing.

use lys_core::Ed25519Identity;
use lys_secrets::{
    Broker, BrokerPaths, Holder, LocalGrants, Presentation, Secret, SecretRelation, SecretsError,
    new_operation_id,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn refusal<T>(result: Result<T, SecretsError>) -> &'static str {
    match result {
        Ok(_) => "admitted",
        Err(error) => error.name(),
    }
}

#[test]
fn the_holder_moves_to_the_next_account_and_a_thief_or_a_dropped_handle_cannot() -> TestResult {
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
        secret: "token".to_owned(),
        granted_by: Some("person:tom".to_owned()),
    });
    let mut broker = Broker::create(&paths, grants, Box::new(|| 1_000))?;
    broker.seal("token", "person:tom", &Secret::from_slice(b"first"))?;
    broker.add_account("token", "second", &Secret::from_slice(b"second"))?;
    let agent = Ed25519Identity::load_or_generate(&keys.join("agent.key"))?;
    let thief = Ed25519Identity::load_or_generate(&keys.join("thief.key"))?;
    let holder = Holder {
        identity: "agent:noor".to_owned(),
        key: agent.public_key_bytes(),
    };
    let issued = broker.issue(&holder, "token", 1, 60_000)?;
    let sign = |key: &Ed25519Identity| {
        Presentation::sign(&issued.id, &new_operation_id()?, 1_000, [3; 32], key)
    };
    assert_eq!(
        refusal(broker.next_account_for(&issued.token, &sign(&thief)?)),
        "PresentationInvalid"
    );
    assert_eq!(
        broker.next_account_for(&issued.token, &sign(&agent)?)?,
        "second"
    );
    let used = broker.use_handle(&issued.token, &sign(&agent)?, |value| {
        value.expose().to_vec()
    })?;
    assert!(matches!(
        used,
        lys_secrets::Used::Forwarded { ref answer, .. } if answer == b"second"
    ));
    broker.drop_handle(&issued.id)?;
    assert_eq!(
        refusal(broker.next_account_for(&issued.token, &sign(&agent)?)),
        "HandleDropped"
    );
    Ok(())
}

#[test]
fn each_spawn_takes_the_next_login_and_the_log_names_the_seat() -> TestResult {
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
    for seat in ["seat:one", "seat:two", "seat:three"] {
        grants.grant(SecretRelation {
            identity: seat.to_owned(),
            secret: "logins".to_owned(),
            granted_by: Some("person:tom".to_owned()),
        });
    }
    let mut broker = Broker::create(&paths, grants, Box::new(|| 1_000))?;
    broker.seal("logins", "person:tom", &Secret::from_slice(b"login-a"))?;
    broker.add_account("logins", "b", &Secret::from_slice(b"login-b"))?;
    let (first, one) = broker.spawn_login("seat:one", "logins")?;
    let (second, two) = broker.spawn_login("seat:two", "logins")?;
    let (third, _three) = broker.spawn_login("seat:three", "logins")?;
    assert_eq!(
        (first.as_str(), second.as_str(), third.as_str()),
        ("primary", "b", "primary")
    );
    assert_ne!(one.expose(), two.expose());
    assert_eq!(
        refusal(broker.spawn_login("seat:stranger", "logins")),
        "PermissionDenied"
    );
    let handed: Vec<(Option<String>, Option<String>)> = broker
        .audit()
        .audit_every_line()?
        .into_iter()
        .filter(|recorded| {
            recorded.line.kind == lys_secrets::AuditKind::SpawnLogin
                && recorded.line.outcome == "handed"
        })
        .map(|recorded| (recorded.line.identity, recorded.line.secret))
        .collect();
    assert_eq!(handed.len(), 3);
    assert_eq!(
        handed.first(),
        Some(&(
            Some("seat:one".to_owned()),
            Some("logins@primary".to_owned())
        ))
    );
    Ok(())
}
