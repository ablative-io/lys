//! Typed deployment configuration for `lys identity`, and its validation.
//!
//! The operator's file (see deploy/identity/config.example.toml) is TOML,
//! deserialized straight into [`DeploymentConfig`] by the `toml` crate, so
//! the shape is declared once, by the types.
//!
//! # Invariants
//!
//! - A file that is not TOML, or not this shape, is `config_invalid` with the
//!   parser's own line and column, never approximated.
//! - No field has a default and unknown keys are refused, so a typo cannot
//!   fall back silently to a value nobody chose.
//! - The database host is never defaulted and never loopback: a loopback name
//!   inside a container names the container, and the database may live on a
//!   network device (ADR-005).
//! - The config carries no secret, so a diagnostic may echo a rejected value.
//!   Credentials live only in the private directory ([`super::credentials`]).

use std::fmt;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use super::error::IdentityError;

/// The login role and schema postgres-init.sql creates for Rauthy. Fixed
/// rather than configured: the init file and the compose file both name it.
pub const RAUTHY_DB_USER: &str = "rauthy";

/// The login role and schema postgres-init.sql creates for `SpiceDB`.
pub const SPICEDB_DB_USER: &str = "spicedb";

/// The whole deployment configuration file.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeploymentConfig {
    /// Node, compose project and private directory.
    pub deployment: DeploymentSection,
    /// The one `PostgreSQL` database.
    pub database: DatabaseSection,
    /// Rauthy's origins, listener and bootstrap administrator.
    pub rauthy: RauthySection,
    /// `SpiceDB`'s published listeners and readiness origin.
    pub spicedb: SpicedbSection,
    /// The two managed OIDC clients.
    pub clients: ClientsSection,
}

/// `[deployment]`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeploymentSection {
    /// The node the operator names for this instance (ADR-005).
    pub node: String,
    /// Compose project name.
    pub project: String,
    /// Private directory, relative to the config file's directory.
    pub private_dir: PathBuf,
}

/// `[database]`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DatabaseSection {
    /// Address Rauthy, `SpiceDB` and `health` dial.
    pub host: String,
    /// Port at that address.
    pub port: u16,
    /// The one database name.
    pub name: String,
    /// Bootstrap superuser for `PostgreSQL`'s init directory.
    pub admin_user: String,
    /// `disable`, `prefer` or `require`.
    pub sslmode: String,
    /// Host publish spec for the local `database` compose profile.
    pub publish: String,
}

/// `[rauthy]`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RauthySection {
    /// The origin browsers use; the issuer derives from it.
    pub public_origin: String,
    /// CIDR of the TLS-terminating proxy; empty for a loopback http origin.
    pub trusted_proxy: String,
    /// Plain-http origin of the node-local listener this CLI calls.
    pub admin_origin: String,
    /// Host publish spec for Rauthy's listener.
    pub publish: String,
    /// Bootstrap administrator's email (a test identity in rows 02 to 05).
    pub admin_email: String,
}

/// `[spicedb]`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpicedbSection {
    /// Plain-http origin where `health` reads readiness.
    pub http_origin: String,
    /// Host publish spec for the HTTP listener.
    pub http_publish: String,
    /// Host publish spec for the gRPC listener.
    pub grpc_publish: String,
}

/// `[clients.*]`: exactly the two clients configure manages.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClientsSection {
    /// The platform's own confidential client.
    pub platform: ClientSpec,
    /// Cambium's confidential client.
    pub cambium: ClientSpec,
}

impl ClientsSection {
    /// The managed clients, each with the config key it is declared under.
    pub fn managed(&self) -> [(&'static str, &ClientSpec); 2] {
        [("platform", &self.platform), ("cambium", &self.cambium)]
    }
}

/// One managed OIDC client.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClientSpec {
    /// Rauthy client id.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Exact redirect URIs.
    pub redirect_uris: Vec<String>,
    /// Exact post-logout redirect URIs.
    pub post_logout_redirect_uris: Vec<String>,
    /// Access and id token signing algorithm.
    pub signing_alg: String,
    /// Allowed PKCE challenge methods; empty for none.
    pub challenges: Vec<String>,
}

/// A validated `scheme://host[:port]` origin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Origin {
    /// Whether the scheme is https.
    pub https: bool,
    /// DNS name or IPv4 address.
    pub host: String,
    /// Explicit port, if one was given.
    pub port: Option<u16>,
}

