//! Adversarial and end-to-end tests for delegation issuance and verification.
//!
//! Every mutant below is signed **for real** by a real key over its own bytes,
//! so each one is a cryptographically perfect artifact and the only thing that
//! can reject it is the rule it was built to probe. A mutant that failed its
//! signature check would prove nothing about the rule; it would prove the
//! signature check works, which is tested separately and once.
//!
//! Each rule from the specification gets its own case, so a drift that disables
//! one rule fails one test. Where a case could be caught by two rules at once,
//! it is constructed so only the intended one can fire.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use crate::delegation::artifact::{DelegationRole, DelegationSubjectKind};
use crate::merkle::tree::{AppendOnlyTree, RawLeaf};
use crate::receipt;

/// The subject kind every case below expects unless it is testing the kind
/// itself. Spelled as a constant so the argument is visible at each call site
/// rather than hidden behind a helper — it is a required argument *because* a
/// verifier must state it.
const DOMAIN: DelegationSubjectKind = DelegationSubjectKind::Domain;
/// The other kind, for the cross-kind cases.
const SEAT: DelegationSubjectKind = DelegationSubjectKind::Seat;

/// The origin under test. `example.test` is a reserved name and deliberately
/// not any configured production origin — a test carrying a real origin would
/// be a committed origin constant.
const ORIGIN: &str = "example.test";
const OTHER_ORIGIN: &str = "attacker.test";

/// Build a deterministic identity from a fixed 32-byte seed, so every test is
/// reproducible and the failure cases name concrete keys.
fn identity(seed: &[u8; 32]) -> Ed25519Identity {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("root.key");
    std::fs::write(&path, seed).unwrap();
    Ed25519Identity::load(&path).unwrap()
}

/// The real anchor's offline root key.
fn root() -> Ed25519Identity {
    identity(b"lys-anchor-delegation-root-seed1")
}

/// An attacker's own root key — a perfectly valid Ed25519 key that nobody
/// trusts.
fn attacker() -> Ed25519Identity {
    identity(b"lys-anchor-delegation-evil-seed2")
}

/// The operational key being delegated to.
fn operational() -> Ed25519Identity {
    identity(b"lys-anchor-delegation-oper-seed3")
}

fn claim() -> DelegationClaim {
    claim_for(ORIGIN)
}

fn claim_for(origin: &str) -> DelegationClaim {
    claim_at(origin, 300)
}

/// A claim at an explicit `sequence`. The default of 300 is the specification
/// vector's, chosen because it is neither 0 (indistinguishable from a field an
/// implementation forgot to write) nor 1 (which `role` already carries, so a
/// field swap would be masked).
fn claim_at(subject_value: &str, sequence: u64) -> DelegationClaim {
    DelegationClaim {
        subject_kind: DelegationSubjectKind::Domain,
        subject_value: subject_value.to_string(),
        delegated_public_key: operational().public_key_bytes(),
        role: DelegationRole::Operational,
        not_before_unix_ms: 1_700_000_000_000,
        sequence,
    }
}

/// The `(seat, speaks-for)` claim for `subject_value` — the other valid pair.
///
/// Its default subject value is **the same string** the domain claim uses, so
/// the cross-kind cases below turn on the kind alone. A seat identifier is
/// arbitrary text minted elsewhere, so that collision is an attacker's choice.
fn seat_claim_for(subject_value: &str) -> DelegationClaim {
    DelegationClaim {
        subject_kind: DelegationSubjectKind::Seat,
        subject_value: subject_value.to_string(),
        delegated_public_key: operational().public_key_bytes(),
        role: DelegationRole::SpeaksFor,
        not_before_unix_ms: 1_700_000_000_000,
        sequence: 300,
    }
}

/// The offset of the `04` role label inside a canonical payload, located by its
/// **neighbours** rather than by a byte search, because `[0x04, 0x02]` can also
/// occur inside the 32-byte delegated key or the subject value. The payload's
/// tail is structurally fixed: `04 <role> 05 <not_before head> 06 <sequence
/// head>`, and both head widths are properties of these fixtures' values.
fn role_offset(payload: &[u8]) -> usize {
    // `06 19 01 2c` (4) + `05 1b <8 bytes>` (10) + `04 <role>` (2) = 16.
    let at = payload.len() - 4 - 10 - 2;
    assert_eq!(payload[at], 0x04, "label 4 must be where the tail puts it");
    assert_eq!(payload[at + 2], 0x05, "label 5 must follow the role value");
    at
}

/// Sign `protected ‖ payload` for real with `key` and wrap the result in the
/// COSE envelope — the mutant factory. Whatever the buckets say, the signature
/// over them is genuine.
fn signed_envelope(protected: &[u8], payload: &[u8], key: &Ed25519Identity) -> Vec<u8> {
    let signature = key.sign(&cbor::sig_structure_bytes(protected, payload));
    encoding::envelope_bytes(protected, payload, &signature)
}

/// The canonical protected bucket for `key`.
fn protected_for(key: &Ed25519Identity) -> Vec<u8> {
    encoding::protected_bytes(encoding::CONTENT_TYPE, &key.public_key_bytes())
}

/// A protected bucket whose `kid` is a `bstr` of `len` bytes rather than 32 —
/// hand-assembled, because the encoder cannot emit one.
fn protected_with_kid_len(len: u8) -> Vec<u8> {
    let mut out = vec![0xa3, 0x01, 0x27, 0x03, 0x78, 0x26];
    out.extend_from_slice(encoding::CONTENT_TYPE.as_bytes());
    out.extend_from_slice(&[0x04, 0x58, len]);
    out.extend(std::iter::repeat_n(0x11u8, usize::from(len)));
    out
}

// ---------------------------------------------------------------------------
// The positive path. Without this, every rejection test below would be
// satisfied by a verifier that refuses everything.
// ---------------------------------------------------------------------------

#[test]
fn a_signed_delegation_verifies_and_carries_back_every_field() {
    let key = root();
    let cose = sign_delegation(&key, &claim()).unwrap();
    let verified = verify_delegation(&cose, &key.public_key_bytes(), DOMAIN, ORIGIN).unwrap();

    assert_eq!(verified.root_public_key, key.public_key_bytes());
    assert_eq!(verified.claim, claim());
    assert_eq!(
        verified.claim.delegated_public_key,
        operational().public_key_bytes()
    );
    assert_eq!(verified.claim.role, DelegationRole::Operational);
    // The parsed value re-encodes to exactly the bytes that were verified.
    assert_eq!(verified.to_cose_bytes(), cose);
}

#[test]
fn the_two_phase_path_produces_the_same_artifact_as_the_one_shot_path() {
    // The offline-signing path and the convenience must agree byte for byte, or
    // an air-gapped operator would produce artifacts nobody else can reproduce.
    let key = root();
    let root_key = key.public_key_bytes();
    let preimage = delegation_preimage(&root_key, &claim());
    let signature = key.sign(&preimage);
    let assembled = assemble_delegation(&root_key, &claim(), &signature).unwrap();
    assert_eq!(assembled, sign_delegation(&key, &claim()).unwrap());

    // And the preimage really is the COSE Sig_structure over both buckets.
    assert_eq!(
        preimage,
        cbor::sig_structure_bytes(&protected_for(&key), &encoding::payload_bytes(&claim()))
    );
    assert_eq!(&preimage[..12], b"\x84\x6aSignature1");

    // `external_aad` is pinned to the empty byte string `h''` (`0x40`). It sits
    // inside the signed bytes, so an unpinned value would mean two conforming
    // implementations produce different signatures over the same claim and
    // neither is wrong. Located structurally: context, then the protected bstr.
    let external_aad_at = 12 + 2 + encoding::PROTECTED_LEN;
    assert_eq!(
        preimage[external_aad_at], 0x40,
        "external_aad must be the zero-length byte string"
    );
    assert_eq!(
        &preimage[12..14],
        &[0x58, 0x4f],
        "the protected bstr head precedes it — 79 bytes"
    );
}

