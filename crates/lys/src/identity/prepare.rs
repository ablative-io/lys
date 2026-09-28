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

use super::config::{CredentialSource, DeploymentConfig};
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
pub fn declared_private_files() -> Vec<&'static str> {
    SECRETS
        .iter()
        .map(|spec| spec.file)
        .chain([COMPOSE_ENV])
        .collect()
}

/// Reads a declared credential from the state directory, refusing a missing
/// one by name.
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

/// Renders the compose environment from the configuration and credentials.
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
    let lines: [(&str, String); 23] = [
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
        (
            "RAUTHY_RP_ORIGIN",
            config
                .issuer
                .public_origin
                .trim_end_matches('/')
                .to_string(),
        ),
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

/// Materialises every declared credential and the compose environment for
/// `config`, answering each private file with its outcome, in order.
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
    Ok(outcomes)
}

/// Runs `lys identity prepare` against the configuration at `config_path`.
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
        declared_private_files().len(),
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
