#![cfg(test)]

//! One proxied call moves the audit log's anchor once, at its settlement
//! line, where it once moved it at every line the call wrote (SECRETS-005
//! R3). The anchor file itself is read back: it names the whole log after
//! the call, so the one write left nothing behind it.

use std::path::Path;

use lys_core::Ed25519Identity;
use lys_secrets::{
    Admitted, Broker, BrokerPaths, Holder, LocalGrants, Presentation, Secret, SecretRelation,
    new_operation_id,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const NOW: i64 = 1_800_000_000_000;

fn paths(dir: &Path) -> BrokerPaths {
    let keys = dir.join("keys");
    BrokerPaths {
        store_dir: dir.join("store"),
        log_dir: dir.join("log"),
        store_key: keys.join("store.key"),
        audit_key: keys.join("audit.key"),
        anchor: keys.join("audit.anchor"),
    }
}

fn granted() -> LocalGrants {
    let grants = LocalGrants::new();
    grants
        .grant(SecretRelation {
            identity: "agent:noor".to_owned(),
            secret: "token".to_owned(),
            granted_by: Some("person:tom".to_owned()),
        })
        .expect("local grants lock must be healthy");
    grants
}

/// The length the anchor file names.
fn anchored(paths: &BrokerPaths) -> Result<u64, Box<dyn std::error::Error>> {
    let anchor: serde_json::Value = serde_json::from_slice(&std::fs::read(&paths.anchor)?)?;
    anchor["len"]
        .as_u64()
        .ok_or_else(|| "the anchor names no length".into())
}

#[test]
fn one_proxied_call_writes_the_anchor_once() -> TestResult {
    let dir = tempfile::tempdir()?;
    std::fs::create_dir_all(dir.path().join("keys"))?;
    let paths = paths(dir.path());
    let agent = Ed25519Identity::load_or_generate(&dir.path().join("agent.key"))?;
    let holder = Holder {
        identity: "agent:noor".to_owned(),
        key: agent.public_key_bytes(),
    };
    let mut broker = Broker::create(&paths, granted(), Box::new(|| NOW))?;
    broker.seal("token", "person:tom", &Secret::from_slice(b"value-one"))?;
    let issued = broker.issue(&holder, "token", 5, NOW + 60_000)?;

    let (lines, writes) = (broker.audit().len(), broker.audit().anchor_writes());
    let presented = Presentation::sign(&issued.id, &new_operation_id()?, NOW, [7; 32], &agent)?;
    let Admitted::Fresh(ticket) = broker.admit_use(&issued.token, &presented, 0)? else {
        return Err("the call was not admitted".into());
    };
    let ticket = broker.at_forward_boundary(ticket)?;
    broker.settle(ticket, 0)?;
    assert_eq!(
        broker.audit().len() - lines,
        2,
        "the call wrote its use line and its settlement line"
    );
    assert_eq!(
        broker.audit().anchor_writes() - writes,
        1,
        "one proxied call writes the anchor once, where it was written once per audit line"
    );
    assert_eq!(anchored(&paths)?, broker.audit().len());

    let (lines, writes) = (broker.audit().len(), broker.audit().anchor_writes());
    let stranger = Ed25519Identity::load_or_generate(&dir.path().join("stranger.key"))?;
    let forged = Presentation::sign(&issued.id, &new_operation_id()?, NOW, [7; 32], &stranger)?;
    let refused = broker.admit_use(&issued.token, &forged, 0).err();
    let refused = refused.map(|error| error.name());
    assert_eq!(refused, Some("PresentationInvalid"));
    assert_eq!(broker.audit().len() - lines, 1);
    assert_eq!(broker.audit().anchor_writes() - writes, 1);
    assert_eq!(anchored(&paths)?, broker.audit().len());

    let size = broker.audit().len();
    drop(broker);
    let reopened = Broker::open(&paths, granted(), Box::new(|| NOW))?;
    assert_eq!(reopened.audit().len(), size);
    Ok(())
}

#[test]
fn a_call_admitted_and_never_settled_still_opens() -> TestResult {
    let dir = tempfile::tempdir()?;
    std::fs::create_dir_all(dir.path().join("keys"))?;
    let paths = paths(dir.path());
    let agent = Ed25519Identity::load_or_generate(&dir.path().join("agent.key"))?;
    let holder = Holder {
        identity: "agent:noor".to_owned(),
        key: agent.public_key_bytes(),
    };
    let mut broker = Broker::create(&paths, granted(), Box::new(|| NOW))?;
    broker.seal("token", "person:tom", &Secret::from_slice(b"value-one"))?;
    let issued = broker.issue(&holder, "token", 5, NOW + 60_000)?;
    let presented = Presentation::sign(&issued.id, &new_operation_id()?, NOW, [7; 32], &agent)?;
    let Admitted::Fresh(ticket) = broker.admit_use(&issued.token, &presented, 0)? else {
        return Err("the call was not admitted".into());
    };
    assert_eq!(
        anchored(&paths)?,
        broker.audit().len() - 1,
        "the use line waits for its settlement to move the anchor"
    );
    drop(ticket);
    let size = broker.audit().len();
    drop(broker);
    let reopened = Broker::open(&paths, granted(), Box::new(|| NOW))?;
    assert_eq!(
        reopened.audit().len(),
        size + 1,
        "the start settled the unsettled call as outcome_unknown"
    );
    assert_eq!(anchored(&paths)?, reopened.audit().len());
    Ok(())
}
