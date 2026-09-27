//! [`IdentityError`]: every named failure of `lys identity`.
//!
//! Invariants:
//!
//! - Every message opens with a stable `snake_case` name (`invalid_issuer`,
//!   `secret_missing`, `services_unready`, ...) so an operator or a test can
//!   match the failure without parsing prose.
//! - Every variant carries the operation, the resource and, where one exists,
//!   the path or address it failed on.
//! - No variant is ever given secret bytes. Credentials reach this module only
//!   as the name of what was missing or unreadable; a message relayed from
//!   Rauthy is scrubbed of the caller's credential before it is stored (see
//!   `rauthy.rs`). The redaction tests in `error_tests.rs` hold that line.

use std::path::PathBuf;

/// A failure of `lys identity prepare`, `configure` or `health`.
#[derive(Debug, thiserror::Error)]
pub enum IdentityError {
    /// The deployment configuration file could not be read.
    #[error("config_unreadable: failed to read deployment configuration {}: {source}", path.display())]
    ConfigUnreadable {
        /// The configuration file.
        path: PathBuf,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },

    /// The deployment configuration is not valid TOML of the expected shape.
    #[error("config_invalid: {}: {reason}", path.display())]
    ConfigInvalid {
        /// The configuration file.
        path: PathBuf,
        /// What the parser refused.
        reason: String,
    },

    /// The issuer origin is not an origin Rauthy can serve an issuer on.
    #[error("invalid_issuer: {field} = {value:?}: {reason}")]
    InvalidIssuer {
        /// The configuration key, e.g. `rauthy.public_origin`.
        field: &'static str,
        /// The rejected value. Configuration holds no secret.
        value: String,
        /// Why it was refused.
        reason: &'static str,
    },

    /// A redirect URI is not an exact, absolute, fragment-free URI.
    #[error("invalid_redirect_uri: {field} = {value:?}: {reason}")]
    InvalidRedirectUri {
        /// The configuration key, e.g. `clients.platform.redirect_uris`.
        field: String,
        /// The rejected value.
        value: String,
        /// Why it was refused.
        reason: &'static str,
    },

    /// A configuration value is outside what the deployment accepts.
    #[error("invalid_config_value: {field} = {value:?}: {reason}")]
    InvalidConfigValue {
        /// The configuration key.
        field: String,
        /// The rejected value.
        value: String,
        /// Why it was refused.
        reason: &'static str,
    },

