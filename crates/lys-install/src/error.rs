//! [`IdentityError`]: every failure of `lys identity`, by a stable name.
//!
//! Each error carries the operation that failed, the resource it acted on and,
//! where one exists, the path it touched. None carries secret bytes: a
//! credential appears in an error only by the file that holds it, and a
//! response body from Rauthy is never copied into a message.

use std::fmt;
use std::path::{Path, PathBuf};

/// The stable name of an identity failure. The name is what an operator, a
/// test or a script matches on; the detail around it may be reworded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    /// The configuration file could not be read.
    ConfigUnreadable,
    /// The configuration file is not valid TOML of the declared shape.
    ConfigInvalid,
    /// The issuer origin is not an absolute origin lys accepts.
    IssuerInvalid,
    /// A client redirect URI is not an absolute URI lys accepts.
    RedirectInvalid,
    /// The database section names no usable address.
    DatabaseAddressInvalid,
    /// A client section is missing, duplicated or malformed.
    ClientInvalid,
    /// A credential the configuration requires is absent.
    SecretMissing,
    /// A credential file holds something other than a credential.
    SecretMalformed,
    /// A private file exists with a mode that others may read.
    PrivateFileModeOpen,
    /// A filesystem operation on a private file failed.
    PrivateFileIo,
    /// The rendered compose environment could not be written.
    RenderFailed,
    /// No connection to the Rauthy admin listener could be made.
    RauthyUnreachable,
    /// A request was sent and its outcome is unknown.
    RauthyUncertain,
    /// Rauthy refused the credential lys presented.
    RauthyUnauthorized,
    /// Rauthy refused the operation for the presented credential.
    RauthyForbidden,
    /// Rauthy refused the request as malformed.
    RauthyBadRequest,
    /// Rauthy answered with a server error.
    RauthyServerError,
    /// Rauthy answered with a status or body lys does not expect.
    RauthyUnexpected,
    /// A managed client read back differs from what was written.
    ReadBackMismatch,
    /// The declared theme mapping is malformed or breaks a theme rule.
    ThemeInvalid,
    /// At least one declared service is not ready.
    Unready,
    /// The data root holds no install to act on.
    NotInstalled,
    /// A binary the install runs is not in the folder named.
    BinaryMissing,
    /// A binary's `--version` could not be run or read.
    VersionUnreadable,
    /// A new build did not start; the previous one was put back.
    UpgradeFailed,
}

impl ErrorKind {
    /// The stable name printed at the start of every diagnostic.
    pub fn name(self) -> &'static str {
        match self {
            Self::ConfigUnreadable => "config_unreadable",
            Self::ConfigInvalid => "config_invalid",
            Self::IssuerInvalid => "issuer_invalid",
            Self::RedirectInvalid => "redirect_uri_invalid",
            Self::DatabaseAddressInvalid => "database_address_invalid",
            Self::ClientInvalid => "client_invalid",
            Self::SecretMissing => "secret_missing",
            Self::SecretMalformed => "secret_malformed",
            Self::PrivateFileModeOpen => "private_file_mode_open",
            Self::PrivateFileIo => "private_file_io",
            Self::RenderFailed => "render_failed",
            Self::RauthyUnreachable => "rauthy_unreachable",
            Self::RauthyUncertain => "rauthy_outcome_uncertain",
            Self::RauthyUnauthorized => "rauthy_unauthorized",
            Self::RauthyForbidden => "rauthy_forbidden",
            Self::RauthyBadRequest => "rauthy_bad_request",
            Self::RauthyServerError => "rauthy_server_error",
            Self::RauthyUnexpected => "rauthy_unexpected_response",
            Self::ReadBackMismatch => "read_back_mismatch",
            Self::ThemeInvalid => "theme_invalid",
            Self::Unready => "unready",
            Self::NotInstalled => "not_installed",
            Self::BinaryMissing => "binary_missing",
            Self::VersionUnreadable => "version_unreadable",
            Self::UpgradeFailed => "upgrade_failed",
        }
    }
}

/// A failure of `lys identity`, carrying operation, resource and path.
#[derive(Debug)]
pub struct IdentityError {
    kind: ErrorKind,
    operation: &'static str,
    resource: String,
    path: Option<PathBuf>,
    detail: String,
}

impl IdentityError {
    /// A failure of `operation` on `resource`, with a detail that must hold
    /// no secret bytes.
    pub fn new(
        kind: ErrorKind,
        operation: &'static str,
        resource: impl Into<String>,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            operation,
            resource: resource.into(),
            path: None,
            detail: detail.into(),
        }
    }

    /// The same failure, naming the file it touched.
    #[must_use]
    pub fn at(mut self, path: &Path) -> Self {
        self.path = Some(path.to_path_buf());
        self
    }

    /// The stable kind of this failure.
    pub fn kind(&self) -> ErrorKind {
        self.kind
    }
}

impl fmt::Display for IdentityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}: {} {}",
            self.kind.name(),
            self.operation,
            self.resource
        )?;
        if let Some(path) = &self.path {
            write!(f, " ({})", path.display())?;
        }
        if !self.detail.is_empty() {
            write!(f, ": {}", self.detail)?;
        }
        Ok(())
    }
}

impl std::error::Error for IdentityError {}

/// Convenience alias for `Result<T, IdentityError>`.
pub type IdentityResult<T> = Result<T, IdentityError>;

#[cfg(test)]
#[path = "error_tests.rs"]
mod tests;
