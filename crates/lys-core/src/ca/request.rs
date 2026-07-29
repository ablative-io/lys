//! PKCS#10 certificate-signing requests — proof of possession for a subject
//! key the holder already controls.
//!
//! Without this module a certificate can only be issued over a keypair the
//! authority itself generated (see
//! [`CertificateAuthority::issue_certificate`](super::CertificateAuthority::issue_certificate)),
//! which means the certificate binds a key the *authority* held, not one the
//! subject proved control of. Anything built on top of such a certificate —
//! transparently logging issuance, gating a write path on a recognised chain —
//! authenticates the authority's willingness to issue rather than the
//! identity of the holder. This module closes that gap: a holder signs a
//! request with their own key, and that self-signature is the proof of
//! possession.
//!
//! # What proof of possession actually prevents
//!
//! It is tempting to read proof of possession as "the verifier learns the
//! holder controls the key", and then to conclude it is redundant — because a
//! verifier who checks a signature against the certified key has just observed
//! that control directly and far more recently. That reading is wrong, and
//! getting it wrong is how an authority ends up dropping the check as
//! ceremony.
//!
//! The attack it prevents is **misattribution by key binding**, and it targets
//! the authority, not the verifier. Suppose Mallory may request certificates
//! and presents Noor's public key under the subject name `agent-mallory`.
//! Without proof of possession the authority issues it, and from then on every
//! statement Noor legitimately signs verifies against a certificate naming
//! Mallory. Noor's work is attributed to Mallory, by a certificate that is
//! genuinely valid, with no forgery anywhere and nothing for a verifier to
//! detect. The join in `lys verify --cert` would report success, correctly, on
//! a false statement.
//!
//! Both paths this crate offers are safe against that: `issue_certificate`
//! generates the key itself so no caller-supplied key is ever bound, and this
//! module requires a signature over the request — which includes the subject
//! name — so a presented key can only be certified by whoever holds it, and
//! only under the name they asked for.
//!
//! # A certificate does not record how it was issued
//!
//! Nothing in X.509 marks a certificate as having been issued over a proven
//! key, and this crate adds no such marker. A relying party therefore **cannot
//! tell from the certificate alone** whether the certified key is one its
//! holder controls or one an authority minted and discarded, and trust in that
//! property rests on the issuer's policy rather than on the artifact.
//!
//! This is a limitation to state rather than paper over, but it is narrower
//! than it first looks. A certificate over a discarded generated key binds a
//! key nobody can sign with, so it can never satisfy a signature check — it
//! fails closed, uselessly rather than dangerously. The property an issuer's
//! policy must actually supply is the one above: that it never binds a key its
//! requester did not prove control of. Making that policy publicly auditable is
//! the argument for logging issuance transparently, rather than something a
//! certificate field could carry.
//!
//! # Requests are replayable, deliberately
//!
//! A request's signature covers the subject key and the requested name, and
//! nothing that ties it to one issuance, one authority, or one moment. Anyone
//! holding a copy can submit it again, to this authority or another, and get a
//! certificate over that key under that name.
//!
//! That is not a weakness, because proof of possession is a claim about key
//! control and not an authorisation of a particular issuance. A replayed
//! request yields a certificate naming the same holder over the same key they
//! already proved they hold — a public fact anybody could have asked to have
//! certified. An authority that needs issuance itself to be authorised needs an
//! access-control decision about the requester, which is a separate concern
//! from this one and must not be built on the request's signature.
//!
//! # Why PKCS#10 rather than a lys-native format
//!
//! RFC 2986 already specifies exactly this exchange, its self-signature is
//! the canonical proof of possession, and third parties can produce and
//! inspect requests with `openssl req` and equivalents in every language.
//! Inventing a `lys/proof-of-possession/v1` tag would add a permanent wire
//! contract nobody else speaks in order to restate a 1998 standard.
//!
//! # What a request is allowed to influence
//!
//! **Exactly one thing: the subject public key.** A verified request also
//! carries the common name it asked for, which the authority compares against
//! the subject it was told to issue for — agreement between the two is
//! required, but the request never *supplies* the name. Validity window,
//! extensions, basic constraints and every other certificate field are built
//! by the authority from its own inputs. A request carrying requested
//! extensions is rejected rather than silently stripped, because a holder
//! whose asks are quietly dropped would believe they were honoured.
//!
//! # Verification is deliberately not non-oracle
//!
//! Artifact verification in this crate collapses every failure into one
//! indistinguishable message so a verifier cannot be used as an oracle. That
//! discipline does not apply here and precise diagnostics are correct: no
//! secret of the authority's participates in checking a request, and the
//! requester already knows their own key, so there is nothing an attacker
//! could learn by distinguishing a malformed request from a bad signature.
//! The authority operator, by contrast, needs to know which of the two it is.