#[test]
fn assembly_refuses_a_signature_that_does_not_match_its_own_bytes() {
    // Catching this here is the whole point: an artifact whose signature does
    // not match its bytes is indistinguishable from a forgery to everyone who
    // receives it later.
    let key = root();
    let root_key = key.public_key_bytes();
    let good = key.sign(&delegation_preimage(&root_key, &claim()));

    let mut flipped = good;
    flipped[0] ^= 0x01;
    assert!(matches!(
        assemble_delegation(&root_key, &claim(), &flipped),
        Err(TrustError::DelegationVerification)
    ));

    // A signature by the wrong key, and one over a different claim, are refused
    // for the same reason.
    let wrong_signer = attacker().sign(&delegation_preimage(&root_key, &claim()));
    assert!(assemble_delegation(&root_key, &claim(), &wrong_signer).is_err());

    let wrong_claim = key.sign(&delegation_preimage(&root_key, &claim_for(OTHER_ORIGIN)));
    assert!(assemble_delegation(&root_key, &claim(), &wrong_claim).is_err());

    // Positive control: the untouched signature assembles.
    assert!(assemble_delegation(&root_key, &claim(), &good).is_ok());
}

#[test]
fn issuance_is_deterministic() {
    // Ed25519 signing is deterministic and the encoder is fixed-shape, so the
    // same inputs must produce byte-identical artifacts. One that varied run to
    // run could not be deduplicated, compared, or logged by its bytes.
    let key = root();
    assert_eq!(
        sign_delegation(&key, &claim()).unwrap(),
        sign_delegation(&key, &claim()).unwrap()
    );
}

// ---------------------------------------------------------------------------
// §3.1 — cross-format confusion.
// ---------------------------------------------------------------------------

#[test]
fn a_delegation_relabelled_as_a_receipt_is_refused_though_its_signature_is_valid() {
    // The injection from §6.2: content type changed to the receipt's. The root
    // key signs the relabelled bucket for real, so the artifact is
    // cryptographically perfect and only the content-type pin can refuse it.
    let key = root();
    let relabelled = encoding::protected_bytes(
        "application/vnd.lys.receipt.v1+cbor",
        &key.public_key_bytes(),
    );
    let mutant = signed_envelope(&relabelled, &encoding::payload_bytes(&claim()), &key);

    // The mutant's own signature checks out — this is not a forgery.
    Ed25519Identity::verify(
        &key.public_key_bytes(),
        &cbor::sig_structure_bytes(&relabelled, &encoding::payload_bytes(&claim())),
        &mutant[mutant.len() - 64..],
    )
    .unwrap();

    assert!(matches!(
        verify_delegation(&mutant, &key.public_key_bytes(), DOMAIN, ORIGIN),
        Err(TrustError::DelegationVerification)
    ));
}

#[test]
fn a_real_receipt_is_not_a_delegation_and_a_delegation_is_not_a_receipt() {
    // Both directions, on genuine artifacts issued by the same key — the
    // strongest form of the cross-format claim, since key separation is doing
    // none of the work.
    let key = root();

    let mut tree = AppendOnlyTree::<RawLeaf>::new();
    tree.append_raw(b"genesis");
    tree.append_raw(b"leaf");
    let path: Vec<[u8; 32]> = tree
        .prove_inclusion(1)
        .unwrap()
        .as_bytes()
        .chunks_exact(32)
        .map(|c| <[u8; 32]>::try_from(c).unwrap())
        .collect();
    let receipt_bytes = receipt::sign_receipt(b"leaf", 1, 2, &path, &key)
        .unwrap()
        .to_cose_bytes();
    let delegation_bytes = sign_delegation(&key, &claim()).unwrap();

    // Each verifies as itself: the positive control for this test.
    receipt::verify_receipt_bytes(&receipt_bytes, b"leaf", &key.public_key_bytes()).unwrap();
    verify_delegation(&delegation_bytes, &key.public_key_bytes(), DOMAIN, ORIGIN).unwrap();

    // Neither verifies as the other.
    assert!(verify_delegation(&receipt_bytes, &key.public_key_bytes(), DOMAIN, ORIGIN).is_err());
    assert!(
        receipt::verify_receipt_bytes(&delegation_bytes, b"leaf", &key.public_key_bytes()).is_err()
    );

    // And the preimages share their first twelve bytes, which is exactly why the
    // separation has to live inside the signed bytes rather than in a prefix.
    let delegation_preimage_bytes =
        cbor::sig_structure_bytes(&protected_for(&key), &encoding::payload_bytes(&claim()));
    assert_eq!(&delegation_preimage_bytes[..12], b"\x84\x6aSignature1");
}

// ---------------------------------------------------------------------------
// §3.2 — THE CENTRAL TRAP. `kid` is a claim, not an authority.
// ---------------------------------------------------------------------------

#[test]
fn a_delegation_signed_by_an_attacker_naming_their_own_key_is_refused() {
    // The injection from §6.2: `kid` replaced with the attacker's root key.
    //
    // The attacker signs a delegation for OUR origin with THEIR root key and
    // puts THEIR key in `kid`. The artifact is internally perfect — it parses,
    // it is canonical, its content type is right, its origin is right, and its
    // signature verifies against the key it carries. It vouches for nothing.
    let evil = attacker();
    let forged = sign_delegation(&evil, &claim()).unwrap();

    // Internally perfect, proven rather than asserted: verified against the key
    // it names, it passes.
    let self_consistent =
        verify_delegation(&forged, &evil.public_key_bytes(), DOMAIN, ORIGIN).unwrap();
    assert_eq!(self_consistent.root_public_key, evil.public_key_bytes());
    assert_eq!(self_consistent.claim.subject_value, ORIGIN);

    // And refused the moment the caller names the root key they actually trust.
    assert!(matches!(
        verify_delegation(&forged, &root().public_key_bytes(), DOMAIN, ORIGIN),
        Err(TrustError::DelegationVerification)
    ));
}

#[test]
fn substituting_the_kid_in_a_genuine_delegation_breaks_the_signature_too() {
    // `kid` rides in the signature-covered protected bucket, so even a caller
    // fooled into expecting the substituted key gets a rejection. This is
    // defence in depth behind the expected-key check, not a replacement for it —
    // the test above is the one that matters, because there the signature is
    // genuine.
    let key = root();
    let evil = attacker();
    let mutant = encoding::envelope_bytes(
        &encoding::protected_bytes(encoding::CONTENT_TYPE, &evil.public_key_bytes()),
        &encoding::payload_bytes(&claim()),
        &key.sign(&delegation_preimage(&key.public_key_bytes(), &claim())),
    );
    assert!(verify_delegation(&mutant, &evil.public_key_bytes(), DOMAIN, ORIGIN).is_err());
}

#[test]
fn a_kid_that_is_not_exactly_thirty_two_bytes_is_refused() {
    // RFC 9052 §3.1 types `kid` as a `bstr`; this format pins its length, so a
    // slot that could not hold an Ed25519 key never parses. The bucket is
    // hand-assembled and signed for real, so only the length rule can refuse it.
    let key = root();
    let mut refused = 0;
    for len in [0u8, 1, 31, 33, 64] {
        let protected = protected_with_kid_len(len);
        let mutant = signed_envelope(&protected, &encoding::payload_bytes(&claim()), &key);
        assert!(
            verify_delegation(&mutant, &key.public_key_bytes(), DOMAIN, ORIGIN).is_err(),
            "a {len}-byte kid was accepted"
        );
        refused += 1;
    }
    assert_eq!(refused, 5, "every kid length must have been tried");

    // Positive control: the same hand-assembled construction at 32 bytes is a
    // well-formed bucket, so the refusals above are about the length alone.
    let genuine = protected_with_kid_len(32);
    assert_eq!(
        genuine,
        encoding::protected_bytes(encoding::CONTENT_TYPE, &[0x11; 32])
    );
}

