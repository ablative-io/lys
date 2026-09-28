#![cfg(test)]
//! R2: the identity-event envelope gives each event one signed byte string and refuses every other.

use std::error::Error;
use std::os::unix::fs::PermissionsExt;

use lys_core::Ed25519Identity;
use lys_identity::encoding::{decode_body, encode_body};
use lys_identity::{
    Actor, AgentId, AuthMethod, Change, IdentityError, IdentityEvent, IdentityId, LifecycleState,
    LinkChange, LinkObservation, LoginBinding, OperationId, PersonId, Profile, Provenance,
    Transition, sign_event, verify_event,
};

type TestResult = Result<(), Box<dyn Error>>;

/// A service key over a fixed seed, so no test depends on generated key material.
fn service_key(seed: u8) -> Result<(Ed25519Identity, tempfile::TempDir), Box<dyn Error>> {
    let dir = tempfile::TempDir::new()?;
    let path = dir.path().join("service.key");
    std::fs::write(&path, [seed; 32])?;
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
    Ok((Ed25519Identity::load(&path)?, dir))
}

fn actor() -> Result<Actor, IdentityError> {
    Ok(Actor::new(
        LoginBinding::new("https://issuer.test", "administrator")?,
        Provenance::new(AuthMethod::Oidc, 1_790_000_000),
    ))
}

fn event(identity: IdentityId, change: Change) -> Result<IdentityEvent, IdentityError> {
    IdentityEvent::new(
        OperationId::from_bytes([7; 16]),
        actor()?,
        identity,
        1_790_000_100,
        change,
    )
}

/// One event of every change kind, and both lifecycle shapes: with and without a reason.
fn every_change() -> Result<Vec<IdentityEvent>, IdentityError> {
    let person = IdentityId::Person(PersonId::from_bytes([1; 16]));
    let agent = IdentityId::Agent(AgentId::from_bytes([2; 16]));
    let transition = |transition, from, to, reason: &str| Change::Transition {
        transition,
        from,
        to,
        reason: reason.to_owned(),
    };
    Ok(vec![
        event(
            person,
            Change::RegisterPerson {
                profile: Profile::new("Ada")?,
            },
        )?,
        event(
            agent,
            Change::RegisterAgent {
                responsible: PersonId::from_bytes([1; 16]),
                profile: Profile::new("Ada's builder")?,
            },
        )?,
        event(
            agent,
            Change::ChangeProfile {
                profile: Profile::new("Builder")?,
            },
        )?,
        event(
            person,
            Change::BindLogin {
                binding: LoginBinding::new("https://issuer.test", "ada")?,
            },
        )?,
        event(
            agent,
            transition(
                Transition::Activate,
                LifecycleState::Registered,
                LifecycleState::Active,
                "",
            ),
        )?,
        event(
            agent,
            transition(
                Transition::Retire,
                LifecycleState::Active,
                LifecycleState::Retired,
                "replaced",
            ),
        )?,
        event(
            person,
            Change::SetupPerson {
                profile: Profile::new("Tom")?,
            },
        )?,
        event(
            person,
            Change::LinkAudit(LinkObservation::new(
                "source-op-1",
                LinkChange::Linked,
                LoginBinding::new("https://accounts.test", "ada-elsewhere")?,
                "https://issuer.test",
                1_790_000_050,
            )?),
        )?,
    ])
}

#[test]
fn every_change_signs_and_verifies_to_the_same_event_and_bytes() -> TestResult {
    let (key, _dir) = service_key(11)?;
    let events = every_change()?;
    let mut verified = 0;
    for original in events {
        let signed = sign_event(original.clone(), &key)?;
        let read = verify_event(signed.bytes(), &key.public_key_bytes())?;
        assert_eq!(read.event(), &original);
        assert_eq!(read.bytes(), signed.bytes());
        assert_eq!(read.payload_commitment(), signed.payload_commitment());
        assert_eq!(
            sign_event(original, &key)?.bytes(),
            signed.bytes(),
            "signing is deterministic"
        );
        verified += 1;
    }
    assert_eq!(verified, 8);
    Ok(())
}

