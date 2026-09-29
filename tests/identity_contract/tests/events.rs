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
    body[2] = 0x03;
    assert_eq!(
        decode_body(&body),
        Err(IdentityError::VersionUnsupported { version: 3 })
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
        let leaf = directory.log()?.leaf(index)?.ok_or("leaf missing")?;
        let checkpoint = directory.log()?.head()?;
        let proof = directory.log()?.inclusion_proof(index)?;
        let key = directory.service_key();
        verify_receipt(&receipt, &leaf, &key, checkpoint, &proof)?;
        let other = directory.log()?.leaf(1)?.ok_or("leaf missing")?;
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
        let whole = directory.log()?.leaf(0)?.ok_or("leaf missing")?;
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

mod receipts {
    use std::error::Error;
    use std::os::unix::fs::PermissionsExt;

    use coset::{CoseSign1, TaggedCborSerializable};
    use identity_contract::fake_issuer::{CLIENT_SECRET, Login};
    use identity_contract::fixtures::{administrator, op, shown};
    use identity_contract::harness::{ADMINISTRATOR, Harness, Service};
    use lys_core::Ed25519Identity;
    use lys_identity::log::Coordinate;
    use lys_identity::receipt::{Receipt, verify_receipt};
    use lys_identity::signer::load_service_key;
    use lys_identity::{
        Actor, AuthMethod, Change, IdentityEvent, LoginBinding, Profile, Provenance, sign_event,
        verify_event,
    };
    use sha2::{Digest, Sha256};

    type TestResult = Result<(), Box<dyn Error>>;

    fn node(left: &[u8], right: &[u8]) -> [u8; 32] {
        let mut hash = Sha256::new();
        hash.update([1]);
        hash.update(left);
        hash.update(right);
        hash.finalize().into()
    }

    /// RFC 9162 section 2.1.3.2, written from the RFC and not from
    /// lys-core, so the inclusion check has a second author.
    fn rfc_inclusion(
        index: u64,
        tree_size: u64,
        leaf: &[u8],
        proof: &[u8],
        root: [u8; 32],
    ) -> bool {
        let siblings = proof.chunks_exact(32);
        if index >= tree_size || !siblings.remainder().is_empty() {
            return false;
        }
        let mut hash = Sha256::new();
        hash.update([0]);
        hash.update(leaf);
        let mut climbed: [u8; 32] = hash.finalize().into();
        let (mut at, mut last) = (index, tree_size - 1);
        for sibling in siblings {
            if last == 0 {
                return false;
            }
            if at & 1 == 1 || at == last {
                climbed = node(sibling, &climbed);
                while at & 1 == 0 && at != 0 {
                    at >>= 1;
                    last >>= 1;
                }
            } else {
                climbed = node(&climbed, sibling);
            }
            at >>= 1;
            last >>= 1;
        }
        last == 0 && climbed == root
    }

    /// Check a receipt the way a stranger would: parse the leaf with
    /// standard COSE tooling, verify its Ed25519 signature under the service
    /// key it was handed, recompute the SHA-256 payload commitment and the
    /// RFC 6962 leaf hash, and walk the inclusion proof by the RFC.
    fn independently(
        receipt: &Receipt,
        leaf: &[u8],
        key: &[u8; 32],
        (tree_size, root): (u64, [u8; 32]),
        proof: &[u8],
    ) -> Result<bool, Box<dyn Error>> {
        let Ok(sign1) = CoseSign1::from_tagged_slice(leaf) else {
            return Ok(false);
        };
        let verify = |signature: &[u8], data: &[u8]| Ed25519Identity::verify(key, data, signature);
        if sign1.verify_signature(b"", verify).is_err() {
            return Ok(false);
        }
        let payload = sign1.payload.ok_or("the signed event carries no payload")?;
        let commitment: [u8; 32] = Sha256::digest(&payload).into();
        let mut leaf_hash = Sha256::new();
        leaf_hash.update([0]);
        leaf_hash.update(leaf);
        let leaf_hash: [u8; 32] = leaf_hash.finalize().into();
        let committed = commitment == receipt.payload_commitment();
        let hashed = leaf_hash == receipt.coordinate().leaf_hash;
        let placed = rfc_inclusion(receipt.coordinate().index, tree_size, leaf, proof, root);
        Ok(committed && hashed && placed)
    }

    /// `ID001_RECEIPT`: a recorded change verifies independently against a
    /// checkpoint and the service key, and a receipt whose actor, payload,
    /// sequence or signature was changed fails, both by the crate's verifier
    /// and by the independent check. Every tampered case is counted.
    #[test]
    fn a_receipt_verifies_independently_and_each_changed_field_fails() -> TestResult {
        let harness = Harness::new(9)?;
        let signing = Ed25519Identity::load(&harness.dir.path().join("service.key"))?;
        let mut directory = harness.open()?;
        let (_, receipt) = directory.register_person(administrator()?, op(1), shown("Ada")?, 10)?;
        directory.register_person(administrator()?, op(2), shown("Grace")?, 11)?;
        let index = receipt.coordinate().index;
        let leaf = directory.log()?.leaf(index)?.ok_or("leaf missing")?;
        let checkpoint = directory.log()?.head()?;
        let proof = directory.log()?.inclusion_proof(index)?;
        let key = directory.service_key();

        verify_receipt(&receipt, &leaf, &key, checkpoint, &proof)?;
        assert!(
            independently(&receipt, &leaf, &key, checkpoint, proof.as_bytes())?,
            "the genuine receipt verifies with standard tooling"
        );

        let original = verify_event(&leaf, &key)?;
        let event = original.event();
        let remade = |actor: Actor, change: Change| {
            IdentityEvent::new(
                event.operation(),
                actor,
                event.identity(),
                event.recorded_at(),
                change,
            )
            .and_then(|made| sign_event(made, &signing))
        };
        let other_actor = Actor::new(
            LoginBinding::new("https://issuer.test", "someone-else")?,
            Provenance::new(AuthMethod::Oidc, 1_790_000_000),
        );
        let changed_actor = Receipt::of(
            &remade(other_actor, event.change().clone())?,
            receipt.coordinate(),
        );
        let changed_payload = Receipt::of(
            &remade(
                event.actor().clone(),
                Change::RegisterPerson {
                    profile: Profile::new("Mallory")?,
                },
            )?,
            receipt.coordinate(),
        );
        let changed_sequence = Receipt::of(
            &original,
            Coordinate {
                index: index + 1,
                ..receipt.coordinate()
            },
        );
        let mut signature_changed = leaf.clone();
        let last = signature_changed.len() - 1;
        signature_changed[last] ^= 1;

        let cases: [(&str, &Receipt, &[u8]); 4] = [
            ("actor", &changed_actor, &leaf),
            ("payload", &changed_payload, &leaf),
            ("sequence", &changed_sequence, &leaf),
            ("signature", &receipt, &signature_changed),
        ];
        let mut failed = 0;
        for (field, tampered, message) in cases {
            assert!(
                verify_receipt(tampered, message, &key, checkpoint, &proof).is_err(),
                "a changed {field} fails the verifier"
            );
            assert!(
                !independently(tampered, message, &key, checkpoint, proof.as_bytes())?,
                "a changed {field} fails the independent check"
            );
            failed += 1;
        }
        assert_eq!(failed, 4, "one failure per changed field");
        Ok(())
    }

    fn login(subject: &str) -> Login {
        Login {
            subject: subject.to_owned(),
            email: "shared@example.test".to_owned(),
        }
    }

    fn hex(bytes: &[u8]) -> String {
        const DIGITS: &[u8; 16] = b"0123456789abcdef";
        let mut out = String::with_capacity(bytes.len() * 2);
        for byte in bytes {
            out.push(char::from(DIGITS[usize::from(byte >> 4)]));
            out.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
        }
        out
    }

    /// `ID001_RECEIPT`: no secret reaches a debug form, an error or a public
    /// response. The secrets are the service's signing seed, the OIDC
    /// client secret and the session cookie's value; the service key's
    /// public half is shown to prove the answers were read at all.
    #[tokio::test]
    async fn no_secret_reaches_a_debug_form_an_error_or_a_public_response() -> TestResult {
        let service = Service::start().await?;
        let seed = std::fs::read(service.dir.path().join("service.key"))?;
        let cookie = service.sign_in(login(ADMINISTRATOR)).await?;
        let session = cookie
            .split_once('=')
            .map(|(_, value)| value.to_owned())
            .ok_or("the cookie has no value")?;
        let secrets = [hex(&seed), CLIENT_SECRET.to_owned(), session];
        let carries_none = |text: &str| secrets.iter().all(|secret| !text.contains(secret));

        let key = load_service_key(&service.dir.path().join("service.key"))?;
        let debug = format!("{key:?}");
        assert!(carries_none(&debug), "the key's debug form: {debug}");
        assert!(!debug.contains(&format!("{:?}", seed.as_slice())));
        let short = service.dir.path().join("short.key");
        std::fs::write(&short, &seed[..31])?;
        std::fs::set_permissions(&short, std::fs::Permissions::from_mode(0o600))?;
        let refused = load_service_key(&short).err().ok_or("a short key loaded")?;
        let shown = format!("{refused} {refused:?}");
        assert!(carries_none(&shown), "the key error: {shown}");
        assert!(!shown.contains(&hex(&seed[..31])));

        let (status, body) = service
            .post(
                "/people",
                Some(&cookie),
                &serde_json::json!({ "operation": op(1).to_string(), "display_name": "Ada" }),
            )
            .await?;
        assert_eq!(status, 200, "{body}");
        let second_use =
            serde_json::json!({ "operation": op(1).to_string(), "display_name": "Grace" });
        let (status, refusal) = service.post("/people", Some(&cookie), &second_use).await?;
        assert_ne!(status, 200, "{refusal}");
        assert_eq!(refusal["refusal"], "OperationReused", "{refusal}");
        let mut answers = vec![body, refusal];
        for path in ["/service-key", "/receipts/0"] {
            let (status, answer) = service.get(path, None).await?;
            assert_eq!(status, 200, "{path}: {answer}");
            answers.push(answer);
        }
        let (_, listed) = service.get("/identities", Some(&cookie)).await?;
        answers.push(listed);
        let public = hex(&key.public_key_bytes());
        assert_eq!(answers[2]["ed25519"], public, "the public key is answered");
        let mut read = 0;
        for answer in &answers {
            assert!(carries_none(&answer.to_string()), "{answer}");
            read += 1;
        }
        assert_eq!(read, 5, "every answer was searched");
        Ok(())
    }
}

mod faults {
    use std::error::Error;

    use identity_contract::fixtures::{administrator, op, shown};
    use identity_contract::harness::{Fault, Harness};
    use lys_identity::{IdentityError, IdentityId};

    type TestResult = Result<(), Box<dyn Error>>;

    /// The boundary after `fault`, walked by an exhaustive match so a new
    /// crash boundary in the harness cannot be left out of the enumeration.
    fn after(fault: Fault) -> Option<Fault> {
        match fault {
            Fault::None => Some(Fault::BeforeLeaf),
            Fault::BeforeLeaf => Some(Fault::LeafStoredWriteFailed),
            Fault::LeafStoredWriteFailed => Some(Fault::AfterLeaf),
            Fault::AfterLeaf => Some(Fault::AfterLeafUnreadable),
            Fault::AfterLeafUnreadable => None,
        }
    }

    /// How an interrupted operation is recovered: by the same process
    /// retrying it, or by a restart that reopens the log and then retries.
    #[derive(Debug, Clone, Copy)]
    enum Recovery {
        Retry,
        Restart,
    }

    /// Drive one boundary: Ada is registered, Grace's registration is cut at
    /// `fault`, and then recovered as `recovery` says. Every projection the
    /// directory answers along the way equals a fresh replay of the log, and
    /// Grace ends up registered exactly once under one enduring id.
    fn exercise(fault: Fault, recovery: Recovery) -> TestResult {
        let harness = Harness::new(9)?;
        let mut directory = harness.open()?;
        directory.register_person(administrator()?, op(1), shown("Ada")?, 10)?;
        harness.fail(fault);
        let first = directory.register_person(administrator()?, op(2), shown("Grace")?, 11);
        if matches!(first, Err(IdentityError::LogUnavailable { .. })) {
            assert!(
                directory.projection().is_err(),
                "{fault:?}: no projection is answered while the append is uncertain"
            );
        }
        harness.fail(Fault::None);
        if matches!(recovery, Recovery::Restart) {
            drop(directory);
            directory = harness.open()?;
        }
        let (grace, _) = directory.register_person(administrator()?, op(2), shown("Grace")?, 11)?;
        if let Ok((answered, _)) = &first {
            assert_eq!(*answered, grace, "{fault:?}: the first answer's id is kept");
        }
        let (again, _) = directory.register_person(administrator()?, op(2), shown("Grace")?, 11)?;
        assert_eq!(again, grace, "{fault:?}: resolved once, answered the same");
        assert_eq!(directory.log()?.len()?, 2, "{fault:?}: no loss, no double");
        let graces = directory
            .projection()?
            .records()
            .filter(|(_, record)| record.profile().display_name() == "Grace")
            .count();
        assert_eq!(graces, 1, "{fault:?}: Grace is one identity");
        let answered = directory.projection()?.clone();
        drop(directory);
        assert_eq!(
            harness.open()?.projection()?,
            &answered,
            "{fault:?} {recovery:?}: the answered projection equals replay"
        );
        Ok(())
    }

    /// `ID001_AUDIT_FAULTS`: every append, pin and projection crash boundary
    /// the harness can cut (none, meaning the process stops after the
    /// commit and before its answer; before the leaf; the leaf stored
    /// behind a failed write; after the leaf with the pin lost; and after
    /// the leaf with the store unreadable) is exercised under both
    /// recoveries, and the exercised cases are counted.
    #[test]
    fn every_crash_boundary_resolves_once_and_answers_equal_replay() -> TestResult {
        let mut boundaries = vec![Fault::None];
        while let Some(next) = boundaries.last().copied().and_then(after) {
            boundaries.push(next);
        }
        let mut exercised = 0;
        for fault in &boundaries {
            for recovery in [Recovery::Retry, Recovery::Restart] {
                exercise(*fault, recovery)?;
                exercised += 1;
            }
        }
        assert_eq!(boundaries.len(), 5, "five boundaries enumerated");
        assert_eq!(exercised, 10, "each boundary under both recoveries");
        Ok(())
    }

    /// The projection boundary on its own: an operation whose leaf is
    /// committed but whose answer never reached the caller is found by the
    /// same operation id after a restart, and keeps its identity.
    #[test]
    fn an_answer_lost_after_the_commit_is_found_by_its_operation_after_a_restart() -> TestResult {
        let harness = Harness::new(9)?;
        let mut directory = harness.open()?;
        let (lost, receipt) =
            directory.register_person(administrator()?, op(1), shown("Ada")?, 10)?;
        drop(directory);
        let mut restarted = harness.open()?;
        let (found, again) =
            restarted.register_person(administrator()?, op(1), shown("Ada")?, 10)?;
        assert_eq!(found, lost);
        assert_eq!(again, receipt);
        assert_eq!(restarted.log()?.len()?, 1);
        assert!(restarted.record(IdentityId::Person(found))?.is_some());
        Ok(())
    }
}
