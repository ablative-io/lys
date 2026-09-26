//! [`IdentityError`]: every failure `lys identity` can report, each by name.
//!
//! # Invariants
//!
//! - Every message begins with a stable `snake_case` failure name
//!   (`secret_missing`, `invalid_issuer`, `database_unreachable`, ...), so an
//!   operator, a gate log and a test can all match on the name rather than
//!   on prose.
//! - Every variant carries the operation, the resource and, where a file is
//!   involved, the path, so a failure is actionable without a backtrace.
//! - No variant carries secret bytes. Credentials are referred to by their
//!   declared name (`rauthy_api_key_secret`) and their file path, never by
//!   value, and Rauthy's own error text is carried only from its response
//!   body, which never echoes the request's credentials.

use std::path::PathBuf;

/// Errors surfaced by `lys identity prepare`, `configure` and `health`.
#[derive(Debug, thiserror::Error)]
pub enum IdentityError {
    /// A filesystem operation on a declared file failed.
    #[error("io_failed: {operation} {}: {source}", path.display())]
    Io {
        /// What was being done, e.g. "read deployment config".
        operation: &'static str,
        /// The file or directory it was done to.
        path: PathBuf,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },

    /// The deployment config is not in the TOML subset this CLI reads, or
    /// does not have the declared shape.
    #[error("config_invalid: {}: {reason}", path.display())]
    ConfigInvalid {
        /// The config file.
        path: PathBuf,
        /// What is wrong, with the line number where one applies.
        reason: String,
    },

    /// A config value failed validation. `name` is the failure name the
    /// value's rule carries (`invalid_database_host`, `invalid_origin`, ...).
    #[error("{name}: {field} {value:?}: {reason}")]
    ConfigValue {
        /// Stable failure name for this rule.
        name: &'static str,
        /// The dotted config key, e.g. `database.host`.
        field: String,
        /// The rejected value. Config files carry no secrets, so echoing it
        /// is safe and is what makes the refusal actionable.
        value: String,
        /// Why it was rejected.
        reason: &'static str,
    },

    /// The public origin cannot yield a valid issuer.
    #[error("invalid_issuer: rauthy.public_origin {value:?}: {reason}")]
    InvalidIssuer {
        /// The rejected origin.
        value: String,
        /// Why it cannot be an issuer.
        reason: &'static str,
    },

    /// A client redirect URI is not an exact, acceptable URI.
    #[error("invalid_redirect_uri: clients.{client} {value:?}: {reason}")]
    InvalidRedirect {
        /// Which managed client (`platform` or `cambium`).
        client: &'static str,
        /// The rejected URI.
        value: String,
        /// Why it was rejected.
        reason: &'static str,
    },