#[test]
fn every_flipped_byte_is_refused() -> TestResult {
    let (key, _dir) = service_key(11)?;
    for original in every_change()? {
        let signed = sign_event(original, &key)?;
        let mut refused = 0;
        for index in 0..signed.bytes().len() {
            let mut altered = signed.bytes().to_vec();
            altered[index] ^= 0x01;
            if verify_event(&altered, &key.public_key_bytes()).is_err() {
                refused += 1;
            }
        }
        assert_eq!(refused, signed.bytes().len());
    }
    Ok(())
}

#[test]
fn another_service_key_is_refused_by_name() -> TestResult {
    let (key, _dir) = service_key(11)?;
    let (other, _other_dir) = service_key(12)?;
    let signed = sign_event(every_change()?.remove(0), &key)?;
    assert_eq!(
        verify_event(signed.bytes(), &other.public_key_bytes()),
        Err(IdentityError::SignerMismatch)
    );
    Ok(())
}

#[test]
fn trailing_bytes_are_not_canonical() -> TestResult {
    let (key, _dir) = service_key(11)?;
    let signed = sign_event(every_change()?.remove(0), &key)?;
    let mut padded = signed.bytes().to_vec();
    padded.push(0x00);
    assert_eq!(
        verify_event(&padded, &key.public_key_bytes()),
        Err(IdentityError::EventNotCanonical)
    );
    Ok(())
}

#[test]
fn a_long_form_head_is_not_canonical() -> TestResult {
    let body = encode_body(&every_change()?.remove(0));
    assert_eq!(body[0], 0xa7, "the body is a map of seven");
    let mut long = vec![0xb8, 0x07];
    long.extend_from_slice(&body[1..]);
    assert_eq!(decode_body(&long), Err(IdentityError::EventNotCanonical));
    Ok(())
}

#[test]
fn an_unknown_version_is_refused_by_name() -> TestResult {
    let mut body = encode_body(&every_change()?.remove(0));
    assert_eq!(&body[..3], &[0xa7, 0x01, 0x01], "key 1 holds version 1");
    body[2] = 0x02;
    assert_eq!(
        decode_body(&body),
        Err(IdentityError::VersionUnsupported { version: 2 })
    );
    Ok(())
}

#[test]
fn an_event_the_table_forbids_cannot_be_made() -> TestResult {
    let agent = IdentityId::Agent(AgentId::from_bytes([2; 16]));
    let person = IdentityId::Person(PersonId::from_bytes([1; 16]));
    let refusals = [
        event(
            agent,
            Change::Transition {
                transition: Transition::Retire,
                from: LifecycleState::Registered,
                to: LifecycleState::Retired,
                reason: "never active".to_owned(),
            },
        ),
        event(
            agent,
            Change::Transition {
                transition: Transition::Suspend,
                from: LifecycleState::Active,
                to: LifecycleState::Suspended,
                reason: String::new(),
            },
        ),
        event(
            agent,
            Change::Transition {
                transition: Transition::Activate,
                from: LifecycleState::Registered,
                to: LifecycleState::Suspended,
                reason: String::new(),
            },
        ),
        event(
            person,
            Change::RegisterAgent {
                responsible: PersonId::from_bytes([1; 16]),
                profile: Profile::new("x")?,
            },
        ),
        event(
            agent,
            Change::BindLogin {
                binding: LoginBinding::new("https://issuer.test", "a")?,
            },
        ),
    ];
    assert!(matches!(
        refusals[0],
        Err(IdentityError::TransitionRefused { .. })
    ));
    assert!(matches!(
        refusals[1],
        Err(IdentityError::ReasonRequired { .. })
    ));
    assert!(matches!(
        refusals[2],
        Err(IdentityError::ChangeMismatch { .. })
    ));
    assert!(matches!(
        refusals[3],
        Err(IdentityError::ChangeMismatch { .. })
    ));
    assert!(matches!(
        refusals[4],
        Err(IdentityError::ChangeMismatch { .. })
    ));
    assert_eq!(
        refusals.iter().filter(|refusal| refusal.is_err()).count(),
        5
    );
    Ok(())
}

mod directory_events {
    use std::error::Error;

    use identity_contract::fixtures::{administrator, op, shown};
    use identity_contract::harness::{Fault, Harness};
    use lys_identity::receipt::verify_receipt;
    use lys_identity::{IdentityError, IdentityId};
    use lys_log_store::{FileLeafStore, LeafStore};

