#![cfg(test)]
//! A signing key is a secret with one named purpose (SECRETS-006 R1), at
//! the library. Each value-bearing read of a secret is asked of a signing
//! key and must be refused `not_a_value_secret`, with its refusal on the
//! audit record where the read writes one. The route tests are in
//! `tests/signing.rs`.

use std::path::Path;

use lys_core::Ed25519Identity;

use crate::audit::AuditKind;
use crate::broker::{Broker, BrokerPaths, Ticket};
use crate::handle::{Holder, IssuedHandle, Presentation, new_operation_id};
use crate::local_grants::{LocalGrants, SecretRelation};
use crate::permission::Relation;
use crate::secret::Secret;
use crate::store::EntryClass;

use super::SigningPurpose;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const NOW: i64 = 1_800_000_000_000;
const KEY: &str = "agent-key";
const OWNER: &str = "person:tom";
const AGENT: &str = "agent:noor";
const NOT_A_VALUE: &str = "not_a_value_secret";

/// A broker holding the signing key `KEY` for `agent_request`, which
/// `AGENT` holds both the use and the read relation on.
fn broker(dir: &Path) -> TestResult<Broker<LocalGrants>> {
    let keys = dir.join("keys");
    std::fs::create_dir_all(&keys)?;
    let paths = BrokerPaths {
        store_dir: dir.join("store"),
        log_dir: dir.join("log"),
        store_key: keys.join("store.key"),
        audit_key: keys.join("audit.key"),
        anchor: keys.join("audit.anchor"),
    };
    let grants = LocalGrants::new();
    for relation in [Relation::Use, Relation::Read] {
        grants.grant_as(
            relation,
            SecretRelation {
                identity: AGENT.to_owned(),
                secret: KEY.to_owned(),
                granted_by: Some(OWNER.to_owned()),
            },
        );
    }
    let mut broker = Broker::create(&paths, grants, Box::new(|| NOW))?;
    let seed = Secret::from_slice(&[0x2b; 32]);
    broker.seal_signing_key(KEY, OWNER, SigningPurpose::AgentRequest, &seed)?;
    Ok(broker)
}

fn holder_key(dir: &Path) -> TestResult<Ed25519Identity> {
    Ok(Ed25519Identity::load_or_generate(&dir.join("agent.key"))?)
}

/// A handle on `KEY` issued to `AGENT`.
fn issued(broker: &mut Broker<LocalGrants>, dir: &Path) -> TestResult<IssuedHandle> {
    let holder = Holder {
        identity: AGENT.to_owned(),
        key: holder_key(dir)?.public_key_bytes(),
    };
    Ok(broker.issue(&holder, KEY, 5, NOW + 60_000)?)
}

/// The outcomes of the audit lines of `kind`, in order.
fn outcomes(broker: &Broker<LocalGrants>, kind: AuditKind) -> TestResult<Vec<String>> {
    Ok(broker
        .audit()
        .audit_every_line()?
        .into_iter()
        .filter(|recorded| recorded.line.kind == kind)
        .map(|recorded| recorded.line.outcome)
        .collect())
}

#[test]
fn a_seed_of_31_bytes_is_refused_signing_key_invalid_naming_31() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut broker = broker(dir.path())?;
    let short = Secret::from_slice(&[7; 31]);
    let refused = broker
        .seal_signing_key("short-key", OWNER, SigningPurpose::AgentRequest, &short)
        .err()
        .ok_or("a seed of 31 bytes was sealed")?;
    assert_eq!(refused.name(), "signing_key_invalid");
    assert!(refused.to_string().contains("31 bytes"), "{refused}");
    assert!(broker.store().entry("short-key").is_none());
    Ok(())
}

#[test]
fn a_purpose_outside_the_list_is_refused_signing_purpose_unknown_naming_it() -> TestResult {
    let refused = SigningPurpose::parse("git_commit")
        .err()
        .ok_or("git_commit was read as a signing purpose")?;
    assert_eq!(refused.name(), "signing_purpose_unknown");
    assert!(refused.to_string().contains("git_commit"), "{refused}");
    assert_eq!(
        SigningPurpose::parse("agent_request")?,
        SigningPurpose::AgentRequest
    );
    Ok(())
}

#[test]
fn a_value_use_of_a_signing_key_is_refused_before_it_is_admitted() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut broker = broker(dir.path())?;
    let issued = issued(&mut broker, dir.path())?;
    let presented = Presentation::sign(
        &issued.id,
        &new_operation_id()?,
        NOW,
        [1; 32],
        &holder_key(dir.path())?,
    )?;
    let refused = broker
        .admit_use_for(&issued.token, &presented, KEY, 0)
        .err()
        .ok_or("a value use of a signing key was admitted")?;
    assert_eq!(refused.name(), NOT_A_VALUE);
    assert_eq!(outcomes(&broker, AuditKind::Use)?, vec![NOT_A_VALUE]);
    assert!(outcomes(&broker, AuditKind::Settlement)?.is_empty());
    Ok(())
}

#[test]
fn the_oauth_proxy_refuses_a_signing_key_not_a_value_secret() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut broker = broker(dir.path())?;
    let issued = issued(&mut broker, dir.path())?;
    let refused = broker
        .oauth_grant_of(&issued.id)
        .err()
        .ok_or("the revocation after a drop read a signing key as a grant")?;
    assert_eq!(refused.name(), NOT_A_VALUE);

    let ticket = Ticket {
        handle: issued.id.as_str().to_owned(),
        identity: AGENT.to_owned(),
        secret: KEY.to_owned(),
        operation: String::new(),
        mark: String::new(),
        reserved: None,
        entry: KEY.to_owned(),
        class: EntryClass::SigningKey,
        uses_left: 4,
        credential: Secret::from_slice(b"never opened for a value"),
    };
    let refused = ticket
        .oauth()
        .err()
        .ok_or("the proxy read a signing key's ticket as a grant")?;
    assert_eq!(refused.name(), NOT_A_VALUE);
    Ok(())
}

#[test]
fn the_login_hand_over_at_spawn_refuses_a_signing_key_not_a_value_secret() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut broker = broker(dir.path())?;
    let refused = broker
        .spawn_login(AGENT, KEY)
        .err()
        .ok_or("a signing key was handed over as a login")?;
    assert_eq!(refused.name(), NOT_A_VALUE);
    assert_eq!(outcomes(&broker, AuditKind::SpawnLogin)?, vec![NOT_A_VALUE]);
    Ok(())
}

#[test]
fn the_sealed_record_read_refuses_a_signing_key_not_a_value_secret() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut broker = broker(dir.path())?;
    let refused = broker
        .read_record(AGENT, KEY, None)
        .err()
        .ok_or("a signing key was read as a record")?;
    assert_eq!(refused.name(), NOT_A_VALUE);
    assert_eq!(outcomes(&broker, AuditKind::SealedRead)?, vec![NOT_A_VALUE]);
    Ok(())
}
