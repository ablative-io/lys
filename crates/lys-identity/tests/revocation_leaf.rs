//! R2: the three certificate-log leaves encode and decode, malformed bytes are
//! refused whole, and a revocation verifies only under the issuing authority's
//! key for its own log's origin.

mod revocation_support;

use std::error::Error;
use std::fmt::Debug;

use lys_core::attestation::sign_attestation;
use lys_identity::revocation::{
    CertificateHash, CertificateLeaf, LEAF_DOMAIN_TAG, REVOCATION_LEAF_LEN, Revocation,
    RevocationError,
};
use revocation_support::fixtures::identity;
use revocation_support::{Issuer, TestResult, issue, new_issuer, open_log, seed};
use tempfile::TempDir;

/// The lowercase hex of `bytes`.
fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        out.push(char::from(DIGITS[usize::from(byte >> 4)]));
        out.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    out
}

/// The 33-byte header of a leaf of `kind`.
fn header(kind: u8) -> Vec<u8> {
    let mut bytes = LEAF_DOMAIN_TAG.to_vec();
    bytes.push(0);
    bytes.push(kind);
    bytes
}

/// A header followed by `body_len` filler bytes.
fn leaf_of(kind: u8, body_len: usize) -> Vec<u8> {
    let mut bytes = header(kind);
    bytes.extend(std::iter::repeat_n(0x5a, body_len));
    bytes
}

/// The seven malformed inputs of leg 2: a wrong tag, a missing zero byte, kind
/// byte 0x04, an issuance leaf ending at its kind byte, revocation leaves of
/// 128 and 130 bytes, and a revocation leaf carrying an 8-byte claim selector.
fn malformed_inputs() -> [Vec<u8>; 7] {
    let mut wrong_tag = leaf_of(0x02, 96);
    wrong_tag[30] = b'2';
    let mut no_zero = leaf_of(0x02, 96);
    no_zero[31] = 0x01;
    [
        wrong_tag,
        no_zero,
        leaf_of(0x04, 40),
        header(0x01),
        leaf_of(0x02, 95),
        leaf_of(0x02, 97),
        leaf_of(0x02, 104),
    ]
}

/// Both Debug renderings of `value`.
fn render<T: Debug>(renderings: &mut Vec<String>, value: &T) {
    renderings.push(format!("{value:?}"));
    renderings.push(format!("{value:#?}"));
}

/// An issuer, two certificates it issued and a log whose origin the
/// revocations are signed for.
struct Fixture {
    dir: TempDir,
    issuer: Issuer,
    seed: [u8; 32],
    a_der: Vec<u8>,
    b_der: Vec<u8>,
    a_cose: Vec<u8>,
    origin: String,
}

fn fixture() -> Result<Fixture, Box<dyn Error>> {
    let dir = TempDir::new()?;
    let seed = seed();
    let issuer = new_issuer(dir.path(), "issuer", seed)?;
    let a = issue(&issuer, "agent-a", vec![])?;
    let b = issue(&issuer, "agent-b", vec![])?;
    let a_key = identity(dir.path(), "agent-a", a.subject_signing_key.to_bytes())?;
    let a_cose = sign_attestation(b"the first payload", &a_key).to_cose_bytes();
    let origin = open_log(&dir.path().join("certificates"))?
        .origin()
        .to_owned();
    Ok(Fixture {
        dir,
        issuer,
        seed,
        a_der: a.der_bytes,
        b_der: b.der_bytes,
        a_cose,
        origin,
    })
}

impl Fixture {
    fn a(&self) -> CertificateHash {
        CertificateHash::of_der(&self.a_der)
    }

    fn b(&self) -> CertificateHash {
        CertificateHash::of_der(&self.b_der)
    }

    /// The three well-formed leaves of leg 1.
    fn leaves(&self) -> [CertificateLeaf; 3] {
        [
            CertificateLeaf::Issuance {
                der: self.a_der.clone(),
            },
            CertificateLeaf::Revocation(Revocation::sign(&self.origin, self.a(), &self.issuer.key)),
            CertificateLeaf::AttestationEntry {
                certificate: self.a(),
                cose: self.a_cose.clone(),
            },
        ]
    }