    type TestResult = Result<(), Box<dyn Error>>;

    #[test]
    fn a_receipt_verifies_and_a_changed_one_does_not() -> TestResult {
        let harness = Harness::new(9)?;
        let mut directory = harness.open()?;
        let (_, receipt) = directory.register_person(administrator()?, op(1), shown("Ada")?, 10)?;
        directory.register_person(administrator()?, op(2), shown("Grace")?, 11)?;
        let index = receipt.coordinate().index;
        let leaf = directory
            .log()?
            .leaf(index)?
            .ok_or("leaf missing")?
            .to_vec();
        let checkpoint = directory.log()?.head()?;
        let proof = directory.log()?.inclusion_proof(index)?;
        let key = directory.service_key();
        verify_receipt(&receipt, &leaf, &key, checkpoint, &proof)?;
        let other = directory.log()?.leaf(1)?.ok_or("leaf missing")?.to_vec();
        assert!(verify_receipt(&receipt, &other, &key, checkpoint, &proof).is_err());
        let mut signature_flipped = leaf.clone();
        let last = signature_flipped.len() - 1;
        signature_flipped[last] ^= 1;
        assert!(verify_receipt(&receipt, &signature_flipped, &key, checkpoint, &proof).is_err());
        let moved = (checkpoint.0, [0; 32]);
        assert!(verify_receipt(&receipt, &leaf, &key, moved, &proof).is_err());
        Ok(())
    }

    #[test]
    fn an_append_whose_leaf_landed_is_found_committed_once() -> TestResult {
        let harness = Harness::new(9)?;
        let mut directory = harness.open()?;
        harness.fail(Fault::AfterLeaf);
        let (person, receipt) =
            directory.register_person(administrator()?, op(1), shown("Ada")?, 10)?;
        assert_eq!(receipt.coordinate().index, 0);
        assert_eq!(directory.log()?.len()?, 1);
        let again = directory.register_person(administrator()?, op(1), shown("Ada")?, 10)?;
        assert_eq!(again.0, person, "the retry finds the one committed event");
        assert_eq!(directory.log()?.len()?, 1);
        Ok(())
    }

    #[test]
    fn an_append_whose_leaf_never_landed_is_refused_and_records_nothing() -> TestResult {
        let harness = Harness::new(9)?;
        let mut directory = harness.open()?;
        harness.fail(Fault::BeforeLeaf);
        let refused = directory.register_person(administrator()?, op(1), shown("Ada")?, 10);
        assert!(matches!(refused, Err(IdentityError::AppendRefused { .. })));
        assert_eq!(directory.log()?.len()?, 0);
        directory.register_person(administrator()?, op(1), shown("Ada")?, 10)?;
        assert_eq!(directory.log()?.len()?, 1);
        Ok(())
    }

    #[test]
    fn an_uncertain_append_holds_every_read_until_it_is_resolved() -> TestResult {
        let harness = Harness::new(9)?;
        let mut directory = harness.open()?;
        harness.fail(Fault::AfterLeafUnreadable);
        let held = directory.register_person(administrator()?, op(1), shown("Ada")?, 10);
        assert!(matches!(held, Err(IdentityError::LogUnavailable { .. })));
        assert!(
            directory.projection().is_err(),
            "no read answers while the append is uncertain"
        );
        harness.fail(Fault::None);
        assert_eq!(
            directory.projection()?.records().count(),
            1,
            "resolved as committed, applied once"
        );
        let live = directory.projection()?.clone();
        drop(directory);
        assert_eq!(harness.open()?.projection()?, &live);
        Ok(())
    }

    #[test]
    fn a_leaf_stored_behind_a_failed_write_is_found_committed_once() -> TestResult {
        let harness = Harness::new(9)?;
        let mut directory = harness.open()?;
        harness.fail(Fault::LeafStoredWriteFailed);
        let (person, receipt) =
            directory.register_person(administrator()?, op(1), shown("Ada")?, 10)?;
        assert_eq!(
            receipt.coordinate().index,
            0,
            "the stored leaf is the answer"
        );
        let again = directory.register_person(administrator()?, op(1), shown("Ada")?, 10)?;
        assert_eq!(again.0, person, "the retry finds the one committed event");
        drop(directory);
        assert_eq!(harness.open()?.log()?.len()?, 1);
        Ok(())
    }