// ---------------------------------------------------------------------------
// §1.0 — the tag is mandatory.
// ---------------------------------------------------------------------------

#[test]
fn an_untagged_cose_sign1_is_refused_though_its_signature_is_valid() {
    // RFC 9052 §4.2 permits an untagged `COSE_Sign1` "depending on the
    // context", so accepting both forms would give one statement two valid
    // encodings — the defect the canonicality rule exists to prevent. The
    // untagged form here is byte-for-byte the tagged artifact minus its first
    // byte, so the signature over the buckets is untouched and genuine.
    let key = root();
    let good = sign_delegation(&key, &claim()).unwrap();
    verify_delegation(&good, &key.public_key_bytes(), DOMAIN, ORIGIN).unwrap();

    assert_eq!(good[0], 0xd2, "tag 18");
    let untagged = &good[1..];
    assert_eq!(untagged[0], 0x84, "a bare 4-array, which is valid COSE");
    assert!(
        ciborium::de::from_reader::<ciborium::value::Value, _>(untagged).is_ok(),
        "the untagged form must be well-formed CBOR for this test to mean anything"
    );

    assert!(matches!(
        verify_delegation(untagged, &key.public_key_bytes(), DOMAIN, ORIGIN),
        Err(TrustError::DelegationVerification)
    ));
}

// ---------------------------------------------------------------------------
// §3.3 — cross-origin replay.
// ---------------------------------------------------------------------------

#[test]
fn a_delegation_for_another_origin_is_refused_though_it_is_internally_perfect() {
    // The injection from §6.2: payload origin changed. Same root key, same
    // delegated key, same role — only the origin differs, and the signature over
    // it is genuine.
    let key = root();
    let elsewhere = sign_delegation(&key, &claim_for(OTHER_ORIGIN)).unwrap();

    // It is a perfectly good delegation for the origin it names.
    verify_delegation(&elsewhere, &key.public_key_bytes(), DOMAIN, OTHER_ORIGIN).unwrap();

    // And is not one for ours, even though the root key is the one we trust —
    // the case an operator sharing a root key across anchors actually hits.
    assert!(matches!(
        verify_delegation(&elsewhere, &key.public_key_bytes(), DOMAIN, ORIGIN),
        Err(TrustError::DelegationVerification)
    ));

    // Origin comparison is exact: no prefix, suffix or case relaxation.
    let mut refused = 0;
    for near_miss in [
        "example.tes",
        "example.test.",
        "Example.Test",
        "sub.example.test",
        " example.test",
    ] {
        let cose = sign_delegation(&key, &claim_for(near_miss)).unwrap();
        assert!(
            verify_delegation(&cose, &key.public_key_bytes(), DOMAIN, ORIGIN).is_err(),
            "origin {near_miss:?} must not satisfy {ORIGIN:?}"
        );
        refused += 1;
    }
    assert_eq!(refused, 5, "every near-miss origin must have been tried");
}

#[test]
fn origin_comparison_is_raw_utf8_byte_equality() {
    // The test vector's origin is ASCII and so cannot expose this; an
    // internationalised one can. `café.test` in NFC (U+00E9) and in NFD
    // (`e` + U+0301) render identically and are different byte strings — a
    // Unicode-normalising comparison would treat them as one origin, and an
    // attacker holding a delegation for either would then hold one for both.
    let key = root();
    let nfc = "caf\u{e9}.test";
    let nfd = "cafe\u{301}.test";
    assert_ne!(nfc.as_bytes(), nfd.as_bytes());

    let cose = sign_delegation(&key, &claim_for(nfc)).unwrap();
    // Positive control: it verifies against its own byte string.
    verify_delegation(&cose, &key.public_key_bytes(), DOMAIN, nfc).unwrap();
    assert!(verify_delegation(&cose, &key.public_key_bytes(), DOMAIN, nfd).is_err());

    // And the reverse direction, so neither form is privileged.
    let cose_nfd = sign_delegation(&key, &claim_for(nfd)).unwrap();
    verify_delegation(&cose_nfd, &key.public_key_bytes(), DOMAIN, nfd).unwrap();
    assert!(verify_delegation(&cose_nfd, &key.public_key_bytes(), DOMAIN, nfc).is_err());
}

// ---------------------------------------------------------------------------
// §3.3 — cross-KIND replay, which is the point of typing the subject.
// ---------------------------------------------------------------------------

/// ⛔ **The collision case.** A seat delegation and a domain delegation naming
/// the **same subject string** must not be interchangeable.
///
/// A verifier that compared only the value would accept the seat delegation as
/// authority over the origin. This is not a contrived overlap: a seat identifier
/// is arbitrary text minted by a registry elsewhere, so an attacker who can
/// choose one chooses the target's origin string. The kind argument is what makes
/// the two namespaces disjoint by construction instead of by hope.
#[test]
fn the_two_subject_kinds_do_not_interchange_even_at_the_same_subject_value() {
    let key = root();
    let expected = key.public_key_bytes();

    let domain = sign_delegation(&key, &claim_for(ORIGIN)).unwrap();
    let seat = sign_delegation(&key, &seat_claim_for(ORIGIN)).unwrap();

    // The premise, asserted rather than assumed: one string, two artifacts.
    assert_eq!(
        AnchorDelegation::from_cose_bytes(&domain)
            .unwrap()
            .claim
            .subject_value,
        AnchorDelegation::from_cose_bytes(&seat)
            .unwrap()
            .claim
            .subject_value,
        "the two artifacts must name the same subject VALUE for this test to be \
         about the KIND"
    );
    assert_ne!(domain, seat, "and they must still be different artifacts");

    // Positive controls: each verifies under its own kind. Without these, a
    // verifier that refused both would satisfy the two refusals below.
    let ok_domain = verify_delegation(&domain, &expected, DOMAIN, ORIGIN).unwrap();
    assert_eq!(ok_domain.claim.subject_kind, DelegationSubjectKind::Domain);
    assert_eq!(ok_domain.claim.role, DelegationRole::Operational);
    let ok_seat = verify_delegation(&seat, &expected, SEAT, ORIGIN).unwrap();
    assert_eq!(ok_seat.claim.subject_kind, DelegationSubjectKind::Seat);
    assert_eq!(ok_seat.claim.role, DelegationRole::SpeaksFor);

    // And neither is accepted by the other's verifier, though both are signed by
    // the root key that verifier trusts and both name the string it expects.
    let mut refused = 0;
    for (name, artifact, kind) in [
        ("a seat delegation at a domain verifier", &seat, DOMAIN),
        ("a domain delegation at a seat verifier", &domain, SEAT),
    ] {
        assert!(
            matches!(
                verify_delegation(artifact, &expected, kind, ORIGIN),
                Err(TrustError::DelegationVerification)
            ),
            "{name} was accepted"
        );
        refused += 1;
    }
    assert_eq!(refused, 2, "both directions must have been tried");
}

// ---------------------------------------------------------------------------
// §1.2 / §3.3 — the (subject_kind, role) pair.
// ---------------------------------------------------------------------------