    /// A declared credential is absent from a deployment that has already
    /// been prepared. Regenerating it would substitute a new identity or
    /// strand the database, so it is refused instead.
    #[error(
        "secret_missing: {name} is absent at {} in a prepared deployment; restore it from the \
         deployment backup (deploy/identity/README.md) rather than generating a new one",
        path.display()
    )]
    SecretMissing {
        /// Declared credential name.
        name: &'static str,
        /// Where it was expected.
        path: PathBuf,
    },

    /// A declared credential file exists but does not hold a value of the
    /// declared form.
    #[error("secret_invalid: {name} at {}: {reason}", path.display())]
    SecretInvalid {
        /// Declared credential name.
        name: &'static str,
        /// The file that held it.
        path: PathBuf,
        /// What is wrong with its form (never its content).
        reason: &'static str,
    },

    /// A private file or directory grants access beyond its owner.
    #[error(
        "private_mode_unrestricted: {} has mode {mode:o}; expected {expected:o}",
        path.display()
    )]
    PrivateModeUnrestricted {
        /// The file or directory.
        path: PathBuf,
        /// Its permission bits.
        mode: u32,
        /// The mode it must have.
        expected: u32,
    },

    /// A private file was written but reading it back did not return what
    /// was written, so the outcome of the write is not established.
    #[error("private_write_unconfirmed: {}: read-back differs from what was written", path.display())]
    PrivateWriteUnconfirmed {
        /// The file.
        path: PathBuf,
    },

    /// The request never reached the service: connection or send failed.
    #[error("{service}_unreachable: {operation} {resource} at {address}: {source}")]
    Unreachable {
        /// `rauthy` or another declared service.
        service: &'static str,
        /// What was being done.
        operation: &'static str,
        /// The API path or resource.
        resource: String,
        /// The address dialled.
        address: String,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },

    /// The request was sent but no complete response arrived, so whether
    /// it took effect is unknown until it is read back.
    #[error("uncertain_outcome: {operation} {resource}: {reason}")]
    UncertainOutcome {
        /// What was being done.
        operation: &'static str,
        /// The API path or resource.
        resource: String,
        /// What went wrong after the request was sent.
        reason: String,
    },

    /// A write whose outcome was uncertain was read back and not found, so
    /// it did not take effect. Re-running `configure` retries it under the
    /// same operation identifier.
    #[error(
        "uncertain_outcome_unresolved: {operation} {resource}: read-back found no effect; re-run configure"
    )]
    UncertainUnresolved {
        /// What was being done.
        operation: &'static str,
        /// The resource read back.
        resource: String,
    },

    /// Rauthy answered with a non-success status.
    #[error("{}: {operation} {resource}: HTTP {status}: {message}", status_name(*.status))]
    RauthyStatus {
        /// What was being done.
        operation: &'static str,
        /// The API path.
        resource: String,
        /// The HTTP status.
        status: u16,
        /// Rauthy's own error message from the response body.
        message: String,
    },

    /// Rauthy answered with a body that is not the declared response type.
    #[error("rauthy_response_invalid: {operation} {resource}: {reason}")]
    RauthyResponse {
        /// What was being done.
        operation: &'static str,
        /// The API path.
        resource: String,
        /// Why the body was not accepted.
        reason: String,
    },

    /// After reconciliation, Rauthy's state still differs from the declared
    /// state for this resource.
    #[error("reconcile_mismatch: {resource}: {reason}")]
    ReconcileMismatch {
        /// The client or theme.
        resource: String,
        /// Which declared value differs.
        reason: String,
    },

    /// The theme mapping file is not the declared mapping.
    #[error("theme_invalid: {}: {reason}", path.display())]
    ThemeInvalid {
        /// The mapping file.
        path: PathBuf,
        /// What is wrong.
        reason: String,
    },

    /// A named contrast pair is below its WCAG 2.1 AA tier. A gap field
    /// whose Rauthy default fails stays a gap: this is a stop for the lead,
    /// never a reason to choose a value the estate does not name.
    #[error(
        "contrast_below_tier: client {client} pair {pair} measures {ratio} against a tier of {tier}"
    )]
    ContrastBelowTier {
        /// Rauthy client id.
        client: String,
        /// The pair, e.g. `text over bg`.
        pair: &'static str,
        /// Measured ratio, two decimal places.
        ratio: String,
        /// Required ratio.
        tier: &'static str,
    },

    /// A theme value is in a form the contrast check cannot resolve.
    #[error("theme_value_unresolved: client {client} field {field} value {value:?}")]
    ThemeValueUnresolved {
        /// Rauthy client id.
        client: String,
        /// The theme field.
        field: &'static str,
        /// The CSS value Rauthy exported.
        value: String,
    },

    /// One or more declared services or the database is not ready. Each
    /// entry is a failure name followed by what was dialled.
    #[error("unready: {}", failures.join("; "))]
    Unready {
        /// One entry per failed check, each beginning with its name.
        failures: Vec<String>,
    },
}

/// The failure name for a non-success Rauthy status.
fn status_name(status: u16) -> &'static str {
    match status {
        400 => "rauthy_bad_request",
        401 | 403 => "rauthy_unauthorized",
        404 => "rauthy_not_found",
        _ => "rauthy_status",
    }
}

#[cfg(test)]
#[path = "error_tests.rs"]
mod tests;