    fn leaf_file(harness: &Harness, index: u64) -> std::path::PathBuf {
        harness
            .log_path()
            .join("leaves")
            .join(format!("{index:020}"))
    }

    #[test]
    fn a_torn_leaf_past_the_pin_is_refused_before_it_is_pinned() -> TestResult {
        let harness = Harness::new(9)?;
        let mut directory = harness.open()?;
        directory.register_person(administrator()?, op(1), shown("Ada")?, 10)?;
        let whole = directory.log()?.leaf(0)?.ok_or("leaf missing")?.to_vec();
        drop(directory);
        std::fs::write(leaf_file(&harness, 1), &whole[..whole.len() / 2])?;
        let refused = harness.open();
        assert!(
            matches!(
                refused
                    .as_ref()
                    .map_err(|error| error.downcast_ref::<IdentityError>()),
                Err(Some(IdentityError::LeafNotAnEvent { index: 1, .. }))
            ),
            "a torn leaf is refused by name"
        );
        let store = FileLeafStore::open(&harness.log_path())?;
        assert_eq!(
            store.pinned().tree_size,
            1,
            "the torn leaf was never pinned"
        );
        std::fs::remove_file(leaf_file(&harness, 1))?;
        assert_eq!(harness.open()?.projection()?.records().count(), 1);
        Ok(())
    }

    #[test]
    fn a_second_writers_leaf_is_applied_when_an_append_finds_it() -> TestResult {
        let harness = Harness::new(9)?;
        let mut first = harness.open()?;
        let mut second = harness.open()?;
        first.register_person(administrator()?, op(1), shown("Ada")?, 10)?;
        let refused = second.register_person(administrator()?, op(2), shown("Grace")?, 11);
        assert!(matches!(refused, Err(IdentityError::AppendRefused { .. })));
        assert_eq!(
            second.projection()?.records().count(),
            1,
            "the other writer's event is applied, not skipped"
        );
        let again = second.register_person(administrator()?, op(1), shown("Ada")?, 10);
        assert!(
            again.is_ok(),
            "the adopted operation answers its first receipt"
        );
        second.register_person(administrator()?, op(2), shown("Grace")?, 11)?;
        assert_eq!(second.log()?.len()?, 2);
        let live = second.projection()?.clone();
        drop((first, second));
        assert_eq!(harness.open()?.projection()?, &live);
        Ok(())
    }

    #[test]
    fn nothing_reads_the_log_or_a_record_while_an_append_is_uncertain() -> TestResult {
        let harness = Harness::new(9)?;
        let mut directory = harness.open()?;
        let (ada, _) = directory.register_person(administrator()?, op(1), shown("Ada")?, 10)?;
        harness.fail(Fault::AfterLeafUnreadable);
        let held = directory.register_person(administrator()?, op(2), shown("Grace")?, 11);
        assert!(matches!(held, Err(IdentityError::LogUnavailable { .. })));
        assert!(
            directory.log().is_err(),
            "the log is not served while uncertain"
        );
        assert!(
            directory.record(IdentityId::Person(ada)).is_err(),
            "no record is served while uncertain"
        );
        harness.fail(Fault::None);
        assert_eq!(directory.log()?.len()?, 2);
        Ok(())
    }

    #[test]
    fn the_same_operation_retried_after_the_log_was_unavailable_records_once() -> TestResult {
        let harness = Harness::new(9)?;
        let mut directory = harness.open()?;
        harness.fail(Fault::AfterLeafUnreadable);
        let held = directory.register_person(administrator()?, op(1), shown("Ada")?, 10);
        assert!(matches!(held, Err(IdentityError::LogUnavailable { .. })));
        harness.fail(Fault::None);
        let (person, receipt) =
            directory.register_person(administrator()?, op(1), shown("Ada")?, 10)?;
        assert_eq!(
            receipt.coordinate().index,
            0,
            "the retry answers the first leaf"
        );
        assert_eq!(directory.log()?.len()?, 1);
        assert!(directory.record(IdentityId::Person(person))?.is_some());
        Ok(())
    }
}
