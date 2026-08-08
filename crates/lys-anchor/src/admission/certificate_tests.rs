#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Gates on [`RecognisedCertificate`].
//!
//! # Where the second party comes from
//!
//! - **A certificate authority this file did not write.** Every certificate
//!   here is issued by `lys-core`'s [`CertificateAuthority`], and the policy is
//!   configured from a *public key* rather than from anything the issuance
//!   handed back for convenience. A policy that recognised certificates by some
//!   property other than the issuer's signature passes nothing below.
//! - **A second authority.** The refusal case is not a mangled certificate; it
//!   is a perfectly valid certificate signed by a different key. That
//!   distinguishes "this policy checks the chain" from "this policy checks that
//!   the bytes parse".
//! - **The passage of time.** One case waits for a certificate to expire rather
//!   than asserting that expiry is handled. It is the only check here that can
//!   tell a chain verification that consults the validity window from one that
//!   does not.
//! - **The written contract on provenance.** [`RecognisedCertificate`]'s docs
//!   say it admits from *both* arms of [`SubmitterContext`] and treats them
//!   identically. [`both_provenances_are_admitted_identically`] holds the code
//!   to that sentence, in both directions — an implementation that quietly
//!   required authentication, and one that quietly ignored the authenticated
//!   arm, each fail it.
//!
//! # Two tests assert weaknesses on purpose
//!
//! [`a_copied_certificate_is_admitted_because_presenting_is_not_proving`] and
//! the asserted half of [`both_provenances_are_admitted_identically`] pin the
//! limitation the module docs describe. They are not endorsements: they exist
//! so that the day somebody closes the gap, a test fails and points at the
//! prose explaining what closing it actually requires — a transport that
//! authenticates the peer, or a submission-authentication wire format that is
//! deliberately not started. A caveat that lives only in a comment is one
//! nobody is forced to read.

use std::collections::BTreeSet;
use std::time::Duration;

use lys_core::Ed25519Identity;
use lys_core::ca::CertificateAuthority;
use tempfile::TempDir;

use super::*;
use crate::admission::{AuthenticatedPeer, MaxSize};
use crate::wire::Submission;

/// One hour, long enough that no test here races its own certificate.
const LONG_TTL: Duration = Duration::from_secs(3600);

/// The statement every case submits. This policy never reads it, which is why
/// one value serves them all.
const STATEMENT: &[u8] = b"a statement";

/// An authority over a deterministic seed, so no test depends on generated key
/// material.
fn authority(seed: &[u8; 32]) -> (CertificateAuthority, TempDir) {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("ca.key");
    std::fs::write(&path, seed).unwrap();
    let identity = Ed25519Identity::load(&path).unwrap();
    (CertificateAuthority::new(identity), tmp)
}

/// The submission every case makes.
fn statement() -> Submission<'static> {
    Submission {
        statement: STATEMENT,
    }
}

/// The submitter sent `credential` themselves, and nobody checked they hold
/// anything it names.
fn asserted(credential: &[u8]) -> SubmitterContext<'_> {
    SubmitterContext::AssertedBySubmitter(credential)
}

/// A transport authenticated the peer against `credential`.
fn authenticated(credential: &[u8]) -> SubmitterContext<'_> {
    SubmitterContext::AuthenticatedByTransport(AuthenticatedPeer::verified_by_transport(credential))
}

#[test]
fn a_certificate_from_the_configured_authority_is_admitted() {
    let (ca, _tmp) = authority(b"lys-anchor-admission-ca-seed-01!");
    let issued = ca
        .issue_certificate("submitter-a", LONG_TTL, vec![])
        .unwrap();

    // Configured from the authority's public key, not from anything issuance
    // returned alongside the certificate.
    let policy = RecognisedCertificate::issued_by(ca.public_key_bytes());
    assert_eq!(
        policy.admit(&statement(), &asserted(&issued.der_bytes)),
        Ok(())
    );
}

#[test]
fn both_provenances_are_admitted_identically() {
    let (ca, _tmp) = authority(b"lys-anchor-admission-ca-seed-12!");
    let issued = ca
        .issue_certificate("submitter-a", LONG_TTL, vec![])
        .unwrap();
    let policy = RecognisedCertificate::issued_by(ca.public_key_bytes());

    // The documented behaviour, in both directions. An implementation that
    // required authentication fails the first; one that only ever looked at the
    // asserted arm fails the second. A single arm could not catch either.
    assert_eq!(
        policy.admit(&statement(), &asserted(&issued.der_bytes)),
        Ok(()),
        "a submitter-asserted certificate must be admitted — the policy is \
         named for recognition, not authentication"
    );
    assert_eq!(
        policy.admit(&statement(), &authenticated(&issued.der_bytes)),
        Ok(()),
        "a transport-authenticated peer's certificate must be admitted"
    );

    // And a refusal is a refusal on both arms too, so the equal treatment is
    // not "admits everything on one of them".
    let (other, _other_tmp) = authority(b"lys-anchor-admission-ca-seed-13!");
    let foreign = other
        .issue_certificate("submitter-a", LONG_TTL, vec![])
        .unwrap();
    assert_eq!(
        policy.admit(&statement(), &asserted(&foreign.der_bytes)),
        Err(NotAdmitted)
    );
    assert_eq!(
        policy.admit(&statement(), &authenticated(&foreign.der_bytes)),
        Err(NotAdmitted),
        "authentication by a transport must not substitute for the chain check"
    );
}