use std::sync::Arc;

use ed25519_dalek::{Signature, VerifyingKey};
use rcgen::{CertificateParams, KeyPair};
use x509_parser::certification_request::X509CertificationRequest;
use x509_parser::oid_registry::OID_SIG_ED25519;
use x509_parser::prelude::FromDer;
use x509_parser::x509::X509Version;

use crate::ca::rcgen_bridge::{IdentitySigner, distinguished_name};
use crate::error::{TrustError, TrustResult};
use crate::keys::Ed25519Identity;

/// The Ed25519 public-key and signature length, in bytes.
const ED25519_KEY_LEN: usize = 32;
/// The Ed25519 signature length, in bytes.
const ED25519_SIGNATURE_LEN: usize = 64;

/// A certificate-signing request whose proof of possession has been verified.
///
/// This type cannot be constructed except by
/// [`verify_certificate_request`], and its fields are private. Holding one is
/// therefore itself the evidence that the request's self-signature verified
/// strictly under the key it carries — there is no path that produces a
/// `VerifiedRequest` for a key the requester did not demonstrably control.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedRequest {
    subject_public_key: [u8; ED25519_KEY_LEN],
    common_name: String,
}

impl VerifiedRequest {
    /// The 32-byte Ed25519 public key the requester proved possession of.
    pub fn subject_public_key(&self) -> &[u8; ED25519_KEY_LEN] {
        &self.subject_public_key
    }

    /// The single common name the request asked to be certified under.
    pub fn common_name(&self) -> &str {
        &self.common_name
    }
}

/// Builds a PKCS#10 certificate-signing request for `subject`, self-signed by
/// `identity`.
///
/// The returned DER is the holder's side of the exchange: it carries
/// `identity`'s public key and a signature over the request made with the
/// matching private key, which is what
/// [`verify_certificate_request`] checks. The private seed is reached only
/// through `IdentitySigner`, so it is never serialised into an rcgen
/// keypair.
///
/// The request is deliberately minimal — one common name, no subject
/// alternative names, no requested extensions — because a request may not
/// influence any certificate field other than the subject key, and a request
/// carrying extensions would be rejected at verification.
///
/// # Errors
///
/// Returns [`TrustError::CertificateGeneration`] if `subject` is empty or
/// only whitespace, or if rcgen cannot build or sign the request.
pub fn create_certificate_request(
    identity: &Arc<Ed25519Identity>,
    subject: &str,
) -> TrustResult<Vec<u8>> {
    if subject.trim().is_empty() {
        return Err(TrustError::CertificateGeneration {
            reason: "certificate-signing request subject must not be empty".to_string(),
        });
    }

    let mut params = CertificateParams::new(Vec::<String>::new()).map_err(|e| {
        TrustError::CertificateGeneration {
            reason: format!("failed to build certificate-signing request parameters: {e}"),
        }
    })?;
    params.distinguished_name = distinguished_name(subject);

    let signer = IdentitySigner::new(Arc::clone(identity));
    let key_pair =
        KeyPair::from_remote(Box::new(signer)).map_err(|e| TrustError::CertificateGeneration {
            reason: format!("failed to construct request key from identity: {e}"),
        })?;

    let request =
        params
            .serialize_request(&key_pair)
            .map_err(|e| TrustError::CertificateGeneration {
                reason: format!("failed to sign certificate-signing request: {e}"),
            })?;

    Ok(request.der().to_vec())
}

