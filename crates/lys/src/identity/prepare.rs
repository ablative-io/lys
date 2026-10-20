//! `lys identity prepare`: validate the configuration and materialise the
//! declared private deployment artifacts.
//!
//! The artifacts are the credentials the three dependency processes share
//! and the compose environment deploy/identity/compose.yaml interpolates.
//! Each is an owner-only file in the state directory. A credential that
//! exists is reused byte for byte; one that is missing is generated only
//! when the configuration says credentials are generated, and refused by
//! name when it says they are provided.

use std::fmt::Write as _;
use std::path::Path;

use base64::Engine;
use zeroize::Zeroizing;

use super::config::{CredentialSource, DeploymentConfig, LOOPBACK_HOSTS, MailTls};
use super::credentials::{Credential, ENCRYPTION_KEY_ID, Shape};
use super::error::{ErrorKind, IdentityError, IdentityResult};
use super::private_files::{self, Outcome};
use crate::commands::output::Emitter;

/// One declared credential: its file in the state directory, the compose
/// variable it is rendered as, and its shape.
#[derive(Debug, Clone, Copy)]
pub struct SecretSpec {
    /// The file name under the state directory.
    pub file: &'static str,
    /// The compose environment variable.
    pub variable: &'static str,
    /// How it is generated and checked.
    pub shape: Shape,
}

/// The bootstrap API key's name, which configure presents with its secret.
pub const API_KEY_NAME: &str = "lys_configure";

/// The issuer's own bootstrap account, which it makes on its first start
/// whatever it is told: a machine account at a fixed address that names no
/// person, whose generated password nobody reads. People are made on Lys's
/// setup page, never here.
pub const BOOTSTRAP_ACCOUNT_EMAIL: &str = "bootstrap@machine.lys.test";

/// The header the directory service names a person's own address in when it
/// signs them in; the sign-in service takes it from its trusted proxies alone.
pub const FORWARDED_FOR: &str = "X-Forwarded-For";

/// The rendered compose environment's file name.
pub const COMPOSE_ENV: &str = "compose.env";

/// The owner-only file the sign-in service's SMTP settings are rendered to,
/// read by compose as the issuer's `env_file` (DIRECTORY-110 R2); with no
/// `[mail]` the issuer's `env_file` is `/dev/null` and it has no SMTP.
pub const MAIL_ENV: &str = "rauthy-smtp.env";

/// The provided credential holding the mail user's password, read when
/// `[mail]` names a user; never generated: it is a server's own.
pub const MAIL_PASSWORD_FILE: &str = "mail-password";

/// Every declared credential, in the order they are materialised.
pub const SECRETS: [SecretSpec; 9] = [
    SecretSpec {
        file: "postgres-superuser-password",
        variable: "IDENTITY_POSTGRES_SUPERUSER_PASSWORD",
        shape: Shape::Alphanumeric(48),
    },
    SecretSpec {
        file: "rauthy-db-password",
        variable: "RAUTHY_DB_PASSWORD",
        shape: Shape::Alphanumeric(48),
    },
    SecretSpec {
        file: "spicedb-db-password",
        variable: "SPICEDB_DB_PASSWORD",
        shape: Shape::Alphanumeric(48),
    },
    SecretSpec {
        file: "spicedb-preshared-key",
        variable: "SPICEDB_PRESHARED_KEY",
        shape: Shape::Alphanumeric(48),
    },
    SecretSpec {
        file: "rauthy-raft-secret",
        variable: "RAUTHY_HQL_SECRET_RAFT",
        shape: Shape::Alphanumeric(48),
    },
    SecretSpec {
        file: "rauthy-api-secret",
        variable: "RAUTHY_HQL_SECRET_API",
        shape: Shape::Alphanumeric(48),
    },
    SecretSpec {
        file: "rauthy-encryption-key",
        variable: "RAUTHY_ENC_KEYS",
        shape: Shape::EncryptionKey,
    },
    SecretSpec {
        file: "rauthy-bootstrap-api-secret",
        variable: "RAUTHY_BOOTSTRAP_API_KEY_SECRET",
        shape: Shape::Alphanumeric(64),
    },
    SecretSpec {
        file: "rauthy-admin-password",
        variable: "RAUTHY_BOOTSTRAP_ADMIN_PASSWORD",
        shape: Shape::Alphanumeric(32),
    },
];

