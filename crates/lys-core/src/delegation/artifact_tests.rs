#![cfg(test)]
//! Round-trip and canonical-encoding-strictness tests for
//! [`Delegation`].
//!
//! The mutants here all have or could have **cryptographically valid
//! signatures** — several are re-encodings of a genuine delegation rather than
//! forgeries, and a vanilla COSE verifier accepts them. lys rejects them because
//! a delegation with two valid encodings is a delegation whose bytes are not its
//! identity, and every downstream comparison (log dedup, a cache key, "is this
//! the same statement I saw yesterday") then depends on which encoding you
//! happened to receive.
//!
//! Signature-bearing versions of the security rules live in `sign_tests`, where
//! they are signed for real; this file is about the parser.
//!
//! # The malleability sweep, and what it does and does not establish
//!
//! [`the_envelope_malleability_sweep_is_refused`] walks every non-canonical
//! **envelope** spelling the suite knows how to build. That is the byte-compare's
//! own territory, because no signature covers the envelope under any verifier's
//! strategy — and at this parse-only entry point the byte-compare is the *only*
//! canonicality guard, since no signature is checked here at all.
//!
//! **The sweep is a lower bound on what the check guards, not an inventory of
//! it.** CBOR admits more non-canonical spellings than any suite enumerates,
//! which is exactly why the rule is "byte-identical to the canonical
//! re-encoding" rather than a list of rejected forms. An earlier note claimed
//! "exactly four artifacts flip to accepted" when the check is removed; that
//! was the set the suite happened to cover, stated as though it were the set
//! that exists.

use super::*;
use crate::delegation::artifact::DelegationSubjectKind;
use crate::delegation::encoding::{CONTENT_TYPE, MAX_SEQUENCE, PROTECTED_LEN};

#[path = "artifact_tests/refusals.rs"]
mod refusals;
#[path = "artifact_tests/round_trip.rs"]
mod round_trip;

const ROOT_KEY: [u8; KEY_LEN] = [0xa1; KEY_LEN];
const SIGNATURE: [u8; 64] = [0x5e; 64];

/// Offset of the protected bucket's first byte inside the artifact: the four
/// bytes `d2 84 58 4f` precede it.
const PROTECTED_AT: usize = 4;
/// Offset of the unprotected bucket byte.
const UNPROTECTED_AT: usize = PROTECTED_AT + PROTECTED_LEN;

/// A real Ed25519 public key for the delegated slot. It must be real: the
/// decoder now validates it as a point strict verification could accept.
fn delegated_key() -> [u8; KEY_LEN] {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("delegated.key");
    std::fs::write(&path, [0x42u8; 32]).unwrap();
    crate::Ed25519Identity::load(&path)
        .unwrap()
        .public_key_bytes()
}

fn claim_for(subject_value: &str, not_before_unix_ms: u64, sequence: u64) -> DelegationClaim {
    DelegationClaim {
        subject_kind: DelegationSubjectKind::Domain,
        subject_value: subject_value.to_string(),
        delegated_public_key: delegated_key(),
        role: DelegationRole::Operational,
        not_before_unix_ms,
        sequence,
    }
}

/// The `(seat, speaks-for)` sample, so the round trip is exercised on both valid
/// pairs rather than only on the one an anchor happens to use.
fn seat_sample() -> Delegation {
    Delegation {
        root_public_key: ROOT_KEY,
        claim: DelegationClaim {
            subject_kind: DelegationSubjectKind::Seat,
            subject_value: "a-seat-identifier".to_string(),
            delegated_public_key: delegated_key(),
            role: DelegationRole::SpeaksFor,
            not_before_unix_ms: 1_700_000_000_000,
            sequence: 300,
        },
        signature: SIGNATURE,
    }
}

fn sample() -> Delegation {
    Delegation {
        root_public_key: ROOT_KEY,
        claim: claim_for("example.test", 1_700_000_000_000, 300),
        signature: SIGNATURE,
    }
}

/// Rebuild the artifact around a deliberately altered payload, so the payload
/// bstr head stays correct and the *canonicality* rule is what rejects the
/// mutant rather than a malformed document.
fn artifact_with_payload(payload: &[u8]) -> Vec<u8> {
    encoding::envelope_bytes(
        &encoding::protected_bytes(CONTENT_TYPE, &ROOT_KEY),
        payload,
        &SIGNATURE,
    )
}
