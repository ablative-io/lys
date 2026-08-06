//! Third-party interoperability gate for PKCS#10 proof of possession: a
//! certificate-signing request produced by the real `openssl` binary is driven
//! through lys's issuance path.
//!
//! # What this exists to hold up
//!
//! `README.md` promises that "the request is a standard PKCS#10 request, so an
//! agent with no lys at all can produce one with `openssl req` and still be
//! certified here". Every other test of that path builds its request with
//! [`create_certificate_request`](lys_core::ca::create_certificate_request) —
//! lys's own encoder — so the suite could only ever confirm that lys agrees
//! with itself. A symmetric mistake in the encoder and the verifier would be
//! invisible to all of it. OpenSSL is the second party: it has never seen this
//! crate, it encodes RFC 2986 to its own reading, and it signs with its own
//! Ed25519. The independence axis here is **implementation**, not platform —
//! one machine, one toolchain.
//!
//! # Both directions, in every test
//!
//! A suite of acceptances cannot tell a working verifier from one that accepts
//! everything, and a suite of refusals cannot tell one from a verifier that
//! refuses everything. So the positive case asserts the certified key equals
//! the key OpenSSL independently reports for its own private key — not merely
//! that issuance returned `Ok` — and each negative case opens with a positive
//! control on the pristine request before tampering with it.
//!
//! The tampered requests are corruptions of a **real** OpenSSL artifact, byte
//! for byte, never a hand-rolled forgery: a hand-rolled request tests this
//! file's idea of PKCS#10, which is the same single party all over again.
//!
//! # No skip path, ever
//!
//! The shared `tests/harness/mod.rs` is not reused here. Its policy is a *skip*
//! when the Go toolchain is absent, which is right for a vendored,
//! network-free cross-check and wrong for this one: a skipped gate reports
//! green while testing nothing, and the promise this file guards is the one a
//! stranger relies on. A missing or Ed25519-incapable `openssl` is therefore a
//! hard failure that names every candidate it tried.
//!
//! Presence is not capability. macOS ships `LibreSSL` at `/usr/bin/openssl`,
//! which answers `openssl version` happily and then refuses `-algorithm
//! ed25519` — so discovery *probes* each candidate by actually generating an
//! Ed25519 key, and a binary that cannot is passed over rather than used to
//! produce a request this gate would then have to interpret.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use lys_core::TrustError;
use lys_core::ca::{
    CertificateAuthority, certificate_subject_public_key, verify_certificate_request,
};
use lys_core::keys::Ed25519Identity;

/// The common name the request asks for and the authority certifies.
const SUBJECT: &str = "agent-noor";

/// A same-length replacement for [`SUBJECT`], for the misattribution case.
/// Equal length keeps the DER structurally valid, so the request is refused by
/// the signature check rather than by the parser.
const IMPOSTOR: &str = "agent-mall";

/// The DER prefix of an Ed25519 `SubjectPublicKeyInfo` (RFC 8410 §4): a 44-byte
/// structure whose final 32 bytes are the raw key.
const ED25519_SPKI_PREFIX: [u8; 12] = [
    0x30, 0x2a, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x03, 0x21, 0x00,
];

/// Candidate `openssl` binaries, in the order they are probed.
///
/// The pinned Homebrew path is tried before `/usr/bin/openssl` because the
/// latter is `LibreSSL` on macOS and cannot do Ed25519; the probe would reject it
/// anyway, but ordering keeps the common case off the failure path.
const OPENSSL_CANDIDATES: [&str; 5] = [
    "/opt/homebrew/bin/openssl",
    "/opt/homebrew/opt/openssl@3/bin/openssl",
    "/usr/local/opt/openssl@3/bin/openssl",
    "openssl",
    "/usr/bin/openssl",
];