impl Origin {
    /// Parses an origin, refusing any path, query, fragment or user part.
    pub fn parse(text: &str) -> Result<Self, &'static str> {
        let (https, rest) = if let Some(rest) = text.strip_prefix("https://") {
            (true, rest)
        } else if let Some(rest) = text.strip_prefix("http://") {
            (false, rest)
        } else {
            return Err("must begin with http:// or https://");
        };
        if rest.contains(['/', '?', '#', '@']) {
            return Err("must be scheme, host and optional port only");
        }
        let (host, port) = match rest.rsplit_once(':') {
            Some((host, port)) => {
                let port = port
                    .parse::<u16>()
                    .ok()
                    .filter(|port| *port != 0)
                    .ok_or("port must be 1 to 65535")?;
                (host, Some(port))
            }
            None => (rest, None),
        };
        if !is_hostname(host) {
            return Err("host must be a DNS name or an IPv4 address");
        }
        Ok(Self {
            https,
            host: host.to_string(),
            port,
        })
    }

    /// The port dialled: the explicit one, else the scheme's default.
    pub fn effective_port(&self) -> u16 {
        self.port.unwrap_or(if self.https { 443 } else { 80 })
    }

    /// `host[:port]` as written.
    pub fn authority(&self) -> String {
        match self.port {
            Some(port) => format!("{}:{port}", self.host),
            None => self.host.clone(),
        }
    }

    /// Whether the host is a loopback name.
    pub fn is_loopback(&self) -> bool {
        is_loopback_host(&self.host)
    }
}

impl fmt::Display for Origin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let scheme = if self.https { "https" } else { "http" };
        write!(f, "{scheme}://{}", self.authority())
    }
}

/// A config file that has been read and validated, with its derived values.
#[derive(Debug)]
pub struct LoadedConfig {
    /// The typed file.
    pub config: DeploymentConfig,
    /// Where it was read from.
    pub path: PathBuf,
    /// The private directory, resolved against the config file's directory.
    pub private_dir: PathBuf,
    /// The public origin the issuer derives from.
    pub public: Origin,
    /// The node-local origin this CLI calls Rauthy at.
    pub admin: Origin,
    /// The origin `health` reads `SpiceDB` readiness at.
    pub spicedb_http: Origin,
}

impl LoadedConfig {
    /// Reads, parses and validates the config at `path`.
    pub fn load(path: &Path) -> Result<Self, IdentityError> {
        let text = std::fs::read_to_string(path).map_err(|source| IdentityError::Io {
            operation: "read deployment config",
            path: path.to_path_buf(),
            source,
        })?;
        let config: DeploymentConfig =
            toml::from_str(&text).map_err(|error| IdentityError::ConfigInvalid {
                path: path.to_path_buf(),
                reason: error.to_string(),
            })?;
        let (public, admin, spicedb_http) = validate(&config)?;
        let base = path.parent().unwrap_or_else(|| Path::new("."));
        let private_dir = base.join(&config.deployment.private_dir);
        Ok(Self {
            config,
            path: path.to_path_buf(),
            private_dir,
            public,
            admin,
            spicedb_http,
        })
    }

    /// The issuer Rauthy derives: `scheme://pub_url/auth/v1/`.
    pub fn issuer(&self) -> String {
        format!("{}/auth/v1/", self.public)
    }
}

/// Validates every field, returning the public, admin and `SpiceDB` origins.
fn validate(config: &DeploymentConfig) -> Result<(Origin, Origin, Origin), IdentityError> {
    let deployment = &config.deployment;
    check(
        "deployment.node",
        &deployment.node,
        is_name(&deployment.node),
        "invalid_node",
        "must be 1 to 64 of A-Z a-z 0-9 . _ -",
    )?;
    let project_ok = deployment
        .project
        .starts_with(|c: char| c.is_ascii_lowercase())
        && deployment
            .project
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_');
    check(
        "deployment.project",
        &deployment.project,
        project_ok,
        "invalid_project",
        "must be a compose project name: lowercase letter, then a-z 0-9 _ -",
    )?;
    let private = deployment.private_dir.to_string_lossy();
    check(
        "deployment.private_dir",
        &private,
        !private.is_empty(),
        "invalid_private_dir",
        "must name a directory",
    )?;
    validate_database(&config.database)?;
    let public = validate_rauthy(&config.rauthy)?;
    let admin = local_origin("rauthy.admin_origin", &config.rauthy.admin_origin)?;
    let spicedb = &config.spicedb;
    let spicedb_http = local_origin("spicedb.http_origin", &spicedb.http_origin)?;
    for (field, value) in [
        ("spicedb.http_publish", &spicedb.http_publish),
        ("spicedb.grpc_publish", &spicedb.grpc_publish),
    ] {
        check(
            field,
            value,
            is_publish(value),
            "invalid_publish",
            PUBLISH_RULE,
        )?;
    }
    validate_clients(&config.clients)?;
    Ok((public, admin, spicedb_http))
}