/// Spec §2.3 clause 3, with a genuine signature: the two pairs outside the table
/// whose halves are each individually recognised.
///
/// Only those two — the transposition cases fail *role decode* and belong to
/// `encoding_tests::transposing_the_subject_kind_and_role_labels_fails_role_decode_not_the_pair_check`,
/// which asserts that attribution. Bundling them here would make this test pass
/// for the wrong reason and would leave clause 3 unproven by any single case.
#[test]
fn a_pair_outside_the_table_is_refused_though_its_signature_is_valid() {
    let key = root();
    let expected = key.public_key_bytes();
    let canonical = encoding::payload_bytes(&claim());
    assert_eq!(&canonical[..3], &[0xa6, 0x01, 0x01], "kind at index 2");
    let role_at = role_offset(&canonical);

    // (kind byte, role byte, why it must be refused)
    let cases = [
        (
            0x01u8,
            0x03u8,
            "domain + speaks-for: both values valid, pair is not",
        ),
        (
            0x02,
            0x02,
            "seat + operational: both values valid, pair is not",
        ),
    ];

    let mut refused = 0;
    for (kind, role, why) in cases {
        // The premise of clause 3: each half decodes on its own.
        let kind_enum = DelegationSubjectKind::from_wire(u64::from(kind)).expect("kind decodes");
        let role_enum = DelegationRole::from_wire(u64::from(role)).expect("role decodes");
        assert!(!kind_enum.permits(role_enum), "{why}");
        let mut payload = canonical.clone();
        payload[2] = kind;
        payload[role_at + 1] = role;
        assert_eq!(payload.len(), canonical.len(), "a repair, not a resize");
        let mutant = signed_envelope(&protected_for(&key), &payload, &key);

        // The mutant is cryptographically perfect: only the pair rule can refuse
        // it. Proven rather than asserted.
        Ed25519Identity::verify(
            &expected,
            &cbor::sig_structure_bytes(&protected_for(&key), &payload),
            &mutant[mutant.len() - 64..],
        )
        .unwrap();

        // Refused under BOTH kinds a caller could name, so no configuration
        // accepts it.
        for kind_expected in [DOMAIN, SEAT] {
            assert!(
                matches!(
                    verify_delegation(&mutant, &expected, kind_expected, ORIGIN),
                    Err(TrustError::DelegationVerification)
                ),
                "({kind}, {role}) was accepted — {why}"
            );
        }
        refused += 1;
    }
    assert_eq!(refused, 2, "both clause-3 pairs must have been tried");

    // Positive controls through the identical construction: the two VALID pairs
    // verify. Without them these refusals would also be satisfied by a decoder
    // that had stopped parsing payloads at all.
    let mut accepted = 0;
    for (kind, role, kind_expected) in [(0x01u8, 0x02u8, DOMAIN), (0x02, 0x03, SEAT)] {
        let mut payload = canonical.clone();
        payload[2] = kind;
        payload[role_at + 1] = role;
        let ok = signed_envelope(&protected_for(&key), &payload, &key);
        verify_delegation(&ok, &expected, kind_expected, ORIGIN).unwrap();
        accepted += 1;
    }
    assert_eq!(
        accepted, 2,
        "both valid pairs must have been controlled for"
    );
}

/// The issuing side refuses the same pairs, through both entry points.
///
/// Without this, `sign_delegation` would happily mint an artifact its own
/// `verify_delegation` refuses — the defect class `check_encodable` exists to
/// close, arriving through the newest rule.
#[test]
fn issuance_refuses_an_invalid_kind_role_pair_through_both_entry_points() {
    let key = root();
    let root_key = key.public_key_bytes();

    let mut refused = 0;
    for (kind, role) in [
        (DelegationSubjectKind::Domain, DelegationRole::SpeaksFor),
        (DelegationSubjectKind::Seat, DelegationRole::Operational),
    ] {
        let mut c = claim();
        c.subject_kind = kind;
        c.role = role;
        assert!(
            matches!(
                sign_delegation(&key, &c),
                Err(TrustError::DelegationEncoding { .. })
            ),
            "({kind:?}, {role:?}) was issuable through the convenience path"
        );
        // And the air-gapped two-phase route, which does not go through
        // `sign_delegation` at all.
        let signature = key.sign(&delegation_preimage(&root_key, &c));
        assert!(
            matches!(
                assemble_delegation(&root_key, &c, &signature),
                Err(TrustError::DelegationEncoding { .. })
            ),
            "({kind:?}, {role:?}) was issuable through the two-phase path"
        );
        refused += 1;
    }
    assert_eq!(refused, 2, "both invalid pairs must have been tried");

    // Positive controls: both valid pairs issue and verify end to end.
    let mut issued = 0;
    for (c, kind_expected) in [(claim(), DOMAIN), (seat_claim_for(ORIGIN), SEAT)] {
        let cose = sign_delegation(&key, &c).unwrap();
        assert_eq!(
            verify_delegation(&cose, &root_key, kind_expected, &c.subject_value)
                .unwrap()
                .claim,
            c
        );
        issued += 1;
    }
    assert_eq!(issued, 2, "both valid pairs must have been issued");
}

// ---------------------------------------------------------------------------
// §3.4 — canonical-encoding strictness, with a valid signature.
// ---------------------------------------------------------------------------

#[test]
fn a_permuted_payload_map_is_refused_though_its_signature_is_valid() {
    // Payload map keys emitted in permuted order (2, 1, 3, 4, 5 instead of
    // 1, 2, 3, 4, 5). Every value is identical and a permissive CBOR reader
    // recovers the same statement; the root key signs these exact bytes, so the
    // signature is genuine.
    //
    // **What refuses this is the positional decode pins, not the byte-compare.**
    // §6.2 originally designated the permutation as the isolating injection for
    // the canonicality rule, and an adversarial review disproved that by
    // deleting the byte-compare and watching this test still pass. The
    // designated case is now `the_envelope_malleability_sweep_is_refused`'s
    // indefinite outer array. This test is kept because the rule it *does* guard
    // is real: without it a permuted map would be a second valid artifact for
    // one statement, and to a conforming stranger — RFC 9052 §9 does not
    // constrain map key ordering — it would be a perfectly good one.
    let key = root();
    let claim = claim();

    let mut permuted = vec![0xa6, 0x03, 0x58, 0x20];
    permuted.extend_from_slice(&claim.delegated_public_key);
    permuted.extend_from_slice(&[0x01, 0x01, 0x02]);
    permuted.push(0x60 | u8::try_from(ORIGIN.len()).unwrap());
    permuted.extend_from_slice(ORIGIN.as_bytes());
    permuted.extend_from_slice(&[0x04, 0x02, 0x05, 0x1b]);
    permuted.extend_from_slice(&claim.not_before_unix_ms.to_be_bytes());
    permuted.extend_from_slice(&[0x06, 0x19, 0x01, 0x2c]);

    let canonical = encoding::payload_bytes(&claim);
    assert_eq!(
        permuted.len(),
        canonical.len(),
        "a permutation, not a different payload"
    );
    assert_ne!(permuted, canonical);

    let mutant = signed_envelope(&protected_for(&key), &permuted, &key);
    assert!(
        ciborium::de::from_reader::<ciborium::value::Value, _>(mutant.as_slice()).is_ok(),
        "the mutant must be well-formed CBOR for this test to mean anything"
    );
    assert!(matches!(
        verify_delegation(&mutant, &key.public_key_bytes(), DOMAIN, ORIGIN),
        Err(TrustError::DelegationVerification)
    ));
}

// ---------------------------------------------------------------------------
// §1.3 — the unprotected header must be empty.
// ---------------------------------------------------------------------------