/// The bootstrap API key credential, by its declared spec.
pub const API_KEY_SECRET: SecretSpec = SECRETS[7];

/// The file names of every private artifact prepare declares.
#[must_use]
pub fn declared_private_files(config: &DeploymentConfig) -> Vec<&'static str> {
    let mail = config.mail.as_ref();
    SECRETS
        .iter()
        .map(|spec| spec.file)
        .chain([COMPOSE_ENV])
        .chain(mail.map(|_| MAIL_ENV))
        .chain(
            mail.and_then(|mail| mail.user.as_ref())
                .map(|_| MAIL_PASSWORD_FILE),
        )
        .collect()
}

/// Reads a declared credential from the state directory, refusing a missing
/// one by name.
///
/// # Errors
/// Returns named secret-file, template or materialisation failures.
pub fn read_secret(state_dir: &Path, spec: SecretSpec) -> IdentityResult<Credential> {
    let path = state_dir.join(spec.file);
    match private_files::read(&path)? {
        Some(bytes) => {
            Credential::from_stored(spec.file, &bytes, spec.shape).map_err(|e| e.at(&path))
        }
        None => Err(IdentityError::new(
            ErrorKind::SecretMissing,
            "read credential",
            spec.file,
            "run lys identity prepare, or place the provided credential in the state directory",
        )
        .at(&path)),
    }
}

fn materialise(
    state_dir: &Path,
    spec: SecretSpec,
    source: CredentialSource,
) -> IdentityResult<(Credential, Outcome)> {
    let path = state_dir.join(spec.file);
    if let Some(bytes) = private_files::read(&path)? {
        let credential =
            Credential::from_stored(spec.file, &bytes, spec.shape).map_err(|e| e.at(&path))?;
        return Ok((credential, Outcome::Unchanged));
    }
    if source == CredentialSource::Provided {
        return Err(IdentityError::new(
            ErrorKind::SecretMissing,
            "prepare",
            spec.file,
            "credentials.source is provided, so a missing credential is not generated",
        )
        .at(&path));
    }
    let credential = Credential::generate(spec.file, spec.shape);
    let outcome = private_files::write(&path, credential.expose().as_bytes())?;
    Ok((credential, outcome))
}

/// The base64 JSON Rauthy reads as its one bootstrap API key, the install's
/// configure key: read, create and update on clients, users, sign-in
/// providers and API keys, read and update on secrets, nothing else. The
/// password policy, which Rauthy keeps under secrets, is Lys's own and is
/// written by the install. The directory service never holds this key: the
/// install makes it one of its own with the API key rights, without Secrets
/// update or any right over keys (`install::directory_key`).
#[must_use]
pub fn bootstrap_api_key() -> String {
    let request = serde_json::json!({
        "name": API_KEY_NAME,
        "exp": null,
        "access": [
            {"group": "Clients", "access_rights": ["read", "create", "update"]},
            {"group": "Secrets", "access_rights": ["read", "update"]},
            {"group": "Users", "access_rights": ["read", "create", "update"]},
            {"group": "AuthProviders", "access_rights": ["read", "create", "update"]},
            {"group": "ApiKeys", "access_rights": ["read", "create", "update"]},
        ],
    });
    base64::engine::general_purpose::STANDARD.encode(request.to_string())
}

