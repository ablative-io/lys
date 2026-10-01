//! A signing use returns an attestation, under the same lease bounds as a forward.

use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use lys_core::Ed25519Identity;
use lys_core::attestation::verify_attestation_bytes_by_signer;
use tempfile::TempDir;

use crate::encoding::{sha256, unhex};
use crate::{
    AuditKind, Broker, BrokerPaths, Checked, EntryClass, Holder, IssuedHandle, LocalGrants,
    Presentation, Relation, Secret, SecretRelation, SecretsError, Used, new_operation_id,
    request_digest,
};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const NOW: i64 = 1_800_000_000_000;
const SECRET: &str = "agent-signing";
const PERSON: &str = "person:keeper";
const SEED: &str = "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60";
const PUBLIC: &str = "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a";

struct World {
    root: TempDir,
    broker: Broker<LocalGrants>,
    holder_key: Ed25519Identity,
    issued: IssuedHandle,
    clock: Arc<AtomicI64>,
}

impl World {
    fn new(class: EntryClass, bytes: &[u8], uses: u64, cap: Option<u64>) -> TestResult<Self> {
        let root = tempfile::tempdir()?;
        let keys = root.path().join("keys");
        std::fs::create_dir(&keys)?;
        let paths = BrokerPaths {
            store_dir: root.path().join("store"),
            log_dir: root.path().join("log"),
            store_key: keys.join("store.key"),
            audit_key: keys.join("audit.key"),
            anchor: keys.join("audit.anchor"),
        };
        let grants = LocalGrants::new();
        grants.grant(SecretRelation {
            identity: "agent:holder".to_owned(),
            secret: SECRET.to_owned(),
            granted_by: Some(PERSON.to_owned()),
        })?;
        grants.grant_as(
            Relation::Member,
            SecretRelation {
                identity: "agent:holder".to_owned(),
                secret: format!("person/{PERSON}"),
                granted_by: Some(PERSON.to_owned()),
            },
        )?;
        let clock = Arc::new(AtomicI64::new(NOW));
        let read_clock = Arc::clone(&clock);
        let mut broker = Broker::create(
            &paths,
            grants,
            Box::new(move || read_clock.load(Ordering::Relaxed)),
        )?;
        broker.seal_once(SECRET, class, PERSON, &Secret::from_slice(bytes))?;
        let holder_key = Ed25519Identity::ephemeral();
        let holder = Holder {
            identity: "agent:holder".to_owned(),
            key: holder_key.public_key_bytes(),
        };
        let issued = broker.issue_capped(&holder, SECRET, uses, NOW + 1_000, cap)?;
        Ok(Self {
            root,
            broker,
            holder_key,
            issued,
            clock,
        })
    }

    fn present(&self, payload: &[u8]) -> Result<Presentation, SecretsError> {
        Presentation::sign(
            &self.issued.id,
            &new_operation_id()?,
            self.clock.load(Ordering::Relaxed),
            request_digest("SIGN", SECRET, payload)?,
            &self.holder_key,
        )
    }

    fn checked(&self) -> Checked {
        Checked::answer(
            &self.broker.asks_for(&self.issued.token),
            self.broker.permissions(),
        )
    }
}

fn seed() -> TestResult<Vec<u8>> {
    unhex(SEED).ok_or_else(|| "invalid seed fixture".into())
}

fn public() -> TestResult<[u8; 32]> {
    Ok(unhex(PUBLIC)
        .ok_or("invalid public fixture")?
        .try_into()
        .map_err(|bytes: Vec<u8>| format!("invalid public length: {}", bytes.len()))?)
}