/// True if `bin` can actually generate an Ed25519 key.
///
/// This is the capability probe, not a presence check: it asks for the exact
/// operation the gate needs and reads the answer, so a binary that exists and
/// answers `version` but lacks the algorithm is correctly reported as unusable.
fn ed25519_capable(bin: &Path) -> bool {
    Command::new(bin)
        .args(["genpkey", "-algorithm", "ed25519", "-outform", "DER"])
        .output()
        .is_ok_and(|out| out.status.success() && !out.stdout.is_empty())
}

/// Resolves an Ed25519-capable `openssl`, or fails the test loudly.
///
/// `LYS_OPENSSL_BIN` overrides discovery. An override that cannot do Ed25519 is
/// a hard failure rather than a fall-through to some other binary: silently
/// ignoring an explicit instruction would mean the gate reports on a toolchain
/// nobody asked it to test.
///
/// # Panics
///
/// Panics if no candidate is Ed25519-capable. There is deliberately no skip.
fn resolve_openssl() -> PathBuf {
    if let Some(overridden) = std::env::var_os("LYS_OPENSSL_BIN") {
        let bin = PathBuf::from(overridden);
        assert!(
            ed25519_capable(&bin),
            "LYS_OPENSSL_BIN points at {} but it cannot generate an Ed25519 key — \
             this gate must not fall back to a different binary than the one named",
            bin.display()
        );
        return bin;
    }

    for candidate in OPENSSL_CANDIDATES {
        let bin = PathBuf::from(candidate);
        if ed25519_capable(&bin) {
            return bin;
        }
    }

    panic!(
        "no Ed25519-capable openssl found (tried, in order: {}). This gate holds up the \
         README's promise that a third party can be certified from a plain `openssl req`, \
         so it fails rather than skips: install OpenSSL 3.x, or set LYS_OPENSSL_BIN to a \
         capable binary. Note that macOS's /usr/bin/openssl is LibreSSL and cannot do \
         Ed25519.",
        OPENSSL_CANDIDATES.join(", ")
    );
}

/// Runs `openssl` with `args`, returning stdout; any non-zero exit is a panic
/// carrying the binary's own stderr, so a failure explains itself.
///
/// # Panics
///
/// Panics if the process cannot be spawned or exits non-zero.
fn openssl(bin: &Path, args: &[&str]) -> Vec<u8> {
    let out = Command::new(bin)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", bin.display()));
    assert!(
        out.status.success(),
        "openssl {} failed ({}): {}",
        args.join(" "),
        out.status,
        String::from_utf8_lossy(&out.stderr)
    );
    out.stdout
}

/// An OpenSSL-produced request and the public key OpenSSL says belongs to the
/// private key that signed it.
struct OpensslRequest {
    /// The PKCS#10 request, DER, exactly as `openssl req` wrote it.
    der: Vec<u8>,
    /// The 32-byte raw Ed25519 public key, read back out of the private key by
    /// `openssl pkey -pubout` — an extraction lys plays no part in.
    public_key: [u8; 32],
    /// Keeps the temporary directory alive for the lifetime of the fixture.
    _dir: tempfile::TempDir,
}