/// Verifies a PKCS#10 request's proof of possession and returns the subject
/// key it establishes.
///
/// Parses `request_der`, then requires all of the following. Every check is a
/// rejection, never a repair:
///
/// - no trailing bytes after the request's outer `SEQUENCE`;
/// - `CertificationRequestInfo` version v1, per RFC 2986;
/// - the signature algorithm is Ed25519;
/// - the subject public key algorithm is Ed25519, with the `parameters` field
///   absent as RFC 8410 §3 requires;
/// - the subject public key is exactly 32 bytes and the signature exactly 64,
///   each in a BIT STRING with no unused bits;
/// - exactly one common name, non-empty;
/// - no requested extensions;
/// - the signature verifies over the raw `CertificationRequestInfo` bytes
///   under the subject key, using `ed25519-dalek` **strict** verification.
///
/// Strict verification is what makes this a proof of possession rather than a
/// formality. Plain Ed25519 verification accepts small-order and torsion
/// public keys, for which signatures can be produced without knowing any
/// private key — so a requester could present a key nobody controls and still
/// satisfy the check, which is precisely the guarantee this function exists to
/// provide. `verify_strict` rejects those keys and signature malleability,
/// matching [`verify_certificate_chain_at`](super::verify_certificate_chain_at).
///
/// x509-parser's own `verify_signature` is deliberately not used: it routes
/// Ed25519 through ring's non-strict verification, which accepts exactly the
/// keys named above.
///
/// # Errors
///
/// Returns [`TrustError::CertificateParsing`] if `request_der` is not a
/// well-formed PKCS#10 request or carries structurally invalid fields, and
/// [`TrustError::CertificateVerification`] if the request is well-formed but
/// its self-signature does not verify. These reuse the certificate error
/// variants rather than introducing request-specific ones, which would be a
/// breaking change to a published error enum.
pub fn verify_certificate_request(request_der: &[u8]) -> TrustResult<VerifiedRequest> {
    let (remaining, request) = X509CertificationRequest::from_der(request_der).map_err(|e| {
        TrustError::CertificateParsing {
            reason: format!("failed to parse certificate-signing request DER: {e:?}"),
        }
    })?;
    // Trailing bytes would let two distinct encodings carry the same request,
    // so anything appended to the outer SEQUENCE is a rejection.
    if !remaining.is_empty() {
        return Err(TrustError::CertificateParsing {
            reason: format!(
                "certificate-signing request has {} trailing byte(s) after the outer SEQUENCE",
                remaining.len()
            ),
        });
    }

    let info = &request.certification_request_info;
    if info.version != X509Version::V1 {
        return Err(TrustError::CertificateParsing {
            reason: format!(
                "certificate-signing request version must be v1 per RFC 2986, got {}",
                info.version.0
            ),
        });
    }

    if request.signature_algorithm.algorithm != OID_SIG_ED25519 {
        return Err(TrustError::CertificateParsing {
            reason: "certificate-signing request signature algorithm is not Ed25519".to_string(),
        });
    }
    if info.subject_pki.algorithm.algorithm != OID_SIG_ED25519 {
        return Err(TrustError::CertificateParsing {
            reason: "certificate-signing request subject key algorithm is not Ed25519".to_string(),
        });
    }
    // RFC 8410 §3: the parameters field MUST be absent for Ed25519. Accepting
    // a populated field would admit two encodings of the same key algorithm.
    if info.subject_pki.algorithm.parameters.is_some() {
        return Err(TrustError::CertificateParsing {
            reason: "Ed25519 subject key algorithm must carry no parameters (RFC 8410)".to_string(),
        });
    }

    let subject_public_key = ed25519_bit_string(
        info.subject_pki.subject_public_key.unused_bits,
        &info.subject_pki.subject_public_key.data,
        ED25519_KEY_LEN,
        "subject public key",
    )?;
    let subject_public_key: [u8; ED25519_KEY_LEN] =
        subject_public_key
            .try_into()
            .map_err(|_err| TrustError::CertificateParsing {
                reason: "subject public key length check did not yield 32 bytes".to_string(),
            })?;

    let signature_bytes = ed25519_bit_string(
        request.signature_value.unused_bits,
        &request.signature_value.data,
        ED25519_SIGNATURE_LEN,
        "signature",
    )?;
    let signature_bytes: [u8; ED25519_SIGNATURE_LEN] =
        signature_bytes
            .try_into()
            .map_err(|_err| TrustError::CertificateParsing {
                reason: "signature length check did not yield 64 bytes".to_string(),
            })?;

    let common_name = single_common_name(info)?;

    // Requested extensions are refused, not stripped: a holder asking to be a
    // CA, or to carry capability claims of their own choosing, must be told no
    // rather than handed a certificate that silently omits the request.
    if let Some(mut extensions) = request.requested_extensions() {
        if extensions.next().is_some() {
            return Err(TrustError::CertificateParsing {
                reason: "certificate-signing request carries requested extensions, which are \
                         never honoured — the authority chooses every certificate field other \
                         than the subject key"
                    .to_string(),
            });
        }
    }

    let verifying_key = VerifyingKey::from_bytes(&subject_public_key).map_err(|_err| {
        TrustError::CertificateVerification {
            reason: "certificate-signing request subject key is not a valid Ed25519 point"
                .to_string(),
        }
    })?;
    let signature = Signature::from_bytes(&signature_bytes);
    verifying_key
        .verify_strict(info.raw, &signature)
        .map_err(|_err| TrustError::CertificateVerification {
            reason: "certificate-signing request signature did not verify under the subject \
                     key it presents — proof of possession failed"
                .to_string(),
        })?;

    Ok(VerifiedRequest {
        subject_public_key,
        common_name,
    })
}