#[test]
fn a_sign_lease_signs_a_digest_and_never_returns_the_key() -> TestResult {
    let seed = seed()?;
    let mut world = World::new(EntryClass::Key, &seed, 2, None)?;
    let payload = sha256(b"agent request");
    let presentation = world.present(&payload)?;
    let checked = world.checked();
    let answer = world.broker.sign_use_for_checked(
        &world.issued.token,
        &presentation,
        SECRET,
        &payload,
        0,
        &checked,
    )?;
    let Used::Forwarded { answer, uses_left } = answer else {
        return Err("first use was retried".into());
    };
    assert_eq!(uses_left, 1);
    verify_attestation_bytes_by_signer(answer.as_bytes(), &payload, &public()?)?;
    assert!(
        verify_attestation_bytes_by_signer(answer.as_bytes(), b"another payload", &public()?)
            .is_err()
    );
    assert!(
        !answer
            .as_bytes()
            .windows(seed.len())
            .any(|bytes| bytes == seed)
    );
    assert!(matches!(world.broker.sign_use_for_checked(
        &world.issued.token, &presentation, SECRET, &payload, 0, &checked,
    )?, Used::Retried { outcome } if outcome == "completed"));
    let settlements: Vec<_> = world
        .broker
        .audit()
        .audit_every_line()?
        .into_iter()
        .filter(|record| record.line.kind == AuditKind::Settlement)
        .collect();
    assert_eq!(settlements.len(), 1);
    assert_eq!(settlements[0].line.outcome, "completed");
    drop(world.broker);
    world.root.close()?;
    Ok(())
}

#[test]
fn a_revoked_or_spent_lease_signs_nothing() -> TestResult {
    let seed = seed()?;
    let payload = b"request";
    let mut world = World::new(EntryClass::Key, &seed, 1, None)?;
    let presentation = world.present(payload)?;
    let checked = world.checked();
    world.broker.sign_use_for_checked(
        &world.issued.token,
        &presentation,
        SECRET,
        payload,
        0,
        &checked,
    )?;
    let fresh = world.present(payload)?;
    assert!(matches!(
        world.broker.sign_use_for_checked(
            &world.issued.token,
            &fresh,
            SECRET,
            payload,
            0,
            &checked
        ),
        Err(SecretsError::LeaseExhausted { .. })
    ));
    let mut revoke = |lease: &str, secret: &str| {
        assert_eq!(lease, world.issued.id.as_str());
        assert_eq!(secret, SECRET);
    };
    world
        .broker
        .revoke_lease(PERSON, &world.issued.id, &mut revoke)?;
    let fresh = world.present(payload)?;
    let checked = world.checked();
    assert!(matches!(
        world.broker.sign_use_for_checked(
            &world.issued.token,
            &fresh,
            SECRET,
            payload,
            0,
            &checked,
        ),
        Err(SecretsError::HandleDropped { .. })
    ));
    assert_eq!(
        world
            .broker
            .audit()
            .audit_every_line()?
            .into_iter()
            .filter(|record| record.line.kind == AuditKind::Settlement)
            .count(),
        1
    );

    Ok(())
}

#[test]
fn signing_requires_a_reservation_within_the_cap() -> TestResult {
    let seed = seed()?;
    let payload = b"request";
    let mut world = World::new(EntryClass::Key, &seed, 4, Some(10))?;
    let checked = world.checked();
    for (reserve, name) in [(0, "ReservationMissing"), (11, "SpendCapReached")] {
        let fresh = world.present(payload)?;
        let error = world
            .broker
            .sign_use_for_checked(
                &world.issued.token,
                &fresh,
                SECRET,
                payload,
                reserve,
                &checked,
            )
            .expect_err("reservation must refuse");
        assert_eq!(error.name(), name);
    }
    Ok(())
}

#[test]
fn signing_releases_the_reservation_at_zero_measured_spend() -> TestResult {
    let seed = seed()?;
    let payload = b"request";
    let mut world = World::new(EntryClass::Key, &seed, 4, Some(10))?;
    let checked = world.checked();
    let fresh = world.present(payload)?;
    world.broker.sign_use_for_checked(
        &world.issued.token,
        &fresh,
        SECRET,
        payload,
        10,
        &checked,
    )?;
    let fresh = world.present(payload)?;
    world.broker.sign_use_for_checked(
        &world.issued.token,
        &fresh,
        SECRET,
        payload,
        10,
        &checked,
    )?;
    let settled: Vec<_> = world
        .broker
        .audit()
        .audit_every_line()?
        .into_iter()
        .filter(|record| record.line.kind == AuditKind::Settlement)
        .map(|record| record.line.spend)
        .collect();
    assert_eq!(settled, [Some(0), Some(0)]);
    Ok(())
}