#[test]
fn a_submission_with_no_credential_at_all_is_refused() {
    let (ca, _tmp) = authority(b"lys-anchor-admission-ca-seed-02!");
    let policy = RecognisedCertificate::issued_by(ca.public_key_bytes());
    assert_eq!(
        policy.admit(&statement(), &SubmitterContext::Unidentified),
        Err(NotAdmitted)
    );
}

#[test]
fn a_certificate_from_another_authority_is_refused() {
    let (ours, _ours_tmp) = authority(b"lys-anchor-admission-ca-seed-03!");
    let (theirs, _theirs_tmp) = authority(b"lys-anchor-admission-ca-seed-04!");
    assert_ne!(
        ours.public_key_bytes(),
        theirs.public_key_bytes(),
        "the two fixture authorities must actually differ"
    );

    let foreign = theirs
        .issue_certificate("submitter-a", LONG_TTL, vec![])
        .unwrap();
    let policy = RecognisedCertificate::issued_by(ours.public_key_bytes());

    // A valid certificate, correctly formed, signed by the wrong key.
    assert_eq!(
        policy.admit(&statement(), &asserted(&foreign.der_bytes)),
        Err(NotAdmitted)
    );

    // Positive control: the same policy admits the same shape of certificate
    // when it comes from the right authority, so the refusal above is about the
    // issuer and not about the fixture being unusable.
    let ours_cert = ours
        .issue_certificate("submitter-a", LONG_TTL, vec![])
        .unwrap();
    assert_eq!(
        policy.admit(&statement(), &asserted(&ours_cert.der_bytes)),
        Ok(())
    );
}

#[test]
fn an_expired_certificate_is_refused() {
    let (ca, _tmp) = authority(b"lys-anchor-admission-ca-seed-05!");
    let short_lived = ca
        .issue_certificate("submitter-a", Duration::from_secs(1), vec![])
        .unwrap();
    let policy = RecognisedCertificate::issued_by(ca.public_key_bytes());

    // Positive control first: it is admitted while it is valid, so the refusal
    // below is caused by the wait and not by the certificate never having
    // worked.
    assert_eq!(
        policy.admit(&statement(), &asserted(&short_lived.der_bytes)),
        Ok(())
    );

    // `notAfter` is truncated to whole seconds at issuance, so two seconds is
    // unambiguously past it. This is the only check here that can distinguish a
    // chain verification that consults the validity window from one that does
    // not.
    std::thread::sleep(Duration::from_millis(2100));

    assert_eq!(
        policy.admit(&statement(), &asserted(&short_lived.der_bytes)),
        Err(NotAdmitted)
    );
}

#[test]
fn a_credential_that_is_not_a_certificate_is_refused() {
    let (ca, _tmp) = authority(b"lys-anchor-admission-ca-seed-06!");
    let policy = RecognisedCertificate::issued_by(ca.public_key_bytes());

    let mut refused = 0;
    let garbage: [&[u8]; 3] = [b"", b"not DER at all", &[0x30, 0x82, 0xff, 0xff]];
    for credential in garbage {
        assert_eq!(
            policy.admit(&statement(), &asserted(credential)),
            Err(NotAdmitted)
        );
        // And a transport asserting it authenticated a peer against garbage
        // does not make garbage a certificate.
        assert_eq!(
            policy.admit(&statement(), &authenticated(credential)),
            Err(NotAdmitted)
        );
        refused += 1;
    }
    assert_eq!(
        refused, 3,
        "every malformed credential must have been tried"
    );
}

#[test]
fn the_subject_allow_list_admits_a_listed_key_and_refuses_an_unlisted_one() {
    let (ca, _tmp) = authority(b"lys-anchor-admission-ca-seed-07!");
    let listed = ca
        .issue_certificate("submitter-a", LONG_TTL, vec![])
        .unwrap();
    let unlisted = ca
        .issue_certificate("submitter-b", LONG_TTL, vec![])
        .unwrap();
    assert_ne!(
        listed.subject_verifying_key.to_bytes(),
        unlisted.subject_verifying_key.to_bytes(),
        "the two fixture subjects must actually differ"
    );

    let mut allowed = BTreeSet::new();
    allowed.insert(listed.subject_verifying_key.to_bytes());
    let policy = RecognisedCertificate::issued_by_to(ca.public_key_bytes(), allowed);

    assert_eq!(
        policy.admit(&statement(), &asserted(&listed.der_bytes)),
        Ok(())
    );
    // Same authority, same validity, same shape — refused on subject identity
    // alone.
    assert_eq!(
        policy.admit(&statement(), &asserted(&unlisted.der_bytes)),
        Err(NotAdmitted)
    );

    assert_eq!(policy.subject_keys().map(BTreeSet::len), Some(1));
    assert_eq!(policy.issuer_public_key(), &ca.public_key_bytes());
}

