//! [`SubmitterContext`] — what was established about the party submitting, and
//! by whom — and [`AuthenticatedPeer`], the arm only something that performed
//! an authentication is entitled to build.
//!
//! # The defect this type exists to prevent, before there is anything to
//! prevent it in
//!
//! A credential can reach an anchor two ways, and they are worth exactly
//! different amounts:
//!
//! - **The submitter sent it.** It arrived inside their message. Nothing
//!   checked that they hold the key it names. It is a claim.
//! - **A transport authenticated the peer against it.** A mutual-TLS handshake
//!   completed, so the peer demonstrably holds the private key for the
//!   certificate's subject. It is an observation.
//!
//! Both are certificate bytes. If they arrive through one field, a policy that
//! runs `verify_certificate_chain` over them returns *admitted* in both cases,
//! and every later reader sees "the certificate was checked". The chain **was**
//! checked. What was never checked, in the first case, is that the sender holds
//! anything at all — and after the fact nothing can tell the two apart.
//!
//! That is this project's oldest recurring defect wearing new clothes: **a
//! signed unchecked string looks checked** (`docs/design/WIRE-FORMATS.md`,
//! recorded after it nearly cost a consuming project). The remedy there was to
//! stop describing a transport as though it were a schema. The remedy here is
//! the same in shape: **the two provenances are two variants, and the one that
//! carries an obligation is the one you cannot write by accident.**
//!
//! It is also the *plausible* wiring that goes wrong, not a careless one.
//! Somebody implementing DP24's transport, holding a peer certificate from a
//! completed handshake, would reasonably populate a `credential` field with it
//! — and the same field would accept a blob parsed straight out of the
//! submitter's message with no visible difference at either end.
//!
//! # ⚠️ This is an asserted fact, not an enforced one, and saying otherwise
//! would rebuild the defect one level up
//!
//! Rust visibility cannot keep [`AuthenticatedPeer::verified_by_transport`]
//! away from a caller who has not authenticated anything. Nothing in this crate
//! observes the handshake, and nothing here can. **What the type buys is that
//! the mistake stops being an invisible field assignment and becomes a line of
//! code that says `verified_by_transport` next to bytes that were not** — a
//! claim somebody typed, in a diff a reviewer reads, under a name that states
//! what it means.
//!
//! That is a real improvement and it is not a capability. A reader who takes
//! [`AuthenticatedPeer`] as proof rather than as an attestation by its
//! constructor's caller has made the same category error this module exists to
//! name.
//!
//! # Two different parties establish two different facts
//!
//! Worth separating because they are easy to conflate and neither implies the
//! other:
//!
//! - **The transport establishes possession** — this peer holds the key. It has
//!   no opinion about which authority issued the certificate or whether this
//!   anchor recognises it.
//! - **The admission policy establishes recognition** — this certificate chains
//!   to an authority the operator named, and is inside its validity window. It
//!   has no way to observe possession.
//!
//! A deployment that wants both needs both, from both parties.
//!
//! # The asymmetry in how the variants are written is deliberate
//!
//! [`SubmitterContext::AssertedBySubmitter`] takes the bytes directly;
//! [`SubmitterContext::AuthenticatedByTransport`] takes a value that can only
//! come from a named constructor. The ceremony sits on the side that carries
//! the obligation, and its absence on the other side is the point: asserting
//! nothing should be easy, and claiming to have authenticated somebody should
//! require writing that you did.
//!
//! # This enum is exhaustive on purpose
//!
//! It is deliberately **not** `#[non_exhaustive]`. A policy matching on it must
//! write every arm, so treating an assertion and an authentication alike is a
//! visible decision in the policy's own source rather than a `_` arm nobody
//! reads — and there is no accessor here that returns "the credential,
//! whichever kind", because that method would be the one-line disarm of the
//! whole distinction. The cost is accepted knowingly: adding a variant later is
//! a breaking change for every implementation, which is the correct amount of
//! friction for changing what an anchor can be told about who is talking to it.

