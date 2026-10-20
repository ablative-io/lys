//! Typed deployment configuration for `lys identity`, read from TOML and
//! validated before anything is written or sent.
//!
//! The configuration holds addresses, names and choices, never a secret:
//! credentials live in owner-only files under the state directory. Every
//! diagnostic names the field and the rule it broke; none echoes a value
//! that could be a credential.

use std::net::Ipv4Addr;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::error::{ErrorKind, IdentityError, IdentityResult};

mod bring_forward;
mod deployment;
pub use deployment::ClientRole;
mod validate;

use validate::{
    authority_of, network_gateway, validate_admin_url, validate_client, validate_origin,
};

/// The compose service name the bundled `PostgreSQL` answers to.
pub const BUNDLED_DATABASE_HOST: &str = "postgres";

/// The whole deployment configuration, validated.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeploymentConfig {
    /// Where the deployment runs and keeps its private state.
    pub deployment: Deployment,
    /// Rauthy's public origin and the address lys reaches its admin API at.
    pub issuer: Issuer,
    /// The one `PostgreSQL` database Rauthy and `SpiceDB` share.
    pub database: Database,
    /// `SpiceDB`'s published listeners.
    pub spicedb: SpiceDb,
    /// Where credentials come from.
    pub credentials: Credentials,
    /// The two OIDC clients `lys identity configure` manages.
    pub clients: Clients,
    /// The passwords Lys takes; Lys's own policy when the table is absent.
    #[serde(default)]
    pub password_policy: PasswordPolicy,
    /// The identity settings (ACCESS-002 R2); their stated defaults when
    /// the table is absent.
    #[serde(default)]
    pub identity: super::install::settings::IdentitySettings,
    /// The `[mail]` table (DIRECTORY-110 R1): the server Lys and the sign-in
    /// service send letters through; absent, Lys admits nobody and the
    /// sign-in service sends nothing.
    #[serde(default)]
    pub mail: Option<Mail>,
    #[serde(skip)]
    base: PathBuf,
}

/// The `[deployment]` table.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Deployment {
    /// The node the operator names for this install.
    pub node: String,
    /// The compose project name, and the name of the install's own
    /// organisation, founded at first-run setup with the administrator as
    /// founder or at the first boot of a store written before organisations
    /// (DIRECTORY-109 R2).
    pub project: String,
    /// The private state directory, relative to the configuration file.
    pub state_dir: PathBuf,
    /// The private IPv4 range the compose network is given, written from its
    /// first address. Its gateway, the range's next address, is the one
    /// address the directory service on this machine reaches the sign-in
    /// service from.
    pub network: String,
    /// The administrator's email an unattended install was given, the only
    /// email the setup page then takes. Absent, the person types their own
    /// on the setup page; nothing is ever filled from the machine.
    #[serde(default)]
    pub admin_email: Option<String>,
}

/// The `[issuer]` table.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Issuer {
    /// The public origin browsers reach Rauthy at, scheme and authority only.
    pub public_origin: String,
    /// The loopback port Rauthy's HTTP listener is published on.
    pub listen_port: u16,
    /// The URL lys reaches Rauthy's admin API at, over loopback HTTP.
    pub admin_url: String,
    /// The addresses the sign-in service takes a person's own address from:
    /// the network's gateway, which the directory service reaches it from, or
    /// none, when every sign-in counts as the service's own address.
    #[serde(default)]
    pub trusted_proxies: Vec<String>,
}

/// The `[database]` table.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Database {
    /// Whether the compose file's own `PostgreSQL` service is the database.
    pub bundled: bool,
    /// The host Rauthy and `SpiceDB` connect to.
    pub host: String,
    /// The port Rauthy and `SpiceDB` connect to.
    pub port: u16,
    /// The database name.
    pub name: String,
    /// The TLS mode: `disable`, `prefer` or `require`.
    pub tls: String,
    /// The `host:port` this machine reaches the same database at.
    pub check_address: String,
    /// The loopback port the bundled service is published on.
    pub publish_port: Option<u16>,
}

