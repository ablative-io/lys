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
    #[serde(skip)]
    base: PathBuf,
}

/// The `[deployment]` table.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Deployment {
    /// The node the operator names for this install.
    pub node: String,
    /// The compose project name.
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

/// The `[clients]` table: the platform's client, and Cambium's where a
/// configuration still names it. An install names only the platform's: a
/// product registers itself as a client of Lys, never of the issuer.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Clients {
    /// The platform's own confidential client, themed identity orange.
    pub platform: Client,
    /// Cambium's client, themed Cambium green, when a configuration names it.
    #[serde(default)]
    pub cambium: Option<Client>,
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
    /// Lys's own policy, from NIST SP 800-63B-4 section 3.1.1.2 for a
    /// password that is the only factor: at least 15 characters, up to 64
    /// allowed, and no rule on which kinds of character it holds.
    fn default() -> Self {
        Self {
            length_min: 15,
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

const TOKEN_ALGORITHMS: [&str; 4] = ["RS256", "RS384", "RS512", "EdDSA"];
const TLS_MODES: [&str; 3] = ["disable", "prefer", "require"];
const LOOPBACK_HOSTS: [&str; 3] = ["localhost", "127.0.0.1", "[::1]"];

fn refuse(kind: ErrorKind, resource: &str, detail: impl Into<String>) -> IdentityError {
    IdentityError::new(kind, "validate configuration", resource, detail)
}

impl DeploymentConfig {
    /// Reads and validates the configuration at `path`.
    pub fn load(path: &Path) -> IdentityResult<Self> {
        let text = std::fs::read_to_string(path).map_err(|error| {
            IdentityError::new(
                ErrorKind::ConfigUnreadable,
                "read configuration",
                "configuration",
                error.to_string(),
            )
            .at(path)
        })?;
        let base = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf();
        Self::parse(&text, base).map_err(|error| error.at(path))
    }

    /// Parses and validates configuration text whose relative paths resolve
    /// against `base`.
    pub fn parse(text: &str, base: PathBuf) -> IdentityResult<Self> {
        let mut config: Self = toml::from_str(text)
            .map_err(|error| refuse(ErrorKind::ConfigInvalid, "configuration", error.message()))?;
        config.base = base;
        config.validate()?;
        Ok(config)
    }

    /// The private state directory, resolved against the configuration file.
    pub fn state_dir(&self) -> PathBuf {
        self.base.join(&self.deployment.state_dir)
    }

    /// The managed clients, platform first.
    pub fn managed_clients(&self) -> Vec<(ClientRole, &Client)> {
        let mut clients = vec![(ClientRole::Platform, &self.clients.platform)];
        if let Some(cambium) = &self.clients.cambium {
            clients.push((ClientRole::Cambium, cambium));
        }
        clients
    }

    /// Rauthy's `PUB_URL`: the public origin's authority.
    pub fn pub_url(&self) -> &str {
        authority_of(&self.issuer.public_origin)
    }

    /// The relying-party id for passkeys: the public origin's host.
    pub fn rp_id(&self) -> &str {
        let authority = self.pub_url();
        if authority.starts_with('[') {
            return authority
                .split_once(']')
                .map_or(authority, |(host, _)| host.trim_start_matches('['));
        }
        authority.split(':').next().unwrap_or(authority)
    }

    /// The compose network's gateway: the address the directory service
    /// reaches the sign-in service from.
    pub fn gateway(&self) -> IdentityResult<Ipv4Addr> {
        network_gateway(&self.deployment.network)
    }

    /// Whether the public origin is served over TLS by a fronting proxy.
    pub fn public_tls(&self) -> bool {
        self.issuer.public_origin.starts_with("https://")
    }

    /// Rauthy's `COOKIE_MODE`. A browser keeps a `Secure` cookie only over
    /// TLS, so an origin served over plain http, which is accepted for a
    /// loopback host alone, asks for the session cookie without that flag;
    /// without it no session survives the sign-in. An https origin keeps
    /// Rauthy's host-bound `Secure` cookie.
    pub fn cookie_mode(&self) -> &'static str {
        if self.public_tls() {
            "host"
        } else {
            "danger-insecure"
        }
    }

    fn validate(&self) -> IdentityResult<()> {
        self.validate_deployment()?;
        self.password_policy.validate()?;
        validate_origin(&self.issuer.public_origin)?;
        if self.public_tls() && self.issuer.trusted_proxies.is_empty() {
            return Err(refuse(
                ErrorKind::IssuerInvalid,
                "issuer.trusted_proxies",
                "an https origin runs the sign-in service in proxy mode, which answers its trusted proxies alone; name the network's gateway in trusted_proxies",
            ));
        }
        let gateway = self.gateway()?.to_string();
        let proxies_ok = self
            .issuer
            .trusted_proxies
            .iter()
            .all(|proxy| proxy.strip_suffix("/32").unwrap_or(proxy) == gateway);
        if !proxies_ok {
            return Err(refuse(
                ErrorKind::IssuerInvalid,
                "issuer.trusted_proxies",
                format!(
                    "the sign-in service takes a person's address from the directory service alone, which reaches it from {gateway}, the gateway of deployment.network"
                ),
            ));
        }
        validate_admin_url(&self.issuer.admin_url)?;
        self.validate_database()?;
        if self
            .clients
            .cambium
            .as_ref()
            .is_some_and(|cambium| cambium.id == self.clients.platform.id)
        {
            return Err(refuse(
                ErrorKind::ClientInvalid,
                "clients",
                "the platform and Cambium clients need distinct ids",
            ));
        }
        for (role, client) in self.managed_clients() {
            validate_client(role, client)?;
        }
        Ok(())
    }

    fn validate_deployment(&self) -> IdentityResult<()> {
        let deployment = &self.deployment;
        if deployment.node.trim().is_empty() {
            return Err(refuse(
                ErrorKind::ConfigInvalid,
                "deployment.node",
                "name the node this instance is installed on",
            ));
        }
        let project_ok = !deployment.project.is_empty()
            && deployment.project.bytes().all(|byte| {
                byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-' || byte == b'_'
            });
        if !project_ok {
            return Err(refuse(
                ErrorKind::ConfigInvalid,
                "deployment.project",
                "use lowercase letters, digits, - and _ only",
            ));
        }
        if let Some(email) = &deployment.admin_email
            && !is_email(email)
        {
            return Err(refuse(
                ErrorKind::ConfigInvalid,
                "deployment.admin_email",
                "expected an email address",
            ));
        }
        Ok(())
    }

    fn validate_database(&self) -> IdentityResult<()> {
        let database = &self.database;
        let host_ok = !database.host.is_empty()
            && !database
                .host
                .contains(|c: char| c.is_whitespace() || "/@?#:".contains(c));
        if !host_ok {
            return Err(refuse(
                ErrorKind::DatabaseAddressInvalid,
                "database.host",
                "expected a host name or address, without scheme, port or credentials",
            ));
        }
        if database.port == 0 {
            return Err(refuse(
                ErrorKind::DatabaseAddressInvalid,
                "database.port",
                "expected a port from 1 to 65535",
            ));
        }
        if database.bundled != (database.host == BUNDLED_DATABASE_HOST) {
            return Err(refuse(
                ErrorKind::DatabaseAddressInvalid,
                "database.host",
                "the bundled service answers as postgres; a database elsewhere is named by its own address with bundled = false",
            ));
        }
        if database.bundled && database.publish_port.is_none() {
            return Err(refuse(
                ErrorKind::DatabaseAddressInvalid,
                "database.publish_port",
                "the bundled service needs a loopback port to be published on",
            ));
        }
        let check_ok = database
            .check_address
            .rsplit_once(':')
            .is_some_and(|(host, port)| {
                !host.is_empty() && port.parse::<u16>().is_ok_and(|p| p > 0)
            });
        if !check_ok {
            return Err(refuse(
                ErrorKind::DatabaseAddressInvalid,
                "database.check_address",
                "expected host:port",
            ));
        }
        if !TLS_MODES.contains(&database.tls.as_str()) {
            return Err(refuse(
                ErrorKind::DatabaseAddressInvalid,
                "database.tls",
                "expected disable, prefer or require",
            ));
        }
        let name_ok = !database.name.is_empty()
            && database
                .name
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_');
        if !name_ok {
            return Err(refuse(
                ErrorKind::DatabaseAddressInvalid,
                "database.name",
                "use lowercase letters, digits and _ only",
            ));
        }
        Ok(())
    }
}

/// Which managed client a configuration entry is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientRole {
    /// The platform's own client.
    Platform,
    /// Cambium's client.
    Cambium,
}

impl ClientRole {
    /// The role's key in the configuration and the theme mapping.
    pub fn key(self) -> &'static str {
        match self {
            Self::Platform => "platform",
            Self::Cambium => "cambium",
        }
    }
}

/// Whether `text` is shaped as an email address: one `@` with text on each
/// side, a dot inside the domain, and no space, control character, quote,
/// backslash or character an address carries in its path or query.
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