    /// A managed client was given the id of Rauthy's built-in client.
    #[error(
        "builtin_client_reserved: {field} names `rauthy`, the built-in client Rauthy's own \
         migration inserts; configure never creates, changes or deletes it"
    )]
    BuiltinClientReserved {
        /// The configuration key that named it.
        field: String,
    },

    /// Both managed clients were given the same id.
    #[error("duplicate_client_id: clients.platform and clients.cambium both name {id:?}")]
    DuplicateClientId {
        /// The shared id.
        id: String,
    },

    /// The state directory lies inside a Git work tree.
    #[error(
        "state_dir_inside_git: {} is inside the Git work tree at {}; generated credentials \
         must stay out of Git, so choose a state_dir outside it",
        state_dir.display(),
        worktree.display()
    )]
    StateDirInsideGit {
        /// The configured state directory.
        state_dir: PathBuf,
        /// The work tree root that contains it.
        worktree: PathBuf,
    },

    /// A credential the operation needs has not been generated.
    #[error(
        "secret_missing: {resource} expected at {}; run `lys identity prepare` first",
        path.display()
    )]
    SecretMissing {
        /// Which credential, e.g. `rauthy_api_key`.
        resource: String,
        /// Where it was expected.
        path: PathBuf,
    },

    /// A private file is readable or writable by group or other.
    #[error(
        "private_file_too_open: {resource} at {} has mode {mode:o}; a private file must grant \
         no group or other access",
        path.display()
    )]
    PrivateFileTooOpen {
        /// Which private file.
        resource: String,
        /// Its path.
        path: PathBuf,
        /// The permission bits found.
        mode: u32,
    },

    /// A filesystem operation failed.
    #[error("io_failed: {operation} {resource} at {}: {source}", path.display())]
    Io {
        /// What was being done, e.g. `write`.
        operation: &'static str,
        /// What it was being done to, e.g. `identity.env`.
        resource: String,
        /// The path.
        path: PathBuf,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },

    /// The declared theme mapping could not be read or failed validation.
    #[error("themes_invalid: {}: {reason}", path.display())]
    ThemesInvalid {
        /// The mapping file.
        path: PathBuf,
        /// What failed.
        reason: String,
    },

    /// A theme colour pair measures below its WCAG 2.1 AA tier.
    #[error(
        "theme_contrast_below_tier: client {client}: {pair} measures {ratio:.2}:1, below the \
         {tier} tier of {required:.1}:1"
    )]
    ContrastBelowTier {
        /// The client whose theme was measured.
        client: String,
        /// The pair, e.g. `text over bg (ink)`.
        pair: &'static str,
        /// The measured ratio.
        ratio: f64,
        /// The tier's name.
        tier: &'static str,
        /// The tier's minimum ratio.
        required: f64,
    },

    /// A theme field holds a CSS value the contrast check cannot measure.
    #[error("theme_value_unmeasurable: client {client}: {field} = {value:?}")]
    ThemeValueUnmeasurable {
        /// The client whose theme was measured.
        client: String,
        /// The theme field.
        field: &'static str,
        /// The CSS value found.
        value: String,
    },

    /// A request could not reach a service, or no response arrived.
    #[error("service_unreachable: {operation} {resource} at {address}: {reason}")]
    Unreachable {
        /// What was being done, e.g. `read client`.
        operation: String,
        /// The resource, e.g. `platform`.
        resource: String,
        /// The service address.
        address: String,
        /// The transport failure.
        reason: String,
    },

    /// A write was sent but its outcome could not be established, even after
    /// reading the resource back.
    #[error(
        "outcome_uncertain: {operation} {resource} at {address} was sent and no response \
         arrived; reading it back did not settle whether it was applied: {reason}"
    )]
    OutcomeUncertain {
        /// What was being done.
        operation: String,
        /// The resource.
        resource: String,
        /// The service address.
        address: String,
        /// What the read-back found.
        reason: String,
    },

    /// Rauthy answered with an error status.
    #[error("rauthy_status: {operation} {resource} returned {status} ({name}): {message}")]
    RauthyStatus {
        /// What was being done.
        operation: String,
        /// The resource.
        resource: String,
        /// The HTTP status.
        status: u16,
        /// The status's stable name, e.g. `unauthorized`.
        name: &'static str,
        /// Rauthy's own message, scrubbed of the caller's credential.
        message: String,
    },

    /// Rauthy answered 2xx with a body that is not the expected shape.
    #[error("rauthy_response_invalid: {operation} {resource}: {reason}")]
    RauthyResponseInvalid {
        /// What was being done.
        operation: String,
        /// The resource.
        resource: String,
        /// What was wrong with the body.
        reason: String,
    },

    /// Rauthy holds clients configure does not manage.
    #[error(
        "unmanaged_clients: Rauthy holds {ids}, which configure does not manage; it manages \
         exactly the platform and Cambium clients beside the built-in rauthy client and \
         deletes nothing, so remove or rename them deliberately first"
    )]
    UnmanagedClients {
        /// The unexpected client ids, comma separated.
        ids: String,
    },

    /// One or more declared services are not ready.
    #[error("services_unready: {names}")]
    ServicesUnready {
        /// Each unready service with its named reason, comma separated.
        names: String,
    },
}

/// Convenience alias for `Result<T, IdentityError>`.
pub type IdentityResult<T> = Result<T, IdentityError>;

#[cfg(test)]
#[path = "error_tests.rs"]
mod tests;