/// The public origin as Rauthy's passkey origin: with its port always
/// written, the scheme's own (443 for https, 80 for http) when the origin
/// names none. Rauthy reads the port as the text after the origin's last
/// `:` and refuses to start without one; the issuer's name, from
/// `RAUTHY_PUB_URL`, keeps the origin as configured.
fn passkey_origin(config: &DeploymentConfig) -> String {
    let origin = config.issuer.public_origin.trim_end_matches('/');
    let authority = config.pub_url();
    let host_end = if authority.starts_with('[') {
        authority.find(']').map_or(authority.len(), |end| end + 1)
    } else {
        0
    };
    if authority[host_end..].contains(':') {
        return origin.to_owned();
    }
    let port = if config.public_tls() { 443 } else { 80 };
    format!("{origin}:{port}")
}

/// Renders the compose environment from the configuration and credentials.
///
/// # Errors
/// Returns named secret-file, template or materialisation failures.
pub fn render_env(
    config: &DeploymentConfig,
    credentials: &[(SecretSpec, Credential)],
) -> IdentityResult<Zeroizing<String>> {
    let mut env = Zeroizing::new(String::new());
    let database = &config.database;
    let publish_port = database
        .publish_port
        .map_or(String::new(), |p| p.to_string());
    let profiles = if database.bundled { "bundled-db" } else { "" };
    let lines: [(&str, String); 24] = [
        ("COMPOSE_PROJECT_NAME", config.deployment.project.clone()),
        ("COMPOSE_PROFILES", profiles.to_string()),
        ("IDENTITY_NODE", config.deployment.node.clone()),
        ("IDENTITY_DB_HOST", database.host.clone()),
        ("IDENTITY_DB_PORT", database.port.to_string()),
        ("IDENTITY_DB_NAME", database.name.clone()),
        ("IDENTITY_DB_TLS", database.tls.clone()),
        ("IDENTITY_DB_PUBLISH_PORT", publish_port),
        ("IDENTITY_NETWORK", config.deployment.network.clone()),
        ("IDENTITY_GATEWAY", config.gateway()?.to_string()),
        ("RAUTHY_LISTEN_PORT", config.issuer.listen_port.to_string()),
        ("RAUTHY_PUB_URL", config.pub_url().to_string()),
        ("RAUTHY_RP_ID", config.rp_id().to_string()),
        ("RAUTHY_RP_ORIGIN", passkey_origin(config)),
        // The sign-in service names itself on the public origin's scheme: it
        // calls itself https only in proxy mode, so proxy mode follows an
        // https origin and nothing else.
        ("RAUTHY_PROXY_MODE", config.public_tls().to_string()),
        ("RAUTHY_COOKIE_MODE", config.cookie_mode().to_string()),
        // Every password sign-in reaches the issuer from the directory
        // service, which names the person's own address in
        // X-Forwarded-For; the issuer takes that header from the trusted
        // proxies alone (the network's gateway), so one person's failures
        // bar only that person. A configuration that trusts no proxy leaves
        // the header name empty, which names no header, rather than
        // trusting every sender.
        (
            "RAUTHY_PEER_IP_HEADER_NAME",
            if config.issuer.trusted_proxies.is_empty() {
                String::new()
            } else {
                FORWARDED_FOR.to_string()
            },
        ),
        (
            "RAUTHY_TRUSTED_PROXIES",
            format!("\"{}\"", config.issuer.trusted_proxies.join("\\n")),
        ),
        ("RAUTHY_ENC_KEY_ACTIVE", ENCRYPTION_KEY_ID.to_string()),
        (
            "RAUTHY_BOOTSTRAP_ADMIN_EMAIL",
            BOOTSTRAP_ACCOUNT_EMAIL.to_string(),
        ),
        ("RAUTHY_BOOTSTRAP_API_KEY", bootstrap_api_key()),
        ("SPICEDB_GRPC_PORT", config.spicedb.grpc_port.to_string()),
        ("SPICEDB_HTTP_PORT", config.spicedb.http_port.to_string()),
        // The issuer's SMTP settings are its own env_file, so that with no
        // [mail] none of its SMTP keys is set, not even empty.
        (
            "IDENTITY_SMTP_ENV_FILE",
            config.mail.as_ref().map_or_else(
                || "/dev/null".to_owned(),
                |_| config.state_dir().join(MAIL_ENV).display().to_string(),
            ),
        ),
    ];
    for (name, value) in &lines {
        if value.contains(['\n', '\r']) {
            return Err(IdentityError::new(
                ErrorKind::RenderFailed,
                "render environment",
                *name,
                "a value may not span lines",
            ));
        }
        writeln!(env, "{name}={value}").map_err(|error| {
            IdentityError::new(
                ErrorKind::RenderFailed,
                "render environment",
                *name,
                error.to_string(),
            )
        })?;
    }
    for (spec, credential) in credentials {
        writeln!(env, "{}={}", spec.variable, credential.expose()).map_err(|error| {
            IdentityError::new(
                ErrorKind::RenderFailed,
                "render environment",
                spec.variable,
                error.to_string(),
            )
        })?;
    }
    Ok(env)
}