/// The `[spicedb]` table.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpiceDb {
    /// The loopback port `SpiceDB`'s gRPC listener is published on.
    pub grpc_port: u16,
    /// The loopback port `SpiceDB`'s HTTP listener is published on.
    pub http_port: u16,
}

/// The `[credentials]` table.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Credentials {
    /// `generate` creates missing credentials; `provided` refuses them.
    pub source: CredentialSource,
}

/// Where credentials come from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CredentialSource {
    /// Missing credentials are generated once and reused afterwards.
    Generate,
    /// Every credential must already exist; a missing one is refused.
    Provided,
}

/// The `[clients]` table: the platform's client, and one app's client where a
/// configuration names it. An install names only the platform's: every app
/// registers itself through the directory's Apps API, never with the issuer.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Clients {
    /// The platform's own confidential client, themed identity orange.
    pub platform: Client,
    /// The one app client installed beside the platform's, themed with the
    /// app accent the theme mapping declares, when a configuration names it.
    #[serde(default)]
    pub app: Option<Client>,
}

/// One managed OIDC client.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Client {
    /// The client id Rauthy registers.
    pub id: String,
    /// The display name on the login page.
    pub name: String,
    /// Exact redirect URIs.
    pub redirect_uris: Vec<String>,
    /// Exact post-logout redirect URIs.
    #[serde(default)]
    pub post_logout_redirect_uris: Vec<String>,
    /// The signing algorithm of this client's tokens.
    pub token_alg: String,
    /// The PKCE challenge methods this client may use; empty for none.
    #[serde(default)]
    pub challenges: Vec<String>,
}

/// The `[password_policy]` table: the passwords Lys takes.
///
/// Lys owns this policy. The install writes it to the sign-in service, which
/// enforces it on every password set, and hands the same values to the
/// directory service, whose setup and account screens show them before a
/// password is sent. The sign-in service is never asked for its own. A
/// length is counted as the sign-in service counts it, in bytes of UTF-8.
/// Each bound is one the sign-in service can hold, and a value outside it is
/// refused by name before anything is written.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PasswordPolicy {
    /// The fewest characters a password has, 8 to 128.
    pub length_min: u16,
    /// The most characters a password has, `length_min` to 128.
    pub length_max: u16,
    /// The fewest lower-case letters, 1 to 32, when any are asked for.
    #[serde(default)]
    pub lower_case: Option<u16>,
    /// The fewest upper-case letters, 1 to 32, when any are asked for.
    #[serde(default)]
    pub upper_case: Option<u16>,
    /// The fewest digits, 1 to 32, when any are asked for.
    #[serde(default)]
    pub digits: Option<u16>,
    /// The fewest other characters, 1 to 32, when any are asked for.
    #[serde(default)]
    pub special: Option<u16>,
    /// How many of a person's last passwords a new one may not be, 1 to 10,
    /// when any are refused.
    #[serde(default)]
    pub not_recently_used: Option<u16>,
}

impl Default for PasswordPolicy {
    /// Lys's own policy: at least 8 characters, up to 64 allowed, and no
    /// rule on which kinds of character it holds. Tom, 15 Oct 2026: fifteen
    /// characters cannot be remembered without writing them down; NIST
    /// SP 800-63B-4 allows 8 where a second factor or sign-in provider is
    /// available, as Google sign-in is here.
    fn default() -> Self {
        Self {
            length_min: 8,
            length_max: 64,
            lower_case: None,
            upper_case: None,
            digits: None,
            special: None,
            not_recently_used: None,
        }
    }
}