/// Generates an Ed25519 key and a PKCS#10 request for `subject` with the real
/// `openssl` binary.
///
/// The invocation is the plain one from the README — `genpkey` then `req -new
/// -key … -subj …` — with no lys-supplied configuration file, because a gate
/// that had to hand OpenSSL a tailored config would be testing a request only
/// lys knows how to ask for.
///
/// # Panics
///
/// Panics if openssl fails, if the extracted public key is not a 44-byte
/// Ed25519 SPKI, or if the request does not end in the 64-byte Ed25519
/// signature BIT STRING the tampering cases rely on.
fn openssl_request(bin: &Path, subject: &str) -> OpensslRequest {
    let dir = tempfile::tempdir().unwrap();
    let key = dir.path().join("holder.key.pem");
    let req = dir.path().join("request.der");
    let key_arg = key.to_str().expect("temp path is UTF-8");
    let req_arg = req.to_str().expect("temp path is UTF-8");

    openssl(bin, &["genpkey", "-algorithm", "ed25519", "-out", key_arg]);
    openssl(
        bin,
        &[
            "req",
            "-new",
            "-key",
            key_arg,
            "-subj",
            &format!("/CN={subject}"),
            "-batch",
            "-outform",
            "DER",
            "-out",
            req_arg,
        ],
    );

    let spki = openssl(bin, &["pkey", "-in", key_arg, "-pubout", "-outform", "DER"]);
    assert_eq!(
        spki.len(),
        ED25519_SPKI_PREFIX.len() + 32,
        "openssl's public key is not a 44-byte Ed25519 SubjectPublicKeyInfo"
    );
    assert_eq!(
        &spki[..ED25519_SPKI_PREFIX.len()],
        &ED25519_SPKI_PREFIX,
        "openssl's public key does not carry the RFC 8410 Ed25519 SPKI prefix"
    );
    let mut public_key = [0u8; 32];
    public_key.copy_from_slice(&spki[ED25519_SPKI_PREFIX.len()..]);

    let der = std::fs::read(&req).unwrap();
    assert_signature_is_the_last_64_bytes(&der);

    OpensslRequest {
        der,
        public_key,
        _dir: dir,
    }
}

/// Confirms the request ends in `BIT STRING`, length 65, zero unused bits,
/// followed by the 64 signature bytes.
///
/// The tampering cases flip a byte at the end of the DER and call it "the
/// signature". That claim has to be checked against what OpenSSL actually
/// emitted rather than assumed, or a future encoding change would quietly turn
/// the negative cases into corruptions of some other field — still refused, but
/// for a reason nobody chose.
fn assert_signature_is_the_last_64_bytes(der: &[u8]) {
    let header = der
        .len()
        .checked_sub(67)
        .expect("request is too short to carry an Ed25519 signature");
    assert_eq!(
        &der[header..header + 3],
        &[0x03, 0x41, 0x00],
        "the request does not end in a 64-byte BIT STRING with no unused bits, so the \
         last 64 bytes are not the signature"
    );
}

/// A certificate authority over a throwaway generated identity.
fn test_authority() -> (tempfile::TempDir, CertificateAuthority) {
    let dir = tempfile::tempdir().unwrap();
    let identity = Ed25519Identity::load_or_generate(&dir.path().join("ca.key")).unwrap();
    (dir, CertificateAuthority::new(identity))
}

/// The positive control every negative case opens with: the untampered request
/// is accepted and establishes the key OpenSSL generated.
///
/// Without it a negative case is satisfied by a verifier that refuses
/// everything, which is indistinguishable from one that works.
fn assert_pristine_is_accepted(request: &OpensslRequest) {
    let verified = verify_certificate_request(&request.der)
        .expect("positive control: the untampered openssl request must verify");
    assert_eq!(verified.subject_public_key(), &request.public_key);
}

/// D-CA: OpenSSL's request is accepted, and the certificate certifies the key
/// OpenSSL generated — the whole point of proof of possession.
#[test]
fn openssl_request_is_certified_over_the_key_openssl_generated() {
    let bin = resolve_openssl();
    let request = openssl_request(&bin, SUBJECT);
    let (_dir, ca) = test_authority();

    let verified = verify_certificate_request(&request.der)
        .expect("a plain `openssl req` PKCS#10 request must verify");
    assert_eq!(verified.common_name(), SUBJECT);
    assert_eq!(
        verified.subject_public_key(),
        &request.public_key,
        "the verified subject key must be the one `openssl pkey -pubout` reports"
    );

    let certified = ca
        .issue_certificate_for_request(&request.der, SUBJECT, Duration::from_secs(3600), vec![])
        .expect("issuance over a verified openssl request must succeed");

    // The key is compared against openssl's independent extraction at every
    // point it could have been substituted: the struct field, and the DER the
    // certificate actually carries. The field alone would only agree with the
    // value this crate put there.
    assert_eq!(certified.subject_public_key, request.public_key);
    assert_eq!(
        certificate_subject_public_key(&certified.der_bytes).unwrap(),
        request.public_key,
        "the issued certificate must bind openssl's key, not some other one"
    );
    assert_ne!(
        request.public_key,
        ca.public_key_bytes(),
        "the holder's key must not be the authority's own — otherwise the comparisons \
         above could pass on a certificate that certified the issuer"
    );
    assert_eq!(certified.issuer_public_key, ca.public_key_bytes());
    ca.verify_certificate_chain(&certified.der_bytes)
        .expect("the issued certificate must verify against its issuer");
}