const PUBLISH_RULE: &str = "must be PORT or IPV4:PORT";

fn validate_database(database: &DatabaseSection) -> Result<(), IdentityError> {
    check(
        "database.host",
        &database.host,
        is_hostname(&database.host),
        "invalid_database_host",
        "must be a DNS name or an IPv4 address the containers and this host both reach",
    )?;
    check(
        "database.host",
        &database.host,
        !is_loopback_host(&database.host),
        "invalid_database_host",
        "a loopback address names the container itself, not the database",
    )?;
    let port = database.port.to_string();
    check(
        "database.port",
        &port,
        database.port != 0,
        "invalid_database_port",
        "must be 1 to 65535",
    )?;
    for (field, value) in [
        ("database.name", &database.name),
        ("database.admin_user", &database.admin_user),
    ] {
        check(
            field,
            value,
            is_identifier(value),
            "invalid_identifier",
            "must be a lowercase PostgreSQL identifier of at most 63 characters",
        )?;
    }
    check(
        "database.admin_user",
        &database.admin_user,
        database.admin_user != RAUTHY_DB_USER && database.admin_user != SPICEDB_DB_USER,
        "invalid_identifier",
        "must differ from the service roles rauthy and spicedb",
    )?;
    check(
        "database.sslmode",
        &database.sslmode,
        matches!(database.sslmode.as_str(), "disable" | "prefer" | "require"),
        "invalid_sslmode",
        "must be one of disable, prefer, require",
    )?;
    check(
        "database.publish",
        &database.publish,
        is_publish(&database.publish),
        "invalid_publish",
        PUBLISH_RULE,
    )
}

fn validate_rauthy(rauthy: &RauthySection) -> Result<Origin, IdentityError> {
    let issuer = |reason| IdentityError::InvalidIssuer {
        value: rauthy.public_origin.clone(),
        reason,
    };
    let public = Origin::parse(&rauthy.public_origin).map_err(issuer)?;
    if !public.https && !public.is_loopback() {
        return Err(issuer(
            "plain http is accepted only for a loopback host; use an https origin behind a TLS proxy",
        ));
    }
    let proxy_ok = if public.https {
        rauthy.trusted_proxy.contains('/')
            && rauthy
                .trusted_proxy
                .chars()
                .all(|c| c.is_ascii_hexdigit() || matches!(c, '.' | ':' | '/'))
    } else {
        rauthy.trusted_proxy.is_empty()
    };
    check(
        "rauthy.trusted_proxy",
        &rauthy.trusted_proxy,
        proxy_ok,
        "invalid_trusted_proxy",
        "must be the proxy's CIDR for an https origin, and empty for an http one",
    )?;
    check(
        "rauthy.publish",
        &rauthy.publish,
        is_publish(&rauthy.publish),
        "invalid_publish",
        PUBLISH_RULE,
    )?;
    let email = &rauthy.admin_email;
    let email_ok = email.len() <= 254
        && email.split('@').count() == 2
        && email
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '@' | '.' | '_' | '-' | '+'));
    check(
        "rauthy.admin_email",
        email,
        email_ok,
        "invalid_admin_email",
        "must be one address of A-Z a-z 0-9 @ . _ - +",
    )?;
    Ok(public)
}

/// A node-local plain-http origin this CLI dials directly.
fn local_origin(field: &str, value: &str) -> Result<Origin, IdentityError> {
    let refuse = |reason| IdentityError::ConfigValue {
        name: "invalid_local_origin",
        field: field.to_string(),
        value: value.to_string(),
        reason,
    };
    let origin = Origin::parse(value).map_err(refuse)?;
    if origin.https {
        return Err(refuse(
            "must be plain http to the node-local listener; TLS belongs to the public origin",
        ));
    }
    Ok(origin)
}