#[test]
fn a_non_empty_unprotected_header_is_refused_though_the_signature_still_verifies() {
    // The injection from §6.2: the unprotected header carries one entry. This is
    // the sharpest of the seven, because the unprotected bucket is *not*
    // signature-covered: the entry can be added to a genuine artifact after the
    // fact and the root key's signature over the rest still verifies perfectly.
    // Only the empty-bucket rule stands between an attacker and unsigned data
    // riding inside a trusted artifact.
    let key = root();
    let good = sign_delegation(&key, &claim()).unwrap();
    verify_delegation(&good, &key.public_key_bytes(), DOMAIN, ORIGIN).unwrap();

    let unprotected_at = 4 + encoding::PROTECTED_LEN;
    assert_eq!(good[unprotected_at], 0xa0, "the empty map");

    let mut mutant = good[..unprotected_at].to_vec();
    mutant.extend_from_slice(&[0xa1, 0x01, 0x01]); // {1: 1}
    mutant.extend_from_slice(&good[unprotected_at + 1..]);

    assert!(
        ciborium::de::from_reader::<ciborium::value::Value, _>(mutant.as_slice()).is_ok(),
        "the mutant must be well-formed CBOR for this test to mean anything"
    );
    // The signature is untouched and still covers the protected bucket and the
    // payload, both of which are byte-identical to the genuine artifact's.
    assert_eq!(&mutant[mutant.len() - 64..], &good[good.len() - 64..]);

    assert!(matches!(
        verify_delegation(&mutant, &key.public_key_bytes(), DOMAIN, ORIGIN),
        Err(TrustError::DelegationVerification)
    ));
}

// ---------------------------------------------------------------------------
// §2.3 — unknown roles are rejected, not carried.
// ---------------------------------------------------------------------------

#[test]
fn an_unknown_role_is_refused_though_its_signature_is_valid() {
    // The injection from §6.2: `role` set to 7. The root key signs it for real,
    // so a tolerant implementation would hand a consumer a cryptographically
    // perfect delegation whose meaning nobody defines — a signed unchecked value
    // that looks checked.
    let key = root();
    let canonical = encoding::payload_bytes(&claim());
    let role_at = role_offset(&canonical);

    // `1` is in the list on purpose: it is the DOMAIN kind's wire value, so an
    // artifact carrying it in the role slot is what a transposed implementation
    // would produce. `3` is a valid role but not for a domain, so it belongs to
    // the pair test rather than here.
    let mut refused = 0;
    for role in [0u8, 1, 4, 7, 23] {
        let mut payload = canonical.clone();
        payload[role_at + 1] = role;
        assert_eq!(payload.len(), canonical.len());
        let mutant = signed_envelope(&protected_for(&key), &payload, &key);
        assert!(
            matches!(
                verify_delegation(&mutant, &key.public_key_bytes(), DOMAIN, ORIGIN),
                Err(TrustError::DelegationVerification)
            ),
            "role {role} was accepted"
        );
        refused += 1;
    }
    assert_eq!(refused, 5, "every unknown role must have been tried");

    // Positive control: the same construction with the known role is accepted,
    // so the refusals above are about the role and not about the construction.
    let ok = signed_envelope(&protected_for(&key), &canonical, &key);
    verify_delegation(&ok, &key.public_key_bytes(), DOMAIN, ORIGIN).unwrap();
}

// ---------------------------------------------------------------------------
// §3.6 — strict Ed25519.
// ---------------------------------------------------------------------------

#[test]
fn a_flipped_signature_bit_is_refused() {
    // The injection from §6.2: a byte flipped in the signature.
    let key = root();
    let good = sign_delegation(&key, &claim()).unwrap();
    let at = good.len() - 64;

    let mut refused = 0;
    for byte in [0usize, 1, 31, 32, 62, 63] {
        for bit in [0x01u8, 0x80] {
            let mut mutant = good.clone();
            mutant[at + byte] ^= bit;
            assert!(
                verify_delegation(&mutant, &key.public_key_bytes(), DOMAIN, ORIGIN).is_err(),
                "a flip of bit {bit:#04x} in signature byte {byte} was accepted"
            );
            refused += 1;
        }
    }
    assert_eq!(refused, 12, "every flip must have been tried");
}

#[test]
fn a_non_canonical_signature_scalar_is_refused() {
    // Strict verification rejects a malleable `s`; non-repudiation requires a
    // unique valid signature per message. Setting the high bits of `s` puts it
    // outside the canonical range, which plain (non-strict) verification would
    // accept.
    let key = root();
    let mut mutant = sign_delegation(&key, &claim()).unwrap();
    let last = mutant.len() - 1;
    mutant[last] |= 0xe0;
    assert!(verify_delegation(&mutant, &key.public_key_bytes(), DOMAIN, ORIGIN).is_err());
}

// ---------------------------------------------------------------------------
// §2.2 — `sequence` orders delegations, and why it had to exist.
// ---------------------------------------------------------------------------

/// The replay defect the `sequence` field was added to close, demonstrated
/// rather than described.
///
/// This test asserts a property that looks like a *weakness* — that two
/// issuances of one claim are byte-identical — because that property is exactly
/// what made ordering-by-log-position exploitable, and it is still true. What
/// changed is that the artifact now carries a number that makes a replayed copy
/// lose.
#[test]
fn a_replayed_delegation_is_byte_identical_which_is_why_sequence_exists() {
    let key = root();

    // 1. The original delegation, at sequence 1.
    let leaf_zero = sign_delegation(&key, &claim_at(ORIGIN, 1)).unwrap();

    // 2. The delegated key is compromised, so the operator supersedes it. The
    //    old artifact is not removed — revocation is an append.
    let mut superseding = claim_at(ORIGIN, 2);
    superseding.delegated_public_key = attacker().public_key_bytes();
    let leaf_n = sign_delegation(&key, &superseding).unwrap();

    // 3. An attacker holding NO key material re-appends the original bytes.
    //    They are public: they are in the log, which is the point of the log.
    let replayed = leaf_zero.clone();

    // Every one of the three verifies. That is not a bug — a replay of a
    // genuine artifact is a genuine artifact — and it is why the defence cannot
    // live in `verify_delegation`.
    for artifact in [&leaf_zero, &leaf_n, &replayed] {
        verify_delegation(artifact, &key.public_key_bytes(), DOMAIN, ORIGIN).unwrap();
    }

    // THE property. A replay and a legitimate re-issuance of the same claim are
    // indistinguishable byte strings, so no fold over the log can tell them
    // apart by inspecting them.
    let reissued = sign_delegation(&key, &claim_at(ORIGIN, 1)).unwrap();
    assert_eq!(
        replayed, reissued,
        "two issuances of one claim must be byte-identical — if this ever \
         stops holding, Ed25519 determinism or the encoder has changed and the \
         reasoning behind `sequence` needs revisiting"
    );

    // And the field that makes the replay a no-op rather than a rollback: the
    // replayed artifact carries a sequence already superseded, which a fold can
    // see. The ordering data is INSIDE the signed bytes, so the attacker cannot
    // alter it without the root key.
    let replayed_seq = verify_delegation(&replayed, &key.public_key_bytes(), DOMAIN, ORIGIN)
        .unwrap()
        .claim
        .sequence;
    let current_seq = verify_delegation(&leaf_n, &key.public_key_bytes(), DOMAIN, ORIGIN)
        .unwrap()
        .claim
        .sequence;
    assert!(
        replayed_seq < current_seq,
        "the replayed delegation must be recognisably superseded"
    );

    // The attacker cannot re-sequence it: bumping the number changes the signed
    // bytes, and they do not hold the root key.
    let mut forged = claim_at(ORIGIN, 3);
    forged.delegated_public_key = operational().public_key_bytes();
    let forged_bytes = sign_delegation(&attacker(), &forged).unwrap();
    assert!(
        verify_delegation(&forged_bytes, &key.public_key_bytes(), DOMAIN, ORIGIN).is_err(),
        "re-sequencing requires the root key"
    );
}