/// The mail user's password from its provided file under `state`, when
/// `[mail]` names a user; none when it names no user.
///
/// # Errors
/// A named user with no password file is refused `SecretMissing` naming the
/// file: a server's password cannot be generated here. A password that is
/// not one line of text, or holds a single quote, is refused
/// `SecretMalformed`, since the env file quotes it.
pub fn mail_password(
    config: &DeploymentConfig,
    state: &Path,
) -> IdentityResult<Option<Zeroizing<String>>> {
    let Some(user) = config.mail.as_ref().and_then(|mail| mail.user.as_ref()) else {
        return Ok(None);
    };
    let path = state.join(MAIL_PASSWORD_FILE);
    let Some(bytes) = private_files::read(&path)? else {
        return Err(IdentityError::new(
            ErrorKind::SecretMissing,
            "read credential",
            MAIL_PASSWORD_FILE,
            format!(
                "place the password of the mail user {user} in the state directory; a server's password cannot be generated here"
            ),
        )
        .at(&path));
    };
    let malformed = |detail: &str| {
        IdentityError::new(
            ErrorKind::SecretMalformed,
            "read credential",
            MAIL_PASSWORD_FILE,
            detail,
        )
        .at(&path)
    };
    let text = Zeroizing::new(
        String::from_utf8(bytes.to_vec())
            .map_err(|error| malformed(&format!("the password is not text: {error}")))?,
    );
    let password = text.trim_end_matches(['\r', '\n']);
    if password.is_empty() || password.contains(['\'', '\n', '\r']) {
        return Err(malformed("the password is one line with no single quote"));
    }
    Ok(Some(Zeroizing::new(password.to_owned())))
}

