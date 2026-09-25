//! Typed errors of the identity subcommands. Every variant names the
//! operation and the resource or path it failed on, and none carries a byte
//! of a secret: a credential is never part of a message, a URL, or a body
//! that reaches a diagnostic.

use std::path::PathBuf;

/// What went wrong in `lys identity`, by operation and resource.
#[derive(Debug, thiserror::Error)]
pub enum IdentityError {
    /// A file or directory operation failed.
    #[error("{operation}: {}: {source}", path.display())]
    Io {
        /// The act that failed, for example `write the venue file`.
        operation: &'static str,
        /// The path it failed on.
        path: PathBuf,
        /// The underlying I/O error.
        #[source]
        source: std::io::Error,
    },

    /// The deployment configuration could not be read or does not hold.
    #[error("{operation}: {}: {detail}", path.display())]
    Config {
        /// The act that failed, for example `read the configuration`.
        operation: &'static str,
        /// The configuration file.
        path: PathBuf,
        /// Why, in words that name the key, never a value that is a secret.
        detail: String,
    },

    /// A value does not satisfy the rule the resource has for it.
    #[error("{operation} on {resource}: {detail}")]
    Invalid {
        /// The act that found the value wanting.
        operation: &'static str,
        /// The resource, for example a client id or a theme name.
        resource: String,
        /// The rule that failed.
        detail: String,
    },

    /// Rauthy answered a status the operation does not accept.
    #[error("{operation} on {resource}: rauthy answered {status}")]
    Status {
        /// The act that was refused.
        operation: &'static str,
        /// The resource the act was on.
        resource: String,
        /// The HTTP status Rauthy answered.
        status: u16,
    },

    /// The request could not be carried, or its outcome is unknown.
    #[error("{operation} on {resource}: {detail}")]
    Transport {
        /// The act whose transport failed.
        operation: &'static str,
        /// The resource the act was on.
        resource: String,
        /// What the transport said, with no request or answer body in it.
        detail: String,
    },

    /// Rauthy's answer did not have the shape the operation reads.
    #[error("{operation} on {resource}: the answer could not be read as {what} ({detail})")]
    Answer {
        /// The act whose answer was unreadable.
        operation: &'static str,
        /// The resource the act was on.
        resource: String,
        /// What the answer was expected to be.
        what: &'static str,
        /// The kind of failure, never the answer's bytes.
        detail: String,
    },

    /// One or more declared services are not ready.
    #[error("{count} of the declared services are not ready: {names}")]
    Unready {
        /// How many are unready.
        count: usize,
        /// Their names, comma separated, each with its reason.
        names: String,
    },
}

/// The identity subcommands' result alias.
pub type IdentityResult<T> = Result<T, IdentityError>;