#[test]
fn sequence_is_signature_covered_and_survives_the_round_trip() {
    // A field outside the signed bytes would be re-writable by the replaying
    // attacker, which would defeat the whole point.
    let key = root();
    let mut checked = 0;
    for sequence in [
        0u64,
        1,
        23,
        24,
        255,
        256,
        65_535,
        65_536,
        1u64 << 63,
        // The largest issuable value: `u64::MAX` is refused so a successor
        // always exists.
        encoding::MAX_SEQUENCE - 1,
    ] {
        let cose = sign_delegation(&key, &claim_at(ORIGIN, sequence)).unwrap();
        let verified = verify_delegation(&cose, &key.public_key_bytes(), DOMAIN, ORIGIN).unwrap();
        assert_eq!(verified.claim.sequence, sequence);

        // Two claims differing only in `sequence` must sign different bytes.
        // Every value above is below MAX_SEQUENCE, so the successor is issuable.
        let other = sign_delegation(&key, &claim_at(ORIGIN, sequence + 1)).unwrap();
        assert_ne!(
            cose, other,
            "sequence {sequence} did not reach the signed bytes"
        );
        checked += 1;
    }
    assert_eq!(
        checked, 10,
        "every head-width boundary must have been tried"
    );
}

#[test]
fn a_sequence_with_no_successor_cannot_be_issued_or_verified() {
    // `u64::MAX` would leave the origin unable to rotate ever again, with no
    // in-band way out. Refused through every entry point, so neither the
    // convenience route nor the air-gapped two-phase route can mint one.
    let key = root();
    let root_key = key.public_key_bytes();

    let mut at_max = claim();
    at_max.sequence = u64::MAX;

    assert!(matches!(
        sign_delegation(&key, &at_max),
        Err(TrustError::DelegationEncoding { .. })
    ));
    let signature = key.sign(&delegation_preimage(&root_key, &at_max));
    assert!(matches!(
        assemble_delegation(&root_key, &at_max, &signature),
        Err(TrustError::DelegationEncoding { .. })
    ));

    // And verification refuses one built by hand and signed for real, so an
    // artifact from another implementation is refused too.
    let mutant = signed_envelope(
        &protected_for(&key),
        &encoding::payload_bytes(&at_max),
        &key,
    );
    assert!(matches!(
        verify_delegation(&mutant, &root_key, DOMAIN, ORIGIN),
        Err(TrustError::DelegationVerification)
    ));

    // Positive control at the boundary: one below is issuable and verifies, so
    // the refusal is about the maximum and not about large sequences.
    let mut at_limit = claim();
    at_limit.sequence = encoding::MAX_SEQUENCE;
    let ok = sign_delegation(&key, &at_limit).unwrap();
    assert_eq!(
        verify_delegation(&ok, &root_key, DOMAIN, ORIGIN)
            .unwrap()
            .claim
            .sequence,
        encoding::MAX_SEQUENCE
    );
}

// ---------------------------------------------------------------------------
// §2.2 — `not_before` still orders nothing.
// ---------------------------------------------------------------------------

#[test]
fn not_before_is_never_an_ordering_key() {
    // Two delegations sharing a `not_before`, and a later one bearing an earlier
    // `not_before`, are both perfectly well-formed. It is an effectivity claim;
    // `sequence` is what orders.
    let key = root();
    let mut earlier = claim();
    earlier.not_before_unix_ms = 1;
    let mut same = claim();
    same.not_before_unix_ms = 1;
    let mut zero = claim();
    zero.not_before_unix_ms = 0;
    let mut far_future = claim();
    far_future.not_before_unix_ms = u64::MAX;
    // The value an `i64` model would find wire-legal and undecodable: two
    // conforming implementations must not disagree about this artifact.
    let mut past_i64_max = claim();
    past_i64_max.not_before_unix_ms = 1u64 << 63;

    let mut accepted = 0;
    for candidate in [earlier, same, zero, far_future, past_i64_max] {
        let cose = sign_delegation(&key, &candidate).unwrap();
        let verified = verify_delegation(&cose, &key.public_key_bytes(), DOMAIN, ORIGIN).unwrap();
        assert_eq!(
            verified.claim.not_before_unix_ms,
            candidate.not_before_unix_ms
        );
        accepted += 1;
    }
    assert_eq!(accepted, 5, "every timestamp must have been exercised");
}

// ---------------------------------------------------------------------------
// §3.5.1 — non-oracle means the CALL, not just the returned value.
// ---------------------------------------------------------------------------

/// **THE test for the timing fix**: the Ed25519 verification must run exactly
/// once per call, whatever the mismatch is.
///
/// # Why this is counted rather than asserted behaviourally
///
/// **A behavioural test cannot see this defect at all.** The fix changes no
/// behaviour: an early return on a `kid` mismatch produces the same error for
/// the same inputs, so every outcome assertion in this file passes with the
/// defect present. That is measured, not supposed — restoring the early return
/// was injected as a drift and **all 61 tests stayed green**, this one included,
/// in its first and purely behavioural form. A rule whose violation changes only
/// the work done needs an instrument that observes the work.
///
/// # And the injection that "passed" without reproducing the defect
///
/// Worth recording because it nearly produced a confident false claim. The
/// first attempt at that drift injection placed the early return **after** the
/// signature check. The suite stayed green — correctly, because the expensive
/// work had already happened, so the oracle was never restored. Read carelessly
/// that looks like "the suite tolerates the defect"; read carelessly the other
/// way, after the counter was added, it would have looked like "the counter
/// catches nothing."
///
/// **That is phantom resolution: a probe that does not exercise the variable
/// under test, whose result is then attributed to the code.** The only reason it
/// surfaced is that the mutated source was read back rather than trusted. An
/// injection is evidence about a rule *only* once you have confirmed it
/// reproduces the defect; a green suite under a mis-aimed injection is evidence
/// about the injection.
///
/// Correctly placed — before the signature check — the injection fails exactly
/// this test and nothing else.
///
/// # Why not a wall-clock assertion
///
/// It is the direct measurement and the wrong tool here: flaky under load, it
/// needs a sampling design to mean anything, and a machine fast enough to blur
/// the difference would report success. The reviewer's 32.8× figure came from
/// round-robin sampling with medians and a control arm reproducing to 1.001×,
/// which belongs outside a unit suite. The counter is deterministic and answers
/// the same question.
///
/// The counter is thread-local and lives beside the verification it counts, so
/// an edit that skips the work skips the increment too.
#[test]
fn signature_verification_runs_for_every_mismatch_kind() {
    use crate::delegation::sign::SIGNATURE_VERIFICATIONS;

    let key = root();
    let expected = key.public_key_bytes();
    let count = || SIGNATURE_VERIFICATIONS.with(std::cell::Cell::get);

    // Each artifact is well-formed and parses, so every case reaches the
    // verification stage. Their signature validity differs; that is the point.
    let wrong_kid = sign_delegation(&attacker(), &claim()).unwrap();
    let wrong_origin = sign_delegation(&key, &claim_for(OTHER_ORIGIN)).unwrap();
    let mut bad_signature = sign_delegation(&key, &claim()).unwrap();
    let last = bad_signature.len() - 1;
    bad_signature[last] ^= 0x01;
    let honest = sign_delegation(&key, &claim()).unwrap();

    // ⛔ THE ARM THIS TEST WAS MISSING, and its absence is worth reading twice.
    //
    // `verify_delegation` grew a fourth comparison — the subject KIND — and this
    // test was not widened with it. Every arm above is a `Domain` artifact at a
    // `Domain` verifier, so `subject_kind_ok` was TRUE in all four: an early
    // return added to the kind comparison could only fire for an artifact none
    // of them produced, and the counter would have stayed at 1 throughout.
    // **The one field the specification flags as never having been measured for
    // leakage was the one field guarded by nothing here.**
    //
    // A valid seat delegation, genuinely signed, at a `Domain`-expecting
    // verifier: it parses, its pair is in the table, its `kid` matches and its
    // subject VALUE matches — `seat_claim_for(ORIGIN)` deliberately reuses the
    // domain fixture's string — so the kind is the only thing wrong with it, and
    // the signature check must still run.
    let seat_at_a_domain_verifier = sign_delegation(&key, &seat_claim_for(ORIGIN)).unwrap();

    let mut measured = 0;
    for (name, artifact, should_verify) in [
        ("wrong kid", wrong_kid, false),
        ("wrong subject value", wrong_origin, false),
        ("wrong subject kind", seat_at_a_domain_verifier, false),
        ("bad signature", bad_signature, false),
        ("honest", honest, true),
    ] {
        let before = count();
        let outcome = verify_delegation(&artifact, &expected, DOMAIN, ORIGIN);
        let after = count();

        assert_eq!(
            after - before,
            1,
            "{name}: the signature verification must run exactly once — a count \
             of 0 means an early return was restored and the 32.8x timing oracle \
             is back"
        );
        assert_eq!(outcome.is_ok(), should_verify, "{name}: wrong outcome");
        measured += 1;
    }
    assert_eq!(
        measured, 5,
        "every arm must have been measured — one per comparison in \
         `verify_delegation` plus the honest case. Widening that function \
         without widening this count leaves the new comparison unguarded, which \
         is exactly how the subject-kind arm came to be missing."
    );

    // The instrument's own control: a call that never reaches the verification
    // stage must NOT increment, or the counter would be measuring nothing and
    // every assertion above would pass regardless of where the check sits.
    let before = count();
    assert!(verify_delegation(b"not cbor at all", &expected, DOMAIN, ORIGIN).is_err());
    assert_eq!(
        count() - before,
        0,
        "a parse failure must not reach the signature check — if this counts, \
         the instrument is incrementing somewhere that proves nothing"
    );
}

