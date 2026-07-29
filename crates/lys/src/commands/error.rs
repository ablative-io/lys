//! [`CliError`] and the CLI-wide [`CliResult`] alias.
//!
//! Every subcommand returns `CliResult<()>`; `main` maps `Err` to exit
//! code 1 after printing the `Display` form to stderr. Messages carry the
//! failing path and operation so users can act on them without a backtrace.
//! No variant ever carries private key material.

use std::path::PathBuf;

/// Errors surfaced by `lys` subcommands.
#[derive(Debug, thiserror::Error)]
pub enum CliError {
    /// An identity key file was required but does not exist. Subcommands
    /// that consume a key (`key inspect`, `attest`) refuse to silently mint
    /// a fresh identity; only `key generate` creates key files.
    #[error(
        "identity key file not found: {} (run `lys key generate --out {}` to create one)",
        path.display(),
        path.display()
    )]
    KeyFileMissing {
        /// Path that was checked for the key file.
        path: PathBuf,
    },

    /// A filesystem operation failed.
    #[error("{context}: {source}")]
    Io {
        /// Description of the operation that failed, including the path.
        context: String,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },

    /// A `lys-core` trust operation failed.
    #[error(transparent)]
    Trust(#[from] lys_core::TrustError),

    /// A JSON file carrying a `lys-core` wire type (sealed envelope, log
    /// proof artifact) could not be parsed. Attestations are not JSON — a
    /// malformed attestation artifact collapses into the relevant command's
    /// generic verification failure instead (non-oracle).
    #[error("failed to parse {what} JSON from {}: {source}", path.display())]
    JsonParse {
        /// What the file was expected to contain, e.g. "sealed envelope".
        what: &'static str,
        /// File that was being parsed.
        path: PathBuf,
        /// Underlying JSON error.
        #[source]
        source: serde_json::Error,
    },

    /// Serializing a `lys-core` wire type (sealed envelope, log proof
    /// artifact) to JSON failed. Attestations are not JSON — their COSE
    /// encoding is infallible.
    #[error("failed to serialize {what} to JSON: {source}")]
    JsonSerialize {
        /// What was being serialized, e.g. "sealed envelope".
        what: &'static str,
        /// Underlying JSON error.
        #[source]
        source: serde_json::Error,
    },

    /// The attestation did not verify against the supplied payload. All
    /// verification failures collapse to this one message by design — the
    /// library deliberately does not distinguish a malformed or
    /// non-canonical artifact from a tampered payload, signature, or
    /// timestamp.
    #[error(
        "attestation verification failed: malformed or non-canonical artifact, payload mismatch, or invalid signature"
    )]
    VerificationFailed,

    /// The certificate did not verify against the trusted issuer key at the
    /// requested instant. Deliberately non-oracle: a forged signature, a
    /// self-signed certificate, a wrong issuer key, and an out-of-window
    /// instant all collapse to this one message so a caller learns nothing
    /// about which check rejected the certificate.
    #[error("certificate verification failed: invalid signature or outside validity window")]
    CertificateVerificationFailed,

    /// A certified attestation did not verify: the certificate, the
    /// attestation, or the binding between them failed.
    ///
    /// Deliberately non-oracle across *all three* halves. `lys verify --cert`
    /// answers one question — "did the identity this certificate vouches for
    /// make this statement?" — and a caller who learns which half failed can
    /// probe each independently. Localising a failure is a debugging task, and
    /// the message names the two commands that do it, each of which is itself
    /// non-oracle within its own surface.
    #[error(
        "certified attestation verification failed: invalid certificate, certificate outside \
         validity window, malformed or non-canonical attestation, payload mismatch, invalid \
         signature, or the attestation was not signed by the key the certificate certifies — \
         run `lys ca verify` and `lys verify` separately to localise it"
    )]
    CertifiedVerificationFailed,

    /// A certificate file could not be decoded as a PEM `CERTIFICATE` block.
    #[error("failed to parse PEM certificate from {}: {reason}", path.display())]
    PemParse {
        /// File that was being parsed.
        path: PathBuf,
        /// Structural problem with the PEM framing.
        reason: String,
    },

    /// A capability-claims file was not valid JSON. Claims are embedded in
    /// certificates byte-for-byte, so malformed input is refused loudly at
    /// issuance rather than baked into a signed artifact.
    #[error("failed to parse capability claims JSON from {}: {source}", path.display())]
    ClaimsJsonParse {
        /// File that was being parsed.
        path: PathBuf,
        /// Underlying JSON error.
        #[source]
        source: serde_json::Error,
    },

    /// An issuer public key argument was not exactly 64 hexadecimal
    /// characters (a 32-byte Ed25519 key as printed by `lys key inspect`).
    #[error(
        "invalid issuer public key: expected exactly 64 hexadecimal characters (a 32-byte Ed25519 key)"
    )]
    InvalidIssuerPublicKey,

    /// A recipient public key argument was not exactly 64 hexadecimal
    /// characters (a 32-byte X25519 key as printed by `lys key inspect`).
    #[error(
        "invalid recipient public key: expected exactly 64 hexadecimal characters (a 32-byte X25519 key)"
    )]
    InvalidRecipientPublicKey,

    /// A sender public key argument was not exactly 64 hexadecimal
    /// characters (a 32-byte Ed25519 key as printed by `lys key inspect`).
    #[error(
        "invalid sender public key: expected exactly 64 hexadecimal characters (a 32-byte Ed25519 key)"
    )]
    InvalidSenderPublicKey,

    /// A sealed envelope could not be opened. Deliberately non-oracle: a
    /// wrong recipient key, a forged or mismatched sender attestation, and a
    /// tampered or corrupt envelope all collapse to this one message so a
    /// caller learns nothing about which check rejected the envelope.
    #[error("sealed envelope open failed: invalid attestation or undecryptable envelope")]
    OpenFailed,

    /// A log directory was required but is missing or uninitialized.
    #[error(
        "log directory not initialized: {} (run `lys log init --dir {} --origin <origin>` first)",
        path.display(),
        path.display()
    )]
    LogDirMissing {
        /// Path that was checked for an initialized log directory.
        path: PathBuf,
    },

    /// The log directory failed its integrity check or a structural rule.
    /// Local trusted state — carries an actionable reason.
    #[error("log directory invalid: {}: {reason}", path.display())]
    LogDirInvalid {
        /// The log directory that failed the check.
        path: PathBuf,
        /// The specific discrepancy or structural violation.
        reason: String,
    },

    /// An inclusion-proof artifact did not verify. Deliberately non-oracle:
    /// a malformed artifact, a bad checkpoint signature, an origin mismatch,
    /// a size mismatch, and a root mismatch all collapse to this one message
    /// so a caller learns nothing about which check rejected the artifact.
    #[error("inclusion proof verification failed: invalid artifact, checkpoint, or leaf")]
    LogInclusionVerificationFailed,

    /// A consistency-proof artifact did not verify. Deliberately non-oracle:
    /// a malformed artifact, a bad checkpoint signature, an origin mismatch,
    /// a size mismatch, and a root mismatch all collapse to this one message
    /// so a caller learns nothing about which check rejected the artifact.
    #[error("consistency proof verification failed: invalid artifact or checkpoints")]
    LogConsistencyVerificationFailed,

    /// A timestamp argument could not be parsed as RFC 3339.
    #[error("invalid timestamp {value:?}: expected RFC 3339, e.g. 2026-07-10T12:00:00Z ({source})")]
    InvalidTimestamp {
        /// The rejected argument value.
        value: String,
        /// Underlying parse error.
        #[source]
        source: chrono::ParseError,
    },
}

/// Convenience alias for `Result<T, CliError>`.
pub type CliResult<T> = Result<T, CliError>;

#[cfg(test)]
#[path = "error_tests.rs"]
mod tests;
