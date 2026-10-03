#![cfg(test)]

//! Derived handles: only an owner or a holder of the lend relation lends,
//! the recipient must itself be permitted, a derived handle stays inside
//! its ancestry's uses, window and spend, a drop above ends it, and a
//! handle issued on its own is untouched.

use lys_core::Ed25519Identity;
use lys_secrets::{
    Broker, BrokerPaths, Holder, IssuedHandle, LocalGrants, Presentation, Relation, Secret,
    SecretRelation, SecretsError, new_operation_id,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn refusal<T>(result: Result<T, SecretsError>) -> &'static str {
    match result {
        Ok(_) => "admitted",
        Err(error) => error.name(),
    }
}

fn relation(identity: &str) -> SecretRelation {
    SecretRelation {
        identity: identity.to_owned(),
        secret: "token".to_owned(),
        granted_by: Some("person:dana".to_owned()),
    }
}

struct Party {
    key: Ed25519Identity,
    holder: Holder,
}

fn party(dir: &std::path::Path, identity: &str) -> Result<Party, Box<dyn std::error::Error>> {
    let key = Ed25519Identity::load_or_generate(&dir.join(format!("{identity}.key")))?;
    let holder = Holder {
        identity: identity.to_owned(),
        key: key.public_key_bytes(),
    };
    Ok(Party { key, holder })
}

fn sign(issued: &IssuedHandle, party: &Party) -> Result<Presentation, SecretsError> {
    Presentation::sign(&issued.id, &new_operation_id()?, 1_000, [1; 32], &party.key)
}

#[test]
fn lending_needs_ownership_or_the_lend_relation_and_stays_inside_its_ancestry() -> TestResult {
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
    for identity in [
        "person:dana",
        "person:tom",
        "agent:dana-bot",
        "agent:tom-bot",
    ] {
        grants
            .grant(relation(identity))
            .expect("local grants lock must be healthy");
    }
    let mut broker = Broker::create(&paths, grants, Box::new(|| 1_000))?;
    broker.seal("token", "person:dana", &Secret::from_slice(b"value"))?;
    let dana = party(&keys, "person:dana")?;
    let tom = party(&keys, "person:tom")?;
    let dana_bot = party(&keys, "agent:dana-bot")?;
    let tom_bot = party(&keys, "agent:tom-bot")?;
    let dana_handle = broker.issue(&dana.holder, "token", 3, 50_000)?;
    let tom_handle = broker.issue(&tom.holder, "token", 3, 50_000)?;
    assert_eq!(
        refusal(broker.derive(
            &tom_handle.token,
            &sign(&tom_handle, &tom)?,
            &tom_bot.holder,
            (1, 40_000),
            None
        )),
        "LendingNotPermitted"
    );
    broker
        .permissions()
        .grant_as(Relation::Lend, relation("person:tom"))
        .expect("local grants lock must be healthy");
    let lent = broker.derive(
        &tom_handle.token,
        &sign(&tom_handle, &tom)?,
        &tom_bot.holder,
        (1, 40_000),
        None,
    )?;
    assert_eq!(
        refusal(broker.derive(
            &dana_handle.token,
            &sign(&dana_handle, &dana)?,
            &dana_bot.holder,
            (4, 40_000),
            None
        )),
        "BeyondAncestry"
    );
    let derived = broker.derive(
        &dana_handle.token,
        &sign(&dana_handle, &dana)?,
        &dana_bot.holder,
        (2, 40_000),
        None,
    )?;
    broker.use_handle(&derived.token, &sign(&derived, &dana_bot)?, Secret::len)?;
    broker.use_handle(&derived.token, &sign(&derived, &dana_bot)?, Secret::len)?;
    broker.use_handle(&dana_handle.token, &sign(&dana_handle, &dana)?, Secret::len)?;
    assert_eq!(
        refusal(broker.use_handle(&dana_handle.token, &sign(&dana_handle, &dana)?, Secret::len)),
        "LeaseExhausted"
    );
    broker.drop_handle(&tom_handle.id)?;
    assert_eq!(
        refusal(broker.use_handle(&lent.token, &sign(&lent, &tom_bot)?, Secret::len)),
        "HandleDropped"
    );
    let sibling = broker.issue(&tom_bot.holder, "token", 1, 50_000)?;
    broker.use_handle(&sibling.token, &sign(&sibling, &tom_bot)?, Secret::len)?;
    Ok(())
}

#[test]
fn a_people_only_secret_is_never_handed_to_an_agent() -> TestResult {
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
    for identity in ["person:dana", "agent:dana-bot"] {
        grants
            .grant(relation(identity))
            .expect("local grants lock must be healthy");
    }
    let mut broker = Broker::create(&paths, grants, Box::new(|| 1_000))?;
    broker.seal("token", "person:dana", &Secret::from_slice(b"value"))?;
    let dana = party(&keys, "person:dana")?;
    let bot = party(&keys, "agent:dana-bot")?;
    assert_eq!(
        refusal(broker.set_recipients("person:tom", "token", lys_secrets::Recipients::PeopleOnly)),
        "LendingNotPermitted"
    );
    broker.set_recipients("person:dana", "token", lys_secrets::Recipients::PeopleOnly)?;
    assert_eq!(
        refusal(broker.issue(&bot.holder, "token", 1, 50_000)),
        "RecipientRefused"
    );
    let own = broker.issue(&dana.holder, "token", 3, 50_000)?;
    assert_eq!(
        refusal(broker.derive(
            &own.token,
            &sign(&own, &dana)?,
            &bot.holder,
            (1, 40_000),
            None
        )),
        "RecipientRefused"
    );
    broker.set_recipients("person:dana", "token", lys_secrets::Recipients::Anyone)?;
    broker.derive(
        &own.token,
        &sign(&own, &dana)?,
        &bot.holder,
        (1, 40_000),
        None,
    )?;
    Ok(())
}

#[test]
fn a_line_of_derived_handles_two_hundred_deep_is_lent_and_used_whole() -> TestResult {
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
    grants
        .grant(relation("person:dana"))
        .expect("local grants lock must be healthy");
    let mut broker = Broker::create(&paths, grants, Box::new(|| 1_000))?;
    broker.seal("token", "person:dana", &Secret::from_slice(b"value"))?;
    let dana = party(&keys, "person:dana")?;
    let mut at = broker.issue(&dana.holder, "token", 20, 50_000)?;
    let mut derived = 0;
    for _ in 1..200 {
        at = broker.derive(
            &at.token,
            &sign(&at, &dana)?,
            &dana.holder,
            (20, 40_000),
            None,
        )?;
        derived += 1;
    }
    assert_eq!(derived, 199, "the old depth of 16 is long passed");
    at = broker.derive(
        &at.token,
        &sign(&at, &dana)?,
        &dana.holder,
        (1, 40_000),
        None,
    )?;
    broker.use_handle(&at.token, &sign(&at, &dana)?, Secret::len)?;
    Ok(())
}
