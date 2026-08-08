//! [`RecognisedCertificate`] — DP9's certificate gate, built on the only thing
//! `lys-core` actually offers: chain verification and key identity.
//!
//! # ⚠️ Read this before deploying it: recognising a credential is not
//! authenticating a submitter
//!
//! This policy admits a submission that **presents** a certificate this anchor
//! recognises. A certificate is a public artifact. Anyone who has seen one can
//! copy it and present it, and this policy will admit them, because nothing in
//! the submission ties the presenter to the certified key. That is asserted by
//! a test in this module rather than left as a caveat, so the property cannot
//! quietly stop being true without somebody reading this file.
//!
//! **The gap is deliberate and it is not this increment's to close.** Closing
//! it means binding a submission to the holder of the subject key, and there
//! are exactly two ways:
//!
//! - **A transport that authenticates the peer** — mutual TLS, an
//!   already-authenticated session — and hands this policy the certificate
//!   *it* verified the peer against. Then presentation and possession are the
//!   same event and this policy is doing the remaining half of the job:
//!   deciding whether the authenticated peer's certificate is one this anchor
//!   recognises. This is the shape the policy is built for.
//! - **A submission-authentication wire format**: the holder signs the
//!   statement with the subject key, under a domain-separation tag, with
//!   freshness and an origin binding so the signature cannot be replayed to
//!   another anchor or at another time. **That is a permanent wire-format
//!   decision and it is not started.** Doing it without a tag, freshness and
//!   an origin binding would be worse than not doing it: a raw-bytes signature
//!   is replayable across anchors and confusable with any other use of the
//!   same key, while *measuring* as a possession check.
//!
//! `DECISIONS.md` DP9 already rules out the third idea anyone reaches for —
//! *"the gate is an access-control decision, and per `ca::request` it must not
//! be built on a request's signature"* — and `lys-core`'s own
//! `ca::request` module says why: a PKCS#10 request is replayable by design,
//! because proof of possession is a claim about key control and not an
//! authorisation of any particular act.
//!
//! # No claim vocabulary is invented here, and that is a hard line
//!
//! `lys-core` exposes `LYS_OID_ARC`, `encode_extension` and `decode_extension`:
//! opaque DER in, opaque DER out, interpreting nothing. There is no capability
//! type, no schema, no semantics and no verification anywhere in that crate.
//! `docs/design/WIRE-FORMATS.md` records what happened the last time a document
//! implied otherwise — a consuming project nearly moved an unverified string
//! into a signed X.509 extension and would have gained *better provenance with
//! identical semantics*, because **a signed unchecked string looks checked**.
//!
//! So this policy reads **no extension**, decodes **no claim**, and asks the
//! certificate exactly two questions, both of which `lys-core` already answers:
//!
//! 1. Was it signed by the authority whose public key this policy was
//!    constructed with, and is it inside its validity window right now?
//!    (`verify_certificate_chain`, which uses strict Ed25519 verification and
//!    rejects self-signed certificates.)
//! 2. Optionally: is its subject key one of a set this policy was constructed
//!    with? (`certificate_subject_public_key`, read **after** the chain
//!    verified — a key recovered from an unverified certificate is an
//!    attacker-chosen value.)
//!
//! Anything richer — *what may this holder do* — requires a capability format
//! that does not exist, and inventing one here would freeze a wire contract
//! that is the operator's to specify.
//!
//! # The refusals are one value; they are not one duration
//!
//! [`policy`](super::policy) names the timing channel in general terms. Here it
//! is concrete, and worth writing down where somebody deploying this will read
//! it: an absent credential returns without touching a parser, malformed DER
//! returns at the parse, a foreign certificate returns after a full Ed25519
//! verification, and an unlisted subject returns after that plus a second
//! parse. Those are four visibly different costs behind one identical answer.
//!
//! Nothing in this file can fix that — the work genuinely differs — and the
//! remedy is not to pad this function but to keep a submitter from being able
//! to measure it, which is a property of the transport in front of it. Stated
//! rather than left implied, because "the refusals are indistinguishable" is a
//! claim about the *message* and reads as a claim about everything.
//!
//! # What the chain check covers
//!
//! One level. `verify_certificate_chain` checks a leaf certificate against a
//! single expected issuer key; there is no intermediate handling, because
//! `lys-core`'s authority issues leaves directly. A deployment with
//! intermediates does not have a configuration problem here, it has a policy to
//! write.

use std::collections::BTreeSet;

use lys_core::ca::{certificate_subject_public_key, verify_certificate_chain};

use super::context::SubmitterContext;
use super::policy::{AdmissionPolicy, NotAdmitted};
use crate::wire::Submission;