/// The behavioural companion to the counted test above: a `kid` mismatch does
/// not cause the signature bytes to go unexamined.
///
/// Kept because it exercises a combination the counted test does not — an
/// artifact that is wrong in *two* ways at once — but note that on its own it
/// cannot detect the defect it is named for. `signature_verification_runs_for_
/// every_mismatch_kind` is the one that can.
#[test]
fn a_wrong_kid_does_not_skip_the_signature_check() {
    let key = root();
    let evil = attacker();

    // An artifact whose `kid` is the attacker's AND whose signature is garbage.
    // If the verifier returned early on the `kid` mismatch, the signature bytes
    // would never be looked at at all.
    let mut both_wrong = sign_delegation(&evil, &claim()).unwrap();
    let sig_at = both_wrong.len() - 64;
    both_wrong[sig_at..].copy_from_slice(&[0u8; 64]);

    assert!(matches!(
        verify_delegation(&both_wrong, &key.public_key_bytes(), DOMAIN, ORIGIN),
        Err(TrustError::DelegationVerification)
    ));

    // And the same artifact against the key it names is *also* refused, because
    // the signature really is invalid — which is what proves the signature arm
    // is live rather than vestigial.
    assert!(matches!(
        verify_delegation(&both_wrong, &evil.public_key_bytes(), DOMAIN, ORIGIN),
        Err(TrustError::DelegationVerification)
    ));

    // Positive control: with the signature restored, the attacker's artifact
    // verifies under the attacker's own key. So the refusals above are about
    // the checks and not about a mutant that is broken beyond recognition.
    let intact = sign_delegation(&evil, &claim()).unwrap();
    verify_delegation(&intact, &evil.public_key_bytes(), DOMAIN, ORIGIN).unwrap();
}

/// Every failure arm performs the *same* verification work, which is what makes
/// the collapsed error type an actual non-oracle rather than a cosmetic one.
///
/// An adversarial review measured a 32.8× separation between a `kid`/origin
/// mismatch and a bad signature, because the first two returned before the
/// Ed25519 verification ran. The error values were identical throughout — which
/// is the finding: **a collapsed error type is not a non-oracle if the work done
/// differs per cause.**
///
/// This test cannot measure time reliably, so it asserts the invariant that the
/// fix rests on: for every mismatch kind, the artifact is one whose signature
/// the verifier must have evaluated to reach its answer. The genuinely
/// time-based measurement is the reviewer's, and it belongs outside a unit
/// suite.
#[test]
fn the_verifier_does_the_same_work_for_every_kind_of_mismatch() {
    let key = root();
    let expected = key.public_key_bytes();

    // Three mismatches, each with a PERFECTLY VALID signature over its own
    // bytes. Under the old early-return code the first two skipped the Ed25519
    // verification entirely and the third did not; now all three run it.
    let wrong_kid = sign_delegation(&attacker(), &claim()).unwrap();
    let wrong_origin = sign_delegation(&key, &claim_for(OTHER_ORIGIN)).unwrap();
    let mut bad_signature = sign_delegation(&key, &claim()).unwrap();
    let last = bad_signature.len() - 1;
    bad_signature[last] ^= 0x01;

    let mut refused = 0;
    for (name, mutant, valid_signature) in [
        ("wrong kid", wrong_kid, true),
        ("wrong origin", wrong_origin, true),
        ("bad signature", bad_signature, false),
    ] {
        // Each mutant's own signature status, asserted rather than assumed, so
        // the three arms really are the three different causes.
        let parsed = AnchorDelegation::from_cose_bytes(&mutant).unwrap();
        let preimage = delegation_preimage(&parsed.root_public_key, &parsed.claim);
        let signature_ok =
            Ed25519Identity::verify(&parsed.root_public_key, &preimage, &parsed.signature).is_ok();
        assert_eq!(
            signature_ok, valid_signature,
            "{name} does not have the signature status this test assumes"
        );

        assert!(matches!(
            verify_delegation(&mutant, &expected, DOMAIN, ORIGIN),
            Err(TrustError::DelegationVerification)
        ));
        refused += 1;
    }
    assert_eq!(refused, 3, "every mismatch kind must have been tried");
}

// ---------------------------------------------------------------------------
// The encode side refuses everything the decode side refuses.
// ---------------------------------------------------------------------------

#[test]
fn issuance_refuses_a_claim_the_verifier_would_reject() {
    // The defect: the artifact cap was enforced at decode only, so a subject
    // value of 3884 bytes signed and verified while 3885 signed SUCCESSFULLY and failed
    // every verification afterwards — a file that fails at a later, far less
    // debuggable moment, which is precisely what assembling-with-verification
    // exists to prevent, arriving through the one door that check did not cover.
    //
    // The boundary is DERIVED here, never hardcoded: it moved by nine bytes when
    // `sequence` was added, and a literal would have gone stale in silence.
    let key = root();
    let root_key = key.public_key_bytes();

    let mut longest_ok = None;
    for len in (1..4000usize).rev() {
        if sign_delegation(&key, &claim_at(&"o".repeat(len), 300)).is_ok() {
            longest_ok = Some(len);
            break;
        }
    }
    let longest_ok = longest_ok.expect("some subject length must be issuable");

    // At the boundary: issuable AND verifiable. The pair is the point — either
    // one alone is what the defect looked like.
    let at_limit = claim_at(&"o".repeat(longest_ok), 300);
    let artifact = sign_delegation(&key, &at_limit).unwrap();
    verify_delegation(&artifact, &root_key, DOMAIN, &at_limit.subject_value).unwrap();

    // One byte over: refused at ISSUANCE, with an actionable reason rather than
    // the non-oracle value. This is the operator's side; no stranger is being
    // kept in the dark.
    let over = claim_at(&"o".repeat(longest_ok + 1), 300);
    let err = sign_delegation(&key, &over).unwrap_err();
    let TrustError::DelegationEncoding { reason } = &err else {
        panic!("expected an encoding error naming the constraint, got {err:?}");
    };
    assert!(
        reason.contains("cap"),
        "the reason must name the constraint, got: {reason}"
    );

    // And the two-phase path refuses it too, so the air-gapped route cannot
    // produce what the convenience route refuses.
    let signature = key.sign(&delegation_preimage(&root_key, &over));
    assert!(matches!(
        assemble_delegation(&root_key, &over, &signature),
        Err(TrustError::DelegationEncoding { .. })
    ));
}