/// A certificate that something in front of this anchor authenticated a peer
/// against.
///
/// Holding one of these means the constructor's caller asserted that the peer
/// **demonstrated possession** of the private key for this certificate's
/// subject — typically by completing a mutual-TLS handshake. This crate does
/// not and cannot check that; see the [module docs](self) on why that makes
/// this an attestation rather than a proof.
///
/// It says nothing about whether the certificate is one this anchor recognises.
/// That is the admission policy's question and a separate one.
///
/// # Not constructible by writing it out, and that is checked
///
/// The field is private, so the only way to obtain a value is
/// [`verified_by_transport`](Self::verified_by_transport) — whose name is the
/// claim being made. The first snippet is the positive control: it establishes
/// that the probe compiles at all, so the second failing means what it looks
/// like rather than meaning the snippet was malformed.
///
/// ```
/// let peer = lys_anchor::AuthenticatedPeer::verified_by_transport(b"der");
/// let _ = peer.certificate();
/// let _ = lys_anchor::Submission { statement: b"s" };
/// ```
///
/// ```compile_fail
/// let _ = lys_anchor::AuthenticatedPeer { certificate: b"der" };
/// let _ = lys_anchor::Submission { statement: b"s" };
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AuthenticatedPeer<'a> {
    certificate: &'a [u8],
}

impl<'a> AuthenticatedPeer<'a> {
    /// Records that the caller authenticated a peer against `certificate`.
    ///
    /// # What calling this asserts
    ///
    /// **That the peer proved control of the private key** matching this
    /// certificate's subject public key, before this call — a completed mutual
    /// TLS handshake, or an equivalent challenge the caller performed and
    /// checked. Nothing downstream re-establishes it, and no error can report
    /// its absence, because its absence is not observable from here.
    ///
    /// Calling it with a certificate that merely arrived alongside a request is
    /// how an unauthenticated value acquires the appearance of an
    /// authenticated one. If that is the situation, the honest arm is
    /// [`SubmitterContext::AssertedBySubmitter`], and a policy is free to
    /// admit on it — [`RecognisedCertificate`](super::RecognisedCertificate)
    /// does exactly that, under a name that says so.
    ///
    /// # What it does not assert
    ///
    /// Nothing about the certificate's issuer, its validity window, or whether
    /// this anchor should accept it. Those are the policy's, and this value is
    /// an input to that decision rather than a substitute for it.
    pub fn verified_by_transport(certificate: &'a [u8]) -> Self {
        Self { certificate }
    }

    /// The certificate DER the peer was authenticated against.
    pub fn certificate(&self) -> &'a [u8] {
        self.certificate
    }
}

/// What was established about the party making a submission, and by whom.
///
/// Constructed by whoever is in front of the anchor — a transport, a CLI, a
/// test — never by the submitter, and never inferred here. There is no
/// [`Default`]: an anchor is told what was established, and "nobody said" is
/// [`Unidentified`](Self::Unidentified), which a caller writes on purpose.
///
/// # The absence of `Default` is checked, not merely intended
///
/// A default would have to pick one of these, and every choice is wrong: a
/// defaulted `Unidentified` makes a caller who forgot the context
/// indistinguishable from one who had none, and anything else is worse. The
/// first snippet is the positive control for the second.
///
/// ```
/// fn requires_default<T: Default>() {}
/// requires_default::<Vec<u8>>();
/// let _ = lys_anchor::SubmitterContext::Unidentified;
/// ```
///
/// ```compile_fail
/// fn requires_default<T: Default>() {}
/// requires_default::<Vec<u8>>();
/// requires_default::<lys_anchor::SubmitterContext<'static>>();
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SubmitterContext<'a> {
    /// Nothing was established about the submitter.
    ///
    /// The correct value whenever the caller performed no authentication and
    /// was handed no credential — a local process appending to its own log, a
    /// test, a CLI. A policy that requires a credential refuses it, which is
    /// the fail-closed direction.
    Unidentified,

    /// The submitter supplied these bytes with their submission.
    ///
    /// **A claim, not an observation.** Nothing checked that the sender holds
    /// anything the bytes describe, and a certificate is a public artifact —
    /// anyone who has seen one can present it. A policy may still admit on
    /// this; what it must not do is describe the result as authentication.
    AssertedBySubmitter(&'a [u8]),

    /// A transport authenticated the peer against this certificate.
    ///
    /// See [`AuthenticatedPeer`] for what building one of these asserts, and
    /// for why it is an attestation by the caller rather than a fact this crate
    /// verified.
    AuthenticatedByTransport(AuthenticatedPeer<'a>),
}

#[cfg(test)]
#[path = "context_tests.rs"]
mod tests;
