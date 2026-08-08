//! [`CliError`] and the CLI-wide [`CliResult`] alias.
//!
//! Every subcommand returns `CliResult<()>`; `main` maps `Err` to exit code 1
//! after printing the `Display` form to stderr. Messages carry the failing path
//! and operation so an operator can act on them without a backtrace. No variant
//! ever carries private key material.
//!
//! # What this layer must not add
//!
//! [`AnchorError::NotAdmitted`] carries nothing — no reason, no policy name, no
//! size — because an admission decision is a function of a submitter's bytes
//! and a refusal that varied by cause would be an oracle for the rule
//! (`crates/lys-anchor/src/error.rs`). This type forwards it **transparently**.
//! There is deliberately no CLI variant that wraps a refusal with the flags the
//! operator passed, no "hint" naming the configured limit, and no branch that
//! reports which policy was in force. The CLI knows all three, and reporting
//! any of them here would reconstruct on the operator's terminal exactly what
//! the library spent a zero-sized type refusing to disclose — and a CLI is
//! precisely where a refusal ends up in front of a submitter, because an
//! operator pastes it.

use std::path::PathBuf;

use lys_anchor::AnchorError;
use lys_core::TrustError;
use lys_log_store::StoreError;

/// Errors surfaced by `lys-anchor` subcommands.
#[derive(Debug, thiserror::Error)]
pub enum CliError {
    /// An anchor operation failed.
    ///
    /// Transparent: `AnchorError`'s messages already name the origin, index or
    /// tree size involved, and restating them here would add a second place for
    /// that wording to drift. It is also how [`AnchorError::NotAdmitted`]
    /// reaches an operator with exactly the text the library chose and nothing
    /// appended.
    #[error(transparent)]
    Anchor(Box<AnchorError>),

    /// A log-storage failure this CLI has nothing to add to. The store's own
    /// message already names the index or path at fault, so it is forwarded
    /// verbatim rather than reworded into something less specific.
    ///
    /// Constructed only by the mapping in
    /// [`commands::anchor::open`](crate::commands::anchor::open) — the variants
    /// an operator can act on differently are translated there into messages
    /// that carry a remedy.
    #[error(transparent)]
    Store(Box<StoreError>),

    /// A `lys-core` operation on trusted local input failed — encoding a
    /// verifier key under this anchor's origin, for instance.
    ///
    /// Not a verification failure: nothing here is reachable by feeding this
    /// CLI a stranger's artifact, so the detail is the product rather than a
    /// leak.
    #[error(transparent)]
    Trust(Box<TrustError>),