#[test]
fn issuance_refuses_an_empty_subject_value_and_an_unusable_delegated_key() {
    let key = root();
    let root_key = key.public_key_bytes();

    let empty = claim_at("", 300);
    assert!(matches!(
        sign_delegation(&key, &empty),
        Err(TrustError::DelegationEncoding { .. })
    ));

    // All-zeros, all-0xff and the canonically-encoded identity point: three keys
    // strict Ed25519 verification can never accept, so a delegation naming one
    // could never authorise anything. Each was accepted before this check.
    let mut identity_point = [0u8; 32];
    identity_point[0] = 1;
    let mut refused = 0;
    for bad_key in [[0u8; 32], [0xffu8; 32], identity_point] {
        let mut c = claim();
        c.delegated_public_key = bad_key;
        assert!(
            matches!(
                sign_delegation(&key, &c),
                Err(TrustError::DelegationEncoding { .. })
            ),
            "an unusable delegated key was issuable"
        );
        // The two-phase path too.
        let signature = key.sign(&delegation_preimage(&root_key, &c));
        assert!(matches!(
            assemble_delegation(&root_key, &c, &signature),
            Err(TrustError::DelegationEncoding { .. })
        ));
        refused += 1;
    }
    assert_eq!(refused, 3, "every unusable key shape must have been tried");

    // Positive control: the real operational key issues.
    sign_delegation(&key, &claim()).unwrap();
}

#[test]
fn an_unusable_delegated_key_is_refused_at_verification_too() {
    // Hand-assembled and signed for real, because the encoder now refuses to
    // build one. A delegation naming a key that can never verify anything is a
    // signed statement with no possible meaning — the same failure the closed
    // role enum refuses, in a different field.
    let key = root();
    let canonical = encoding::payload_bytes(&claim());
    let delegated = operational().public_key_bytes();
    let at = canonical
        .windows(32)
        .position(|w| w == delegated)
        .expect("the delegated key is in the payload");

    let mut identity_point = [0u8; 32];
    identity_point[0] = 1;
    let mut refused = 0;
    for bad_key in [[0u8; 32], [0xffu8; 32], identity_point] {
        let mut payload = canonical.clone();
        payload[at..at + 32].copy_from_slice(&bad_key);
        let mutant = signed_envelope(&protected_for(&key), &payload, &key);
        assert!(
            matches!(
                verify_delegation(&mutant, &key.public_key_bytes(), DOMAIN, ORIGIN),
                Err(TrustError::DelegationVerification)
            ),
            "a delegation naming an unusable key was accepted"
        );
        refused += 1;
    }
    assert_eq!(refused, 3, "every unusable key shape must have been tried");

    // Positive control: the same hand-assembly with the real key verifies.
    let ok = signed_envelope(&protected_for(&key), &canonical, &key);
    verify_delegation(&ok, &key.public_key_bytes(), DOMAIN, ORIGIN).unwrap();
}

#[test]
fn an_empty_subject_value_is_refused_at_verification_too() {
    // The rule is about the VERIFIER, not the string: acceptance is a comparison
    // against the caller's configured subject, so a verifier whose subject is
    // unset would match an empty-subject delegation and accept it. Refusing at
    // decode makes that misconfiguration fail closed.
    let key = root();
    let mut payload = vec![0xa6, 0x01, 0x01, 0x02, 0x60, 0x03, 0x58, 0x20];
    payload.extend_from_slice(&operational().public_key_bytes());
    payload.extend_from_slice(&[0x04, 0x02, 0x05, 0x1b]);
    payload.extend_from_slice(&1_700_000_000_000u64.to_be_bytes());
    payload.extend_from_slice(&[0x06, 0x19, 0x01, 0x2c]);

    let mutant = signed_envelope(&protected_for(&key), &payload, &key);

    // The case that motivates the rule: a verifier that passes "" as its
    // expected subject value. Without the decode rule this would SUCCEED.
    assert!(matches!(
        verify_delegation(&mutant, &key.public_key_bytes(), DOMAIN, ""),
        Err(TrustError::DelegationVerification)
    ));
    // And with a real origin configured, naturally.
    assert!(verify_delegation(&mutant, &key.public_key_bytes(), DOMAIN, ORIGIN).is_err());
}

// ---------------------------------------------------------------------------
// §3.5 — non-oracle failure.
// ---------------------------------------------------------------------------

#[test]
fn every_verification_failure_is_the_same_error() {
    let key = root();
    let expected = key.public_key_bytes();
    let good = sign_delegation(&key, &claim()).unwrap();

    // Positive control first: a verifier that refused everything would satisfy
    // every assertion below.
    verify_delegation(&good, &expected, DOMAIN, ORIGIN).unwrap();

    let wrong_root = sign_delegation(&attacker(), &claim()).unwrap();
    let wrong_origin = sign_delegation(&key, &claim_for(OTHER_ORIGIN)).unwrap();

    let mut bad_signature = good.clone();
    let last = bad_signature.len() - 1;
    bad_signature[last] ^= 0x01;

    let relabelled = signed_envelope(
        &encoding::protected_bytes("application/vnd.lys.receipt.v1+cbor", &expected),
        &encoding::payload_bytes(&claim()),
        &key,
    );

    let canonical_payload = encoding::payload_bytes(&claim());
    let role_at = role_offset(&canonical_payload);
    let mut unknown_role_payload = canonical_payload.clone();
    unknown_role_payload[role_at + 1] = 7;
    let unknown_role = signed_envelope(&protected_for(&key), &unknown_role_payload, &key);

    // A pair every field of which is individually valid: domain + speaks-for.
    let mut invalid_pair_payload = canonical_payload;
    invalid_pair_payload[role_at + 1] = 0x03;
    let invalid_pair = signed_envelope(&protected_for(&key), &invalid_pair_payload, &key);

    // A seat delegation, genuinely signed, presented to a domain verifier that
    // names the same subject string.
    let wrong_kind = sign_delegation(&key, &seat_claim_for(ORIGIN)).unwrap();

    let unprotected_at = 4 + encoding::PROTECTED_LEN;
    let mut non_empty_unprotected = good[..unprotected_at].to_vec();
    non_empty_unprotected.extend_from_slice(&[0xa1, 0x01, 0x01]);
    non_empty_unprotected.extend_from_slice(&good[unprotected_at + 1..]);

    let mut trailing = good;
    trailing.push(0x00);

    let cases = [
        ("wrong root key", wrong_root),
        ("wrong subject value", wrong_origin),
        ("forged signature", bad_signature),
        ("relabelled as a receipt", relabelled),
        ("unknown role", unknown_role),
        ("invalid (kind, role) pair", invalid_pair),
        ("wrong subject kind", wrong_kind),
        ("non-empty unprotected", non_empty_unprotected),
        ("trailing garbage", trailing),
        ("empty input", vec![]),
    ];

    let mut refused = 0;
    for (name, mutant) in cases {
        let err = verify_delegation(&mutant, &expected, DOMAIN, ORIGIN).unwrap_err();
        assert!(
            matches!(err, TrustError::DelegationVerification),
            "{name} produced a distinguishable error: {err:?}"
        );
        assert_eq!(
            format!("{err}"),
            format!("{}", TrustError::DelegationVerification),
            "{name} produced a distinguishable message"
        );
        refused += 1;
    }
    assert_eq!(refused, 10, "every case must have been rejected");
}
