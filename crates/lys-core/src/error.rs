//! [`TrustError`] and the crate-wide [`TrustResult`] alias.
//!
//! Every fallible public API on the trust primitives returns
//! `TrustResult<T>`. Each variant names a distinct trust operation, and every
//! `Display` string carries the operation name so callers can surface a
//! precise diagnostic without parsing free-form text. Variants that carry a
//! dynamic cause use a `reason: String` field; signature verification has a
//! dedicated parameterless variant so callers can match it structurally
//! rather than by string.

/// Errors returned from the trust primitives.
///
/// Variants are grouped by operation: certificate lifecycle (generation,
/// parsing, verification, revocation), the Merkle transparency log, sealed
/// payload transport (seal/unseal), key management, signing, and the
/// dedicated signature-verification failure.
///
/// # Why this is `#[non_exhaustive]`
///
/// Downstream `match`es must carry a wildcard arm, which makes **every future
/// variant an additive change instead of a breaking one**. That matters more
/// here than in a typical library: this crate gains a verification surface with
/// each new artifact class, and each one wants its own non-oracle failure value
/// (see [`Self::ReceiptVerification`] and [`Self::BundleVerification`]). Without
/// this attribute, giving a new artifact class an honest error would mean a
/// semver break, and the cheap way out — reusing another class's variant —
/// makes errors mean less over time.
///
/// # Non-oracle variants carry no cause, deliberately
///
/// The verification failures are deliberately causeless. A caller learns *that*
/// an artifact failed, never *which check* failed, because these types are
/// reachable from network-exposed surfaces where a distinguishable error is a
/// parsing oracle. Callers diagnosing their own artifacts verify the embedded
/// pieces individually, where the errors are specific and actionable.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum TrustError {
    /// Generating a certificate failed.
    #[error("certificate generation failed: {reason}")]
    CertificateGeneration {
        /// Human-readable cause of the generation failure.
        reason: String,
    },

    /// Parsing a certificate or extracting one of its fields failed.
    #[error("certificate parsing failed: {reason}")]
    CertificateParsing {
        /// Human-readable cause of the parsing failure.
        reason: String,
    },

    /// Verifying a certificate chain failed.
    #[error("certificate verification failed: {reason}")]
    CertificateVerification {
        /// Human-readable cause of the verification failure.
        reason: String,
    },

    /// A certificate revocation operation failed.
    ///
    /// Reserved for consumer-side operations: revocation tracking is
    /// explicitly a consumer concern (a design non-goal for this crate), so
    /// this crate never constructs the variant itself. It exists so
    /// consumers implementing revocation stores can surface failures through
    /// the shared [`TrustError`] type.
    #[error("certificate revocation failed: {reason}")]
    CertificateRevocation {
        /// Human-readable cause of the revocation failure.
        reason: String,
    },

    /// A Merkle transparency-log operation failed.
    #[error("merkle tree operation failed: {reason}")]
    MerkleTree {
        /// Human-readable cause of the Merkle operation failure.
        reason: String,
    },

    /// Sealing a payload for a recipient failed.
    #[error("seal failed: {reason}")]
    Seal {
        /// Human-readable cause of the seal failure.
        reason: String,
    },

    /// Unsealing failed — deliberately omits the cause so callers cannot
    /// distinguish wrong-key from tampered-ciphertext (non-oracle).
    #[error("unseal failed")]
    UnsealFailed,

    /// Attestation verification failed — the sender's public key did not
    /// match, or the signature over the sealed payload was invalid.
    #[error("attestation verification failed")]
    AttestationFailed,

    /// A key-management operation failed: file I/O, environment loading,
    /// base64 decoding, or key-material length validation.
    #[error("key management failed: {reason}")]
    KeyManagement {
        /// Human-readable cause of the key-management failure.
        reason: String,
    },

    /// Producing a signature failed.
    ///
    /// Reserved for consumer-side signing pipelines. This crate's own
    /// Ed25519 signing is infallible (dalek 2 deterministic signing), so the
    /// crate never constructs the variant itself; consumers whose signing
    /// paths can fail (HSMs, remote signers, key lookups) surface those
    /// failures through it.
    #[error("signing failed: {reason}")]
    Signing {
        /// Human-readable cause of the signing failure.
        reason: String,
    },

    /// A signature failed verification, or the supplied signature or public
    /// key bytes were structurally invalid.
    ///
    /// This is also the attestation artifact class's single non-oracle
    /// failure: a malformed or non-canonical COSE attestation artifact, a
    /// payload-hash mismatch, and an invalid signature all collapse to this
    /// one value, so callers cannot distinguish which check rejected the
    /// artifact.
    #[error("invalid signature")]
    InvalidSignature,

    /// Building or encoding a checkpoint or signed note failed (invalid
    /// origin or key name, malformed body).
    #[error("checkpoint encoding failed: {reason}")]
    CheckpointEncoding {
        /// Human-readable cause of the encoding failure.
        reason: String,
    },

    /// Parsing a checkpoint body failed. Used on already-verified body text
    /// and operator-supplied text; artifact verification collapses it
    /// (non-oracle).
    #[error("checkpoint parsing failed: {reason}")]
    CheckpointParsing {
        /// Human-readable cause of the parsing failure.
        reason: String,
    },

    /// A note verifier key string was malformed or internally inconsistent.
    /// Trusted operator input — carries an actionable reason.
    #[error("invalid note verifier key: {reason}")]
    VerifierKey {
        /// Human-readable cause of the verifier-key failure.
        reason: String,
    },

    /// A signed note failed verification — deliberately omits the cause so
    /// callers cannot distinguish malformed envelope, unknown key, or bad
    /// signature (non-oracle).
    #[error("note verification failed")]
    NoteVerification,

    /// Building a log proof artifact failed (tree too large for JSON-safe
    /// integers, size/index invariant violations at build time).
    #[error("log artifact encoding failed: {reason}")]
    LogArtifactEncoding {
        /// Human-readable cause of the encoding failure.
        reason: String,
    },

    /// A log proof artifact failed verification — deliberately omits the
    /// cause (non-oracle): bad checkpoint signature, size mismatch, root
    /// mismatch, malformed hashes, and kind confusion are indistinguishable.
    #[error("log artifact verification failed")]
    LogArtifactVerification,

    /// An anchor receipt failed verification — deliberately omits the cause
    /// (non-oracle): a bad signature, the wrong anchor key, a tampered
    /// inclusion path, a malformed or non-canonical artifact, and a leaf the
    /// receipt does not prove are all indistinguishable.
    ///
    /// Distinct from [`Self::InvalidSignature`], which it previously reused: a
    /// receipt failure is not necessarily a signature failure, and saying so
    /// misdescribed every other way one can fail. The *class* of artifact is
    /// already known to the caller from the function they called, so naming it
    /// leaks nothing a caller did not supply.
    #[error("receipt verification failed")]
    ReceiptVerification,

    /// A verification bundle failed verification — deliberately omits the
    /// cause (non-oracle): a malformed container, a broken inclusion proof, a
    /// receipt from the wrong anchor, and a chain whose links do not join are
    /// all indistinguishable.
    ///
    /// Distinct from [`Self::LogArtifactVerification`], which it previously
    /// reused, for the same reason: a bundle is a different artifact class with
    /// a different set of ways to be wrong.
    #[error("bundle verification failed")]
    BundleVerification,
}

/// Convenience alias for `Result<T, TrustError>`.
pub type TrustResult<T> = std::result::Result<T, TrustError>;

#[cfg(test)]
#[path = "error_tests.rs"]
mod tests;