/// A single flipped bit in OpenSSL's own signature is refused — at
/// verification and at issuance.
#[test]
fn openssl_request_with_a_flipped_signature_bit_is_refused() {
    let bin = resolve_openssl();
    let request = openssl_request(&bin, SUBJECT);
    let (_dir, ca) = test_authority();
    assert_pristine_is_accepted(&request);

    let mut tampered = request.der.clone();
    let last = tampered.len() - 1;
    tampered[last] ^= 0x01;
    assert_ne!(tampered, request.der);

    let error = verify_certificate_request(&tampered)
        .expect_err("a request whose signature was altered must not verify");
    assert!(
        matches!(error, TrustError::CertificateVerification { .. }),
        "expected a verification failure, got {error:?} — a parsing failure would mean \
         the flip broke the structure rather than the signature"
    );

    let error = ca
        .issue_certificate_for_request(&tampered, SUBJECT, Duration::from_secs(3600), vec![])
        .expect_err("issuance must refuse a request whose proof of possession failed");
    assert!(matches!(error, TrustError::CertificateVerification { .. }));
}

/// The misattribution attack the `request` module exists to prevent, built
/// rather than argued: rewriting the common name in OpenSSL's signed request
/// leaves a structurally valid request whose signature no longer covers the
/// name it now carries, and it is refused.
///
/// This also pins that the self-signature covers the **subject name** and not
/// merely the key — a verifier that checked only the key would accept this and
/// issue Noor's key under Mallory's name.
#[test]
fn openssl_request_re_subjected_to_another_name_is_refused() {
    let bin = resolve_openssl();
    let request = openssl_request(&bin, SUBJECT);
    let (_dir, ca) = test_authority();
    assert_pristine_is_accepted(&request);

    assert_eq!(
        SUBJECT.len(),
        IMPOSTOR.len(),
        "the rewrite must be in place"
    );
    let matches: Vec<usize> = request
        .der
        .windows(SUBJECT.len())
        .enumerate()
        .filter(|(_, window)| *window == SUBJECT.as_bytes())
        .map(|(index, _)| index)
        .collect();
    assert_eq!(
        matches.len(),
        1,
        "expected exactly one occurrence of the subject name in the request DER, found {}",
        matches.len()
    );

    let mut tampered = request.der.clone();
    let at = matches[0];
    tampered[at..at + IMPOSTOR.len()].copy_from_slice(IMPOSTOR.as_bytes());
    assert_ne!(
        tampered, request.der,
        "the rewrite must actually have changed the request"
    );

    // The rewritten request really does claim the other name: it parses, and
    // it is refused for the signature rather than for being malformed.
    let error = verify_certificate_request(&tampered)
        .expect_err("a request re-subjected to another name must not verify");
    assert!(
        matches!(error, TrustError::CertificateVerification { .. }),
        "expected a verification failure, got {error:?}"
    );

    let error = ca
        .issue_certificate_for_request(&tampered, IMPOSTOR, Duration::from_secs(3600), vec![])
        .expect_err("the authority must not certify a key under a name its holder never signed");
    assert!(matches!(error, TrustError::CertificateVerification { .. }));
}