fn validate_clients(clients: &ClientsSection) -> Result<(), IdentityError> {
    check(
        "clients.cambium.id",
        &clients.cambium.id,
        clients.cambium.id != clients.platform.id,
        "invalid_client_id",
        "the two managed clients must have different ids",
    )?;
    for (key, spec) in clients.managed() {
        let field = |name: &str| format!("clients.{key}.{name}");
        let id_ok = (2..=256).contains(&spec.id.len())
            && spec
                .id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'));
        check(
            &field("id"),
            &spec.id,
            id_ok,
            "invalid_client_id",
            "must be 2 to 256 of A-Z a-z 0-9 . _ -",
        )?;
        check(
            &field("id"),
            &spec.id,
            spec.id != "rauthy",
            "invalid_client_id",
            "the built-in rauthy client is never managed by configure",
        )?;
        let name_ok = (2..=128).contains(&spec.name.len())
            && spec
                .name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == ' ' || c == '-');
        check(
            &field("name"),
            &spec.name,
            name_ok,
            "invalid_client_name",
            "must be 2 to 128 of A-Z a-z 0-9 space -",
        )?;
        check(
            &field("signing_alg"),
            &spec.signing_alg,
            matches!(
                spec.signing_alg.as_str(),
                "RS256" | "RS384" | "RS512" | "EdDSA"
            ),
            "invalid_signing_alg",
            "must be one of RS256, RS384, RS512, EdDSA",
        )?;
        for challenge in &spec.challenges {
            check(
                &field("challenges"),
                challenge,
                challenge == "S256",
                "invalid_challenge",
                "only S256 is accepted",
            )?;
        }
        if spec.redirect_uris.is_empty() {
            return Err(IdentityError::InvalidRedirect {
                client: key,
                value: String::new(),
                reason: "at least one redirect URI is required",
            });
        }
        for uri in spec
            .redirect_uris
            .iter()
            .chain(&spec.post_logout_redirect_uris)
        {
            check_redirect(key, uri)?;
        }
    }
    Ok(())
}

/// Refuses a redirect URI that is not exact, absolute and acceptable.
pub fn check_redirect(client: &'static str, uri: &str) -> Result<(), IdentityError> {
    let refuse = |reason| IdentityError::InvalidRedirect {
        client,
        value: uri.to_string(),
        reason,
    };
    if uri.contains(['#', '*']) {
        return Err(refuse("must be exact: no fragment and no wildcard"));
    }
    if !uri
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || ",.:/_-&?=~!$'()+%".contains(c))
    {
        return Err(refuse("holds a character Rauthy refuses in a redirect URI"));
    }
    let (scheme, rest) = if let Some(rest) = uri.strip_prefix("https://") {
        ("https://", rest)
    } else if let Some(rest) = uri.strip_prefix("http://") {
        ("http://", rest)
    } else {
        return Err(refuse("must be an absolute http or https URI"));
    };
    let authority_end = rest.find(['/', '?']).unwrap_or(rest.len());
    let (authority, _) = rest.split_at(authority_end);
    let origin = Origin::parse(&format!("{scheme}{authority}")).map_err(refuse)?;
    if !origin.https && !origin.is_loopback() {
        return Err(refuse("plain http is accepted only for a loopback host"));
    }
    Ok(())
}

/// Returns the named value error when `ok` is false.
fn check(
    field: &str,
    value: &str,
    ok: bool,
    name: &'static str,
    reason: &'static str,
) -> Result<(), IdentityError> {
    if ok {
        Ok(())
    } else {
        Err(IdentityError::ConfigValue {
            name,
            field: field.to_string(),
            value: value.to_string(),
            reason,
        })
    }
}

fn is_hostname(host: &str) -> bool {
    !host.is_empty()
        && host.len() <= 253
        && !host.starts_with(['.', '-'])
        && !host.ends_with(['.', '-'])
        && host
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-')
}

fn is_loopback_host(host: &str) -> bool {
    host.eq_ignore_ascii_case("localhost") || host.starts_with("127.")
}

fn is_name(value: &str) -> bool {
    (1..=64).contains(&value.len())
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
}

fn is_identifier(value: &str) -> bool {
    value.len() <= 63
        && value.starts_with(|c: char| c.is_ascii_lowercase() || c == '_')
        && value
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

/// `PORT` or `IPV4:PORT`, the forms the compose file publishes.
fn is_publish(value: &str) -> bool {
    let (address, port) = value.rsplit_once(':').unwrap_or(("", value));
    let port_ok = port.parse::<u16>().is_ok_and(|port| port != 0);
    let address_ok = address.is_empty()
        || (address.split('.').count() == 4
            && address.split('.').all(|octet| octet.parse::<u8>().is_ok()));
    port_ok && address_ok
}

#[cfg(test)]
#[path = "config_tests.rs"]
mod tests;