/// The issuer's SMTP environment from `[mail]`, none without it: the keys
/// the pinned sign-in service reads (`SMTP_URL`, `SMTP_PORT`, `SMTP_FROM`,
/// the mode flag, `SMTP_USERNAME`, `SMTP_PASSWORD`), each value single-quoted so
/// compose takes it literally. A loopback mail host is reached from the container
/// at the network's gateway.
///
/// # Errors
/// Refuses a configuration whose gateway cannot be named.
pub fn render_mail_env(
    config: &DeploymentConfig,
    password: Option<&str>,
) -> IdentityResult<Option<Zeroizing<String>>> {
    let Some(mail) = &config.mail else {
        return Ok(None);
    };
    let host = if LOOPBACK_HOSTS.contains(&mail.host.as_str()) {
        config.gateway()?.to_string()
    } else {
        mail.host.clone()
    };
    let mut lines = vec![
        ("SMTP_URL", host),
        ("SMTP_PORT", mail.port.to_string()),
        ("SMTP_FROM", mail.from.clone()),
    ];
    // The pinned issuer's modes are two flags, not one key: TLS from the
    // first byte is its default (with a downgrade to STARTTLS when TLS
    // fails), SMTP_STARTTLS_ONLY takes STARTTLS straight away, and
    // SMTP_DANGER_INSECURE allows plain; its config.toml at v0.36.2.
    match mail.tls {
        MailTls::Implicit => {}
        MailTls::Starttls => lines.push(("SMTP_STARTTLS_ONLY", "true".to_owned())),
        MailTls::None => lines.push(("SMTP_DANGER_INSECURE", "true".to_owned())),
    }
    // The pinned issuer reads SMTP_USERNAME and SMTP_PASSWORD before it looks
    // at the mode and ends at boot when either is unset (its mailer.rs, an
    // expect on each); in danger-insecure mode it then uses neither. So a
    // relay with no sign-in, which validation holds to tls = "none", is given
    // a pair of placeholders the issuer never sends.
    let (user, password) = match (&mail.user, password) {
        (Some(user), Some(password)) => (user.clone(), password.to_owned()),
        _ => ("unused".to_owned(), "unused".to_owned()),
    };
    lines.push(("SMTP_USERNAME", user));
    lines.push(("SMTP_PASSWORD", password));
    let mut env = Zeroizing::new(String::new());
    for (name, value) in lines {
        writeln!(env, "{name}='{value}'").map_err(|error| {
            IdentityError::new(
                ErrorKind::RenderFailed,
                "render mail environment",
                name,
                error.to_string(),
            )
        })?;
    }
    Ok(Some(env))
}

/// Materialises every declared credential and the compose environment for
/// `config`, answering each private file with its outcome, in order.
///
/// # Errors
/// Returns named secret-file, template or materialisation failures.
pub fn materialise_all(config: &DeploymentConfig) -> IdentityResult<Vec<(&'static str, Outcome)>> {
    let state_dir = config.state_dir();
    private_files::ensure_dir(&state_dir)?;
    let mut credentials = Vec::with_capacity(SECRETS.len());
    let mut outcomes = Vec::with_capacity(SECRETS.len() + 1);
    for spec in SECRETS {
        let (credential, outcome) = materialise(&state_dir, spec, config.credentials.source)?;
        outcomes.push((spec.file, outcome));
        credentials.push((spec, credential));
    }
    let env = render_env(config, &credentials)?;
    let env_outcome = private_files::write(&state_dir.join(COMPOSE_ENV), env.as_bytes())?;
    outcomes.push((COMPOSE_ENV, env_outcome));
    let password = mail_password(config, &state_dir)?;
    if let Some(mail) = render_mail_env(config, password.as_deref().map(String::as_str))? {
        let outcome = private_files::write(&state_dir.join(MAIL_ENV), mail.as_bytes())?;
        outcomes.push((MAIL_ENV, outcome));
    }
    Ok(outcomes)
}

/// Runs `lys identity prepare` against the configuration at `config_path`.
///
/// # Errors
/// Returns named secret-file, template or materialisation failures.
pub fn run(config_path: &Path, json: bool) -> IdentityResult<()> {
    let config = DeploymentConfig::load(config_path)?;
    let mut emitter = Emitter::new(json);
    emitter.field("node", "node", config.deployment.node.clone());
    emitter.field(
        "state",
        "state_dir",
        config.state_dir().display().to_string(),
    );
    let mut outcomes = serde_json::Map::new();
    for (file, outcome) in materialise_all(&config)? {
        emitter.note(&format!("{file} {} (mode 600)", outcome.word()));
        outcomes.insert(file.to_string(), outcome.word().into());
    }
    emitter.field(
        "private files",
        "private_files",
        declared_private_files(&config).len(),
    );
    if emitter.is_json() {
        emitter.field("", "outcomes", serde_json::Value::Object(outcomes));
    }
    emitter.finish();
    Ok(())
}

#[cfg(test)]
#[path = "prepare_tests.rs"]
mod tests;