/// Extracts a fixed-length Ed25519 value from a BIT STRING, rejecting unused
/// bits and any other length.
fn ed25519_bit_string<'a>(
    unused_bits: u8,
    data: &'a [u8],
    expected_len: usize,
    field: &str,
) -> TrustResult<&'a [u8]> {
    if unused_bits != 0 {
        return Err(TrustError::CertificateParsing {
            reason: format!(
                "certificate-signing request {field} BIT STRING must have no unused bits, got \
                 {unused_bits}"
            ),
        });
    }
    if data.len() != expected_len {
        return Err(TrustError::CertificateParsing {
            reason: format!(
                "certificate-signing request {field} must be {expected_len} bytes for Ed25519, \
                 got {}",
                data.len()
            ),
        });
    }
    Ok(data)
}

/// Returns the request's single common name.
///
/// More than one common name is rejected rather than resolved: the authority
/// compares one name against the subject it was asked to certify while a
/// reader might display another, and that disagreement is a confusion surface
/// with no legitimate use.
fn single_common_name(
    info: &x509_parser::certification_request::X509CertificationRequestInfo<'_>,
) -> TrustResult<String> {
    let mut names = info.subject.iter_common_name();
    let first = names.next().ok_or_else(|| TrustError::CertificateParsing {
        reason: "certificate-signing request subject carries no common name".to_string(),
    })?;
    if names.next().is_some() {
        return Err(TrustError::CertificateParsing {
            reason: "certificate-signing request subject carries more than one common name"
                .to_string(),
        });
    }

    let common_name = first
        .as_str()
        .map_err(|e| TrustError::CertificateParsing {
            reason: format!("certificate-signing request common name is not printable text: {e}"),
        })?
        .to_string();
    if common_name.trim().is_empty() {
        return Err(TrustError::CertificateParsing {
            reason: "certificate-signing request common name must not be empty".to_string(),
        });
    }
    Ok(common_name)
}

#[cfg(test)]
#[path = "request_tests.rs"]
mod tests;