/// Admits a submission that presents an X.509 certificate issued by a named
/// authority — and, optionally, bound to one of a named set of subject keys.
///
/// The bytes in the submitter's context are read as certificate DER. That
/// reading is this policy's, not a lys wire format: a
/// [`SubmitterContext`]'s credential is opaque to the rest of the crate, and a
/// different policy is free to want something else there.
///
/// # It admits from BOTH provenances, deliberately, and its name is the honest
/// description of that
///
/// [`SubmitterContext`] separates bytes a submitter asserted from a peer a
/// transport authenticated, and this policy writes both arms of that match and
/// treats them identically. That is not an oversight and not a shortcut: a
/// certificate a submitter sent, chaining to the operator's authority and
/// inside its window, is exactly what "recognised" means, and refusing it would
/// make this type unusable in the library-only deployment it was written for.
///
/// **The consequence is that admission by this policy is not authentication,
/// on either arm.** On the asserted arm the presenter proved nothing. On the
/// authenticated arm somebody else proved something and this policy did not
/// observe it. A deployment that needs "the peer holds this key *and* the
/// certificate is recognised" needs a transport that establishes the first —
/// which is why the two facts have two parties in
/// [`context`](super::context).
///
/// **Read the [module docs](self) before deploying this.** It recognises a
/// credential; it does not authenticate a presenter, and the difference is the
/// whole security argument.
///
/// # The absence of `Default` is checked, not merely intended
///
/// There is no authority this crate could name, and a policy that defaulted to
/// trusting one would be the single worst default in the workspace. The first
/// snippet is the positive control for the second.
///
/// ```
/// fn requires_default<T: Default>() {}
/// requires_default::<Vec<u8>>();
/// let _ = lys_anchor::RecognisedCertificate::issued_by([0u8; 32]);
/// ```
///
/// ```compile_fail
/// fn requires_default<T: Default>() {}
/// requires_default::<Vec<u8>>();
/// requires_default::<lys_anchor::RecognisedCertificate>();
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecognisedCertificate {
    issuer_public_key: [u8; 32],
    subject_keys: Option<BTreeSet<[u8; 32]>>,
}

impl RecognisedCertificate {
    /// Admits any certificate this authority issued and that is currently
    /// within its validity window.
    ///
    /// The subject is not constrained: whoever the authority was willing to
    /// certify, this anchor is willing to admit. That is the right shape only
    /// when the authority's issuance policy *is* the admission policy — the
    /// authority is the operator's own, and issuing a certificate is already
    /// the decision to let the holder write.
    pub fn issued_by(issuer_public_key: [u8; 32]) -> Self {
        Self {
            issuer_public_key,
            subject_keys: None,
        }
    }

    /// Admits a certificate this authority issued **and** whose subject key is
    /// in `subject_keys`.
    ///
    /// An empty set admits nothing. That is the fail-closed direction and it is
    /// left as-is rather than reinterpreted as "no restriction": a set that
    /// silently meant *everyone* when it was accidentally emptied is how an
    /// allow-list stops being one without anybody noticing.
    pub fn issued_by_to(issuer_public_key: [u8; 32], subject_keys: BTreeSet<[u8; 32]>) -> Self {
        Self {
            issuer_public_key,
            subject_keys: Some(subject_keys),
        }
    }

    /// The authority public key this policy recognises certificates from.
    pub fn issuer_public_key(&self) -> &[u8; 32] {
        &self.issuer_public_key
    }

    /// The subject keys this policy is restricted to, if it is restricted at
    /// all. `None` means any subject the authority certified.
    pub fn subject_keys(&self) -> Option<&BTreeSet<[u8; 32]>> {
        self.subject_keys.as_ref()
    }
}

impl AdmissionPolicy for RecognisedCertificate {
    /// Admits a submission whose context carries a certificate from the
    /// configured authority, currently valid, and — where the policy is
    /// restricted — over one of the configured subject keys.
    ///
    /// # Errors
    ///
    /// [`NotAdmitted`], for every one of: no credential at all, a credential
    /// that is not parseable certificate DER, a certificate from another
    /// authority, a certificate outside its validity window, and a certificate
    /// over an unlisted subject key. The five are indistinguishable to the
    /// caller by construction, since there is one value to return.
    fn admit(
        &self,
        _submission: &Submission<'_>,
        context: &SubmitterContext<'_>,
    ) -> Result<(), NotAdmitted> {
        // Both credential-bearing arms, written out and treated alike. The
        // match is exhaustive with no `_`, so a third provenance could not be
        // added without this policy having to say what it thinks of it — and
        // the equal treatment here is a decision visible in this file rather
        // than a collapse hidden behind an accessor.
        let credential = match context {
            SubmitterContext::Unidentified => return Err(NotAdmitted),
            SubmitterContext::AssertedBySubmitter(bytes) => *bytes,
            SubmitterContext::AuthenticatedByTransport(peer) => peer.certificate(),
        };

        // Chain first. Everything read out of the certificate below is
        // attacker-chosen until this has succeeded.
        verify_certificate_chain(credential, &self.issuer_public_key)
            .map_err(|_err| NotAdmitted)?;

        if let Some(allowed) = &self.subject_keys {
            let subject_key =
                certificate_subject_public_key(credential).map_err(|_err| NotAdmitted)?;
            if !allowed.contains(&subject_key) {
                return Err(NotAdmitted);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
#[path = "certificate_tests.rs"]
mod tests;