impl PasswordPolicy {
    fn validate(&self) -> IdentityResult<()> {
        let field = |name: &str| format!("password_policy.{name}");
        if !(8..=128).contains(&self.length_min) {
            return Err(refuse(
                ErrorKind::ConfigInvalid,
                &field("length_min"),
                "expected 8 to 128",
            ));
        }
        if !(self.length_min..=128).contains(&self.length_max) {
            return Err(refuse(
                ErrorKind::ConfigInvalid,
                &field("length_max"),
                "expected length_min to 128",
            ));
        }
        let counts = [
            ("lower_case", self.lower_case, 32),
            ("upper_case", self.upper_case, 32),
            ("digits", self.digits, 32),
            ("special", self.special, 32),
            ("not_recently_used", self.not_recently_used, 10),
        ];
        for (name, value, most) in counts {
            if value.is_some_and(|value| !(1..=most).contains(&value)) {
                return Err(refuse(
                    ErrorKind::ConfigInvalid,
                    &field(name),
                    format!("expected 1 to {most}, or the field left out"),
                ));
            }
        }
        Ok(())
    }
}

/// The `[mail]` table: the SMTP server letters go through. Its password is
/// never here; it is the provided credential file `mail-password` under
/// state, read by `prepare` when `user` is set.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Mail {
    /// The server's host name or address.
    pub host: String,
    /// Its port.
    pub port: u16,
    /// The user the server is signed in to as; none for a server that takes
    /// no sign-in.
    #[serde(default)]
    pub user: Option<String>,
    /// The sender every letter carries: an address, or `Name <address>`.
    pub from: String,
    /// How the connection is secured.
    pub tls: MailTls,
}

/// How the mail server's connection is secured: TLS from the first byte,
/// upgraded after the greeting, or none, which only a loopback host may use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MailTls {
    /// TLS from the first byte.
    Implicit,
    /// Upgraded after the greeting.
    Starttls,
    /// Plain.
    None,
}

impl MailTls {
    /// The word the configuration and the service configuration use.
    #[must_use]
    pub fn word(self) -> &'static str {
        match self {
            Self::Implicit => "implicit",
            Self::Starttls => "starttls",
            Self::None => "none",
        }
    }
}

const TOKEN_ALGORITHMS: [&str; 4] = ["RS256", "RS384", "RS512", "EdDSA"];
const TLS_MODES: [&str; 3] = ["disable", "prefer", "require"];
pub(crate) const LOOPBACK_HOSTS: [&str; 3] = ["localhost", "127.0.0.1", "[::1]"];

fn refuse(kind: ErrorKind, resource: &str, detail: impl Into<String>) -> IdentityError {
    IdentityError::new(kind, "validate configuration", resource, detail)
}

impl ClientRole {
    /// The role's key in the configuration and the theme mapping.
    #[must_use]
    pub fn key(self) -> &'static str {
        match self {
            Self::Platform => "platform",
            Self::App => "app",
        }
    }
}

/// Whether `text` is a sender a letter may carry: an address, or a name
/// then an address in angle brackets, as `Lys <lys@example.test>`.
#[must_use]
pub fn is_mailbox(text: &str) -> bool {
    if let Some((name, rest)) = text.split_once('<') {
        return !name.trim().is_empty()
            && !name.contains('>')
            && rest
                .strip_suffix('>')
                .is_some_and(|address| !address.contains('>') && is_email(address));
    }
    is_email(text)
}

/// Whether `text` is shaped as an email address: one `@` with text on each
/// side, a dot inside the domain, and no space, control character, quote,
/// backslash or character an address carries in its path or query.
#[must_use]
pub fn is_email(text: &str) -> bool {
    let refused = |c: char| {
        c.is_whitespace() || c.is_control() || matches!(c, '/' | '?' | '#' | '%' | '"' | '\\')
    };
    !text.chars().any(refused)
        && text.split_once('@').is_some_and(|(local, domain)| {
            !local.is_empty()
                && domain.contains('.')
                && !domain.starts_with('.')
                && !domain.ends_with('.')
                && !domain.contains('@')
        })
}

#[cfg(test)]
#[path = "config_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "config_move_tests.rs"]
mod move_tests;