#[test]
fn an_empty_allow_list_admits_nothing() {
    let (ca, _tmp) = authority(b"lys-anchor-admission-ca-seed-08!");
    let issued = ca
        .issue_certificate("submitter-a", LONG_TTL, vec![])
        .unwrap();

    // Positive control: unrestricted, this exact certificate is admitted. So
    // the refusal below is the empty set doing its job, not a broken fixture.
    let unrestricted = RecognisedCertificate::issued_by(ca.public_key_bytes());
    assert_eq!(
        unrestricted.admit(&statement(), &asserted(&issued.der_bytes)),
        Ok(())
    );

    let restricted = RecognisedCertificate::issued_by_to(ca.public_key_bytes(), BTreeSet::new());
    assert_eq!(
        restricted.admit(&statement(), &asserted(&issued.der_bytes)),
        Err(NotAdmitted)
    );
}

#[test]
fn a_copied_certificate_is_admitted_because_presenting_is_not_proving() {
    let (ca, _tmp) = authority(b"lys-anchor-admission-ca-seed-09!");
    let alice = ca.issue_certificate("alice", LONG_TTL, vec![]).unwrap();

    let mut allowed = BTreeSet::new();
    allowed.insert(alice.subject_verifying_key.to_bytes());
    let policy = RecognisedCertificate::issued_by_to(ca.public_key_bytes(), allowed);

    // Mallory holds no key of Alice's — only a copy of her certificate, which
    // is a public artifact anybody who has seen one submission has. She is
    // admitted, and this assertion says so out loud.
    let copied: Vec<u8> = alice.der_bytes.clone();
    assert_eq!(
        policy.admit(&statement(), &asserted(&copied)),
        Ok(()),
        "this policy recognises a credential; it does not authenticate a presenter"
    );

    // The copy really is a copy — the test is not accidentally reusing the
    // original value under another name.
    assert_eq!(copied, alice.der_bytes);
}

#[test]
fn every_refusal_is_the_same_value_whatever_tripped_it() {
    let (ours, _ours_tmp) = authority(b"lys-anchor-admission-ca-seed-10!");
    let (theirs, _theirs_tmp) = authority(b"lys-anchor-admission-ca-seed-11!");
    let foreign = theirs
        .issue_certificate("submitter-a", LONG_TTL, vec![])
        .unwrap();
    let unlisted = ours
        .issue_certificate("submitter-b", LONG_TTL, vec![])
        .unwrap();

    let cert_policy = RecognisedCertificate::issued_by_to(ours.public_key_bytes(), BTreeSet::new());
    let size_policy = MaxSize::new(0);

    let long = vec![b'x'; 64];
    let long_submission = Submission { statement: &long };
    let refusals: Vec<NotAdmitted> = vec![
        // Nothing established about the submitter at all.
        cert_policy
            .admit(&statement(), &SubmitterContext::Unidentified)
            .unwrap_err(),
        // A credential that is not a certificate.
        cert_policy
            .admit(&statement(), &asserted(b"not DER at all"))
            .unwrap_err(),
        // A certificate from another authority.
        cert_policy
            .admit(&statement(), &asserted(&foreign.der_bytes))
            .unwrap_err(),
        // A valid certificate from this authority, subject not on the list.
        cert_policy
            .admit(&statement(), &asserted(&unlisted.der_bytes))
            .unwrap_err(),
        // The same, but reached through the authenticated arm: the provenance
        // must not change the refusal either.
        cert_policy
            .admit(&statement(), &authenticated(&unlisted.der_bytes))
            .unwrap_err(),
        // A different policy entirely, tripped by length.
        size_policy
            .admit(&long_submission, &SubmitterContext::Unidentified)
            .unwrap_err(),
    ];

    // Six refusals from six distinct routes, across two policies and both
    // provenances: a submitter cannot tell which rule refused, or which policy
    // the anchor even runs.
    assert_eq!(refusals.len(), 6, "every refusal case must have been tried");
    let mut compared = 0;
    for refusal in &refusals {
        assert_eq!(refusal, &refusals[0]);
        assert_eq!(format!("{refusal:?}"), format!("{:?}", refusals[0]));
        compared += 1;
    }
    assert_eq!(compared, 6);

    // Positive control: this comparison can fail. `NotAdmitted` is a
    // zero-sized type, so equality between two of its values is trivially
    // true — the loop above would pass over a list of anything. The check that
    // it is comparing something with a *distinguishable* alternative is that
    // an admitted submission is not equal to a refused one, under the same
    // machinery.
    let admitted: Result<(), NotAdmitted> =
        RecognisedCertificate::issued_by(ours.public_key_bytes())
            .admit(&statement(), &asserted(&unlisted.der_bytes));
    assert_eq!(admitted, Ok(()));
    assert_ne!(admitted, Err(refusals[0]));
    assert_ne!(
        format!("{admitted:?}"),
        format!("{:?}", Err::<(), _>(refusals[0]))
    );
}