    /// One revocation that verifies and three that are refused, leg 3.
    fn revocations(&self) -> Result<[(Revocation, &'static str); 4], Box<dyn Error>> {
        let other = new_issuer(self.dir.path(), "other", seed())?;
        let by_other_key = Revocation::sign(&self.origin, self.a(), &other.key);
        let for_other_origin =
            Revocation::sign("example.test/lys/another-log", self.a(), &self.issuer.key);
        let valid = Revocation::sign(&self.origin, self.a(), &self.issuer.key);
        let other_hash = Revocation {
            certificate: self.b(),
            signature: valid.signature,
        };
        Ok([
            (valid, "signed by the issuing authority for this origin"),
            (by_other_key, "signed by a second authority's key"),
            (for_other_origin, "signed for another origin"),
            (other_hash, "signed over another certificate's hash"),
        ])
    }
}

#[test]
fn d013_r2_ac1_three_leaves_round_trip() -> TestResult {
    let fixture = fixture()?;
    let mut round_trips = 0;
    for (kind, leaf) in [0x01u8, 0x02, 0x03].into_iter().zip(fixture.leaves()) {
        let bytes = leaf.encode();
        assert_eq!(&bytes[..31], LEAF_DOMAIN_TAG.as_slice());
        assert_eq!(bytes[31], 0, "a zero byte follows the tag");
        assert_eq!(bytes[32], kind, "the kind byte");
        match kind {
            0x01 => assert_eq!(bytes.len(), 33 + fixture.a_der.len()),
            0x02 => assert_eq!(bytes.len(), REVOCATION_LEAF_LEN),
            _ => assert_eq!(bytes.len(), 65 + fixture.a_cose.len()),
        }
        assert_eq!(CertificateLeaf::decode(&bytes)?, leaf);
        round_trips += 1;
    }
    assert_eq!(REVOCATION_LEAF_LEN, 129);
    assert_eq!(round_trips, 3);
    Ok(())
}

#[test]
fn d013_r2_ac2_seven_malformed_inputs_are_refused_whole() {
    let inputs = malformed_inputs();
    assert_eq!(
        inputs[3].len(),
        33,
        "the issuance leaf ends at its kind byte"
    );
    assert_eq!(inputs[4].len(), 128);
    assert_eq!(inputs[5].len(), 130);
    assert_eq!(inputs[6].len(), 137);
    let (mut refusals, mut decoded) = (0, 0);
    for bytes in &inputs {
        match CertificateLeaf::decode(bytes) {
            Err(RevocationError::CertificateLeafMalformed { .. }) => refusals += 1,
            Err(other) => panic!("refused with another name: {other}"),
            Ok(..) => decoded += 1,
        }
    }
    assert_eq!(refusals, 7);
    assert_eq!(decoded, 0);
}

#[test]
fn d013_r2_ac3_revocation_verifies_only_under_the_issuer_for_its_origin() -> TestResult {
    let fixture = fixture()?;
    let issuer_key = fixture.issuer.public_key();
    let (mut verified, mut refused) = (0, 0);
    for (revocation, case) in fixture.revocations()? {
        match revocation.verify(&fixture.origin, &issuer_key) {
            Ok(()) => verified += 1,
            Err(RevocationError::RevocationNotSignedByIssuer) => refused += 1,
            Err(other) => panic!("{case}: refused with another name: {other}"),
        }
    }
    assert_eq!(verified, 1);
    assert_eq!(refused, 3);
    Ok(())
}

#[test]
fn d013_r2_ac4_debug_renderings_carry_no_seed_bytes() -> TestResult {
    let fixture = fixture()?;
    let seed_hex = hex(&fixture.seed);
    assert_eq!(seed_hex.len(), 64);
    let mut renderings: Vec<String> = Vec::new();
    for leaf in fixture.leaves() {
        render(&mut renderings, &leaf);
        render(&mut renderings, &leaf.encode());
        render(&mut renderings, &CertificateLeaf::decode(&leaf.encode())?);
    }
    for bytes in malformed_inputs() {
        render(&mut renderings, &CertificateLeaf::decode(&bytes));
    }
    for (revocation, ..) in fixture.revocations()? {
        render(&mut renderings, &revocation);
        render(
            &mut renderings,
            &revocation.verify(&fixture.origin, &fixture.issuer.public_key()),
        );
    }
    render(&mut renderings, &fixture.a());
    render(&mut renderings, &fixture.b());
    assert!(renderings.len() >= 40, "{} renderings", renderings.len());
    for rendering in &renderings {
        assert!(
            !rendering.contains(&seed_hex),
            "a rendering carries the issuing authority's seed"
        );
    }
    Ok(())
}