    /// An anchor directory was required but is missing or uninitialized.
    #[error(
        "anchor directory not initialized: {} (run `lys-anchor init --dir {} --origin <origin> --key <keyfile> --genesis <file> --admit <policy>` first)",
        path.display(),
        path.display()
    )]
    AnchorDirMissing {
        /// Path that was checked for an initialized anchor directory.
        path: PathBuf,
    },

    /// The anchor directory failed its integrity check or a structural rule.
    /// Local trusted state — carries an actionable reason.
    #[error("anchor directory invalid: {}: {reason}", path.display())]
    AnchorDirInvalid {
        /// The anchor directory that failed the check.
        path: PathBuf,
        /// The specific discrepancy or structural violation.
        reason: String,
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

    /// Serializing a proof artifact to JSON failed. Receipts are not JSON —
    /// their COSE encoding is infallible.
    #[error("failed to serialize {what} to JSON: {source}")]
    JsonSerialize {
        /// What was being serialized, e.g. "inclusion proof artifact".
        what: &'static str,
        /// Underlying JSON error.
        #[source]
        source: serde_json::Error,
    },

    /// An issuer public key argument was not exactly 64 hexadecimal characters
    /// (a 32-byte Ed25519 key as printed by `lys key inspect`).
    #[error(
        "invalid --issuer-public-key: expected exactly 64 hexadecimal characters (a 32-byte Ed25519 key)"
    )]
    InvalidIssuerPublicKey,

    /// A subject key argument was not exactly 64 hexadecimal characters
    /// (a 32-byte Ed25519 key as printed by `lys key inspect`).
    #[error(
        "invalid --subject-key {value:?}: expected exactly 64 hexadecimal characters (a 32-byte Ed25519 key)"
    )]
    InvalidSubjectKey {
        /// The rejected argument value, echoed back so an operator passing
        /// several can see which one was refused.
        value: String,
    },

    /// A flag the chosen `--admit` policy requires was not supplied.
    ///
    /// Clap enforces the same requirement declaratively, so this is the
    /// belt-and-braces arm rather than the usual route. It exists because the
    /// alternative at that point in the code is `unreachable!()`, and a
    /// precondition asserted by a panic is one the next refactor removes
    /// silently.
    #[error("--admit {policy} requires {flag}")]
    AdmissionArgumentMissing {
        /// The policy that was named.
        policy: &'static str,
        /// The flag it needs.
        flag: &'static str,
    },

    /// A flag was supplied that the chosen `--admit` policy does not read.
    ///
    /// Refused rather than ignored. An operator who writes
    /// `--admit accept-all --max-bytes 4096` believes they have configured a
    /// limit; an anchor that accepted the invocation and admitted everything
    /// would be running an admission rule its operator did not choose, which is
    /// the exact failure DP23 spent a type parameter preventing.
    #[error("--admit {policy} does not read {flag}; remove it or choose the policy that does")]
    AdmissionArgumentIgnored {
        /// The policy that was named.
        policy: &'static str,
        /// The flag it does not read.
        flag: &'static str,
    },
}

impl From<AnchorError> for CliError {
    /// Boxes an anchor failure into the CLI's vocabulary.
    ///
    /// Boxed rather than inlined so `CliError` — and therefore every
    /// `CliResult` in this binary — stays small. `AnchorError` carries an
    /// origin, two sizes and a `TrustError` in its widest variant, and
    /// `result_large_err` is a merge gate here.
    fn from(err: AnchorError) -> Self {
        Self::Anchor(Box::new(err))
    }
}

impl From<TrustError> for CliError {
    /// Boxes a `lys-core` failure, for the reason [`From<AnchorError>`] boxes.
    fn from(err: TrustError) -> Self {
        Self::Trust(Box::new(err))
    }
}

impl From<StoreError> for CliError {
    /// Translates a storage failure into the CLI's vocabulary.
    ///
    /// Two variants are mapped by hand because the CLI can say something the
    /// library cannot: an uninitialized directory gets the `lys-anchor init`
    /// remedy, and re-initialization explains that the origin is pinned at
    /// init. The rest keep the store's own message, which already names the
    /// index or path involved.
    ///
    /// The catch-all is deliberate rather than lazy: `StoreError` is
    /// `#[non_exhaustive]`, and a new storage failure arriving in a future
    /// version must reach the operator verbatim instead of being refused a
    /// mapping at compile time and then hurried into whichever variant was
    /// closest. This mirrors `crates/lys/src/commands/log/store.rs`, whose
    /// reasoning applies here unchanged.
    fn from(err: StoreError) -> Self {
        match err {
            StoreError::NotInitialized { path } => Self::AnchorDirMissing { path },
            StoreError::AlreadyInitialized { path } => Self::AnchorDirInvalid {
                path,
                reason: "already initialized (log.json exists); the origin is pinned at init \
                         and an anchor directory is never re-initialized"
                    .to_string(),
            },
            StoreError::Corrupt { path, reason } => Self::AnchorDirInvalid { path, reason },
            StoreError::Io { context, source } => Self::Io { context, source },
            StoreError::Trust(source) => Self::Trust(Box::new(source)),
            other => Self::Store(Box::new(other)),
        }
    }
}

/// Convenience alias for `Result<T, CliError>`.
pub type CliResult<T> = Result<T, CliError>;

#[cfg(test)]
#[path = "error_tests.rs"]
mod tests;