#[test]
fn signing_refuses_a_closed_window() -> TestResult {
    let seed = seed()?;
    let payload = b"request";
    let mut world = World::new(EntryClass::Key, &seed, 4, Some(10))?;
    let checked = world.checked();
    world.clock.store(NOW + 1_001, Ordering::Relaxed);
    let fresh = world.present(payload)?;
    assert!(matches!(
        world.broker.sign_use_for_checked(
            &world.issued.token,
            &fresh,
            SECRET,
            payload,
            10,
            &checked
        ),
        Err(SecretsError::LeaseWindowClosed { .. })
    ));
    Ok(())
}

#[test]
fn signing_refuses_removed_permission() -> TestResult {
    let seed = seed()?;
    let payload = b"request";
    let mut world = World::new(EntryClass::Key, &seed, 4, Some(10))?;
    world.broker.permissions().revoke("agent:holder", SECRET)?;
    let fresh = world.present(payload)?;
    let checked = world.checked();
    assert!(matches!(
        world.broker.sign_use_for_checked(
            &world.issued.token,
            &fresh,
            SECRET,
            payload,
            10,
            &checked
        ),
        Err(SecretsError::PermissionDenied { .. })
    ));
    Ok(())
}

#[test]
fn signing_binds_the_payload_and_the_presented_request() -> TestResult {
    let seed = seed()?;
    let mut world = World::new(EntryClass::Key, &seed, 2, None)?;
    let presentation = world.present(b"one payload")?;
    let checked = world.checked();
    let mut altered = presentation.clone();
    altered.request[0] ^= 1;
    assert!(matches!(
        world.broker.sign_use_for_checked(
            &world.issued.token,
            &altered,
            SECRET,
            b"one payload",
            0,
            &checked,
        ),
        Err(SecretsError::PresentationInvalid { .. })
    ));
    assert!(matches!(
        world.broker.sign_use_for_checked(
            &world.issued.token,
            &presentation,
            SECRET,
            b"different payload",
            0,
            &checked
        ),
        Err(SecretsError::PresentationInvalid { .. })
    ));
    Ok(())
}

fn refused_entry(class: EntryClass, bytes: &[u8], name: &str) -> TestResult {
    let mut world = World::new(class, bytes, 2, None)?;
    let presentation = world.present(b"request")?;
    let checked = world.checked();
    let error = world
        .broker
        .sign_use_for_checked(
            &world.issued.token,
            &presentation,
            SECRET,
            b"request",
            0,
            &checked,
        )
        .expect_err("invalid signing entry must refuse");
    assert_eq!(error.name(), name);
    let settlements: Vec<_> = world
        .broker
        .audit()
        .audit_every_line()?
        .into_iter()
        .filter(|record| record.line.kind == AuditKind::Settlement)
        .collect();
    assert_eq!(settlements.len(), 1);
    assert_eq!(settlements[0].line.outcome, name);
    Ok(())
}

#[test]
fn signing_refuses_a_credential_entry() -> TestResult {
    refused_entry(EntryClass::Credential, &seed()?, "SigningKeyRequired")
}

#[test]
fn signing_refuses_a_short_seed() -> TestResult {
    refused_entry(EntryClass::Key, &seed()?[..31], "SigningSeedInvalid")
}

#[test]
fn signing_refuses_an_empty_seed() -> TestResult {
    refused_entry(EntryClass::Key, &[], "SigningSeedInvalid")
}

#[test]
fn signing_refuses_a_long_seed() -> TestResult {
    refused_entry(EntryClass::Key, &[0; 33], "SigningSeedInvalid")
}
