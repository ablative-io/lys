#![cfg(test)]
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

use super::*;
use crate::delegation::artifact::{DelegationRole, DelegationSubjectKind};
use crate::merkle::tree::{AppendOnlyTree, RawLeaf};
use crate::receipt;

#[path = "sign_tests/envelope.rs"]
mod envelope;
#[path = "sign_tests/issuance.rs"]
mod issuance;
#[path = "sign_tests/sequence.rs"]
mod sequence;
#[path = "sign_tests/subject.rs"]
mod subject;
#[path = "sign_tests/uniform_failure.rs"]
mod uniform_failure;

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
