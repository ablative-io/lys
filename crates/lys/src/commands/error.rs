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

    /// An attestation envelope file could not be parsed as JSON.
    #[error("failed to parse attestation JSON from {}: {source}", path.display())]
    JsonParse {
        /// File that was being parsed.
        path: PathBuf,
        /// Underlying JSON error.
        #[source]
        source: serde_json::Error,
    },

    /// Serializing an attestation envelope to JSON failed.
    #[error("failed to serialize attestation to JSON: {source}")]
    JsonSerialize {
        /// Underlying JSON error.
        #[source]
        source: serde_json::Error,
    },

    /// The attestation did not verify against the supplied payload. All
    /// verification failures collapse to this one message by design — the
    /// library deliberately does not distinguish a tampered payload from a
    /// tampered signature or timestamp.
    #[error("attestation verification failed: payload hash mismatch or invalid signature")]
    VerificationFailed,
}

/// Convenience alias for `Result<T, CliError>`.
pub type CliResult<T> = Result<T, CliError>;

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn key_file_missing_display_names_path_and_remedy() {
        let err = CliError::KeyFileMissing {
            path: PathBuf::from("/keys/agent.key"),
        };
        let display = err.to_string();
        assert!(display.contains("/keys/agent.key"), "got: {display}");
        assert!(display.contains("lys key generate"), "got: {display}");
    }

    #[test]
    fn io_display_carries_context_and_source() {
        let err = CliError::Io {
            context: "failed to read payload file /tmp/p".to_string(),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "gone"),
        };
        let display = err.to_string();
        assert!(
            display.contains("failed to read payload file /tmp/p"),
            "got: {display}"
        );
        assert!(display.contains("gone"), "got: {display}");
    }

    #[test]
    fn verification_failed_display_is_actionable() {
        let display = CliError::VerificationFailed.to_string();
        assert!(
            display.contains("attestation verification failed"),
            "got: {display}"
        );
    }
}
