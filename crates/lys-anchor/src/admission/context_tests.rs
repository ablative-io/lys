#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Gates on [`SubmitterContext`] and [`AuthenticatedPeer`].
//!
//! # What can and cannot be checked here, stated before the checks
//!
//! The property this type exists for — *only something that authenticated a
//! peer builds an [`AuthenticatedPeer`]* — **is not testable and is not
//! enforced.** Rust visibility cannot ask who is calling, and nothing in this
//! crate observes a handshake. Any test claiming to verify it would be
//! measuring its own fixture.
//!
//! So the second party here is narrower and honest about it:
//!
//! - **The compiler**, for the one mechanical part that *is* enforced: the
//!   field is private, so the struct literal does not compile and the named
//!   constructor is the only route. That is a `compile_fail` doctest on the
//!   type itself, with a positive control, in the shape this repo uses.
//! - **Distinguishability**, checked rather than assumed: two contexts built
//!   over the *same bytes* through the two different routes must not be equal.
//!   If they compared equal, the split would be decoration — a policy could not
//!   tell them apart even if it wanted to, and the type would be the
//!   looks-checked defect it was written to prevent.
//! - **Exhaustiveness**, checked by a `match` with no `_` arm, written here so
//!   that adding a variant breaks this file too rather than only breaking
//!   downstream code nobody in this repo compiles.

use super::*;

/// The same certificate bytes, reached two ways.
const CREDENTIAL: &[u8] = b"the same bytes, whatever established them";

#[test]
fn the_two_provenances_are_distinguishable_over_identical_bytes() {
    let asserted = SubmitterContext::AssertedBySubmitter(CREDENTIAL);
    let authenticated = SubmitterContext::AuthenticatedByTransport(
        AuthenticatedPeer::verified_by_transport(CREDENTIAL),
    );

    // Keyed on the bytes being identical — otherwise the inequality below could
    // be about the payload rather than about the provenance, which is the only
    // thing this type adds.
    assert_eq!(
        match asserted {
            SubmitterContext::AssertedBySubmitter(bytes) => bytes,
            SubmitterContext::AuthenticatedByTransport(peer) => peer.certificate(),
            SubmitterContext::Unidentified => b"",
        },
        CREDENTIAL
    );
    assert_eq!(
        match authenticated {
            SubmitterContext::AssertedBySubmitter(bytes) => bytes,
            SubmitterContext::AuthenticatedByTransport(peer) => peer.certificate(),
            SubmitterContext::Unidentified => b"",
        },
        CREDENTIAL
    );

    assert_ne!(
        asserted, authenticated,
        "the two provenances must not compare equal over the same bytes, or the \
         split is decoration"
    );
    assert_ne!(format!("{asserted:?}"), format!("{authenticated:?}"));
    assert_ne!(
        std::mem::discriminant(&asserted),
        std::mem::discriminant(&authenticated)
    );

    // Positive control: the comparisons above are not blind. The same
    // machinery reports equality when the values really are the same.
    assert_eq!(asserted, SubmitterContext::AssertedBySubmitter(CREDENTIAL));
    assert_eq!(
        std::mem::discriminant(&asserted),
        std::mem::discriminant(&SubmitterContext::AssertedBySubmitter(b"other bytes"))
    );
}

#[test]
fn unidentified_is_neither_of_the_credential_bearing_arms() {
    let nobody = SubmitterContext::Unidentified;
    assert_ne!(nobody, SubmitterContext::AssertedBySubmitter(CREDENTIAL));
    assert_ne!(
        nobody,
        SubmitterContext::AuthenticatedByTransport(AuthenticatedPeer::verified_by_transport(
            CREDENTIAL
        ))
    );
}

#[test]
fn every_variant_is_reachable_and_the_match_is_exhaustive() {
    // No `_` arm anywhere in this test. A fourth variant makes this file stop
    // compiling, which is the friction the enum's docs promise and the only
    // place in this repo that would notice.
    let cases = [
        SubmitterContext::Unidentified,
        SubmitterContext::AssertedBySubmitter(CREDENTIAL),
        SubmitterContext::AuthenticatedByTransport(AuthenticatedPeer::verified_by_transport(
            CREDENTIAL,
        )),
    ];

    let mut seen_unidentified = 0;
    let mut seen_asserted = 0;
    let mut seen_authenticated = 0;
    for case in cases {
        match case {
            SubmitterContext::Unidentified => seen_unidentified += 1,
            SubmitterContext::AssertedBySubmitter(bytes) => {
                assert_eq!(bytes, CREDENTIAL);
                seen_asserted += 1;
            }
            SubmitterContext::AuthenticatedByTransport(peer) => {
                assert_eq!(peer.certificate(), CREDENTIAL);
                seen_authenticated += 1;
            }
        }
    }
    // Count what fired: three arms, each exactly once. A sweep that missed one
    // would satisfy every assertion inside the loop.
    assert_eq!(
        (seen_unidentified, seen_asserted, seen_authenticated),
        (1, 1, 1)
    );
}

#[test]
fn the_certificate_comes_back_out_exactly_as_it_went_in() {
    // Not a round trip through an encoder — there is none — but a check that
    // the constructor stores rather than copies, trims or reinterprets. The
    // bytes are deliberately not valid DER: this type imposes no format, and a
    // constructor that had started validating would fail here.
    let odd: &[u8] = &[0xff, 0x00, 0x0a, 0xc3, 0x28];
    let peer = AuthenticatedPeer::verified_by_transport(odd);
    assert_eq!(peer.certificate(), odd);
}
