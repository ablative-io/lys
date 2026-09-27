//! Generated credential material: zeroized, redacted, and reused once made.
//!
//! Invariants:
//!
//! - A [`Secret`] lives in a [`Zeroizing`] buffer and its `Debug` and
//!   `Display` forms are the literal `<redacted>`; the only way to its bytes
//!   is [`Secret::expose`], called where the bytes leave for the private env
//!   file or an `Authorization` header.
//! - Every credential is generated here from the operating system's CSPRNG
//!   (through `rand`'s thread generator) and written once, mode `0600`, under
//!   `<state_dir>/credentials/`. A credential that exists is reused, never
//!   regenerated: the database roles, Rauthy's encryption key and its API key
//!   were initialised with it, so rotating it silently would lock the product
//!   out of its own data.
//! - A credential's name, never its value, is what appears in errors.

use std::fmt;
use std::path::{Path, PathBuf};

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use rand::distr::Alphanumeric;
use rand::{Rng, RngCore};
use zeroize::{Zeroize, Zeroizing};

use crate::identity::error::{IdentityError, IdentityResult};
use crate::identity::private_files::{self, WriteOutcome};

/// The subdirectory of the state directory that holds credentials.
pub const CREDENTIALS_DIR: &str = "credentials";

/// Rauthy's encryption key id (`ENC_KEY_ACTIVE`).
pub const ENC_KEY_ID: &str = "identity1";

/// The name of the Rauthy API key configure authenticates with.
pub const API_KEY_NAME: &str = "lys_identity";

/// Secret material. Never printed.
pub struct Secret(Zeroizing<String>);

impl Secret {
    /// Wrap a value obtained elsewhere (e.g. a client secret Rauthy issued).
    pub fn new(value: String) -> Self {
        Self(Zeroizing::new(value))
    }

    /// The secret's bytes, for the one place they must leave.
    pub fn expose(&self) -> &str {
        &self.0
    }

    fn alphanumeric(length: usize) -> Self {
        let mut rng = rand::rng();
        let value: String = (0..length)
            .map(|_| char::from(rng.sample(Alphanumeric)))
            .collect();
        Self::new(value)
    }

    fn encryption_key() -> Self {
        let mut bytes = Zeroizing::new([0_u8; 32]);
        rand::rng().fill_bytes(&mut *bytes);
        Self::new(STANDARD.encode(bytes.as_slice()))
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Secret(<redacted>)")
    }
}

impl fmt::Display for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("<redacted>")
    }
}

/// Every credential `prepare` generates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CredentialKind {
    /// The `PostgreSQL` superuser password (local service only).
    PostgresSuperuser,
    /// The `identity_rauthy` role's password.
    RauthyDatabase,
    /// The `identity_spicedb` role's password.
    SpicedbDatabase,
    /// Rauthy's 32-byte encryption key, base64.
    RauthyEncryptionKey,
    /// Rauthy's Hiqlite Raft secret.
    HiqliteRaft,
    /// Rauthy's Hiqlite API secret.
    HiqliteApi,
    /// The secret of the Rauthy API key configure uses.
    RauthyApiKey,
    /// The bootstrap administrator's password (a test identity).
    RauthyAdminPassword,
    /// `SpiceDB`'s `gRPC` preshared key.
    SpicedbPresharedKey,
}

impl CredentialKind {
    /// Every kind, in the order they are written.
    pub const ALL: [CredentialKind; 9] = [
        CredentialKind::PostgresSuperuser,
        CredentialKind::RauthyDatabase,
        CredentialKind::SpicedbDatabase,
        CredentialKind::RauthyEncryptionKey,
        CredentialKind::HiqliteRaft,
        CredentialKind::HiqliteApi,
        CredentialKind::RauthyApiKey,
        CredentialKind::RauthyAdminPassword,
        CredentialKind::SpicedbPresharedKey,
    ];

    /// The credential's name: its file name and its name in errors.
    pub fn name(self) -> &'static str {
        match self {
            CredentialKind::PostgresSuperuser => "postgres_superuser",
            CredentialKind::RauthyDatabase => "rauthy_database",
            CredentialKind::SpicedbDatabase => "spicedb_database",
            CredentialKind::RauthyEncryptionKey => "rauthy_encryption_key",
            CredentialKind::HiqliteRaft => "rauthy_hiqlite_raft",
            CredentialKind::HiqliteApi => "rauthy_hiqlite_api",
            CredentialKind::RauthyApiKey => "rauthy_api_key",
            CredentialKind::RauthyAdminPassword => "rauthy_admin_password",
            CredentialKind::SpicedbPresharedKey => "spicedb_preshared_key",
        }
    }

    /// The credential's path under `state_dir`.
    pub fn path(self, state_dir: &Path) -> PathBuf {
        state_dir.join(CREDENTIALS_DIR).join(self.name())
    }

    fn generate(self) -> Secret {
        match self {
            CredentialKind::RauthyEncryptionKey => Secret::encryption_key(),
            // Rauthy requires an API key secret of at least 64 characters.
            CredentialKind::RauthyApiKey => Secret::alphanumeric(64),
            _ => Secret::alphanumeric(48),
        }
    }
}

/// The full set of generated credentials.
#[derive(Debug)]
pub struct Credentials {
    secrets: Vec<(CredentialKind, Secret)>,
}

impl Credentials {
    /// Read every credential under `state_dir`, generating and writing each
    /// one that does not exist yet. Returns the per-file outcomes.
    pub fn load_or_generate(state_dir: &Path) -> IdentityResult<(Self, Vec<(PathBuf, WriteOutcome)>)> {
        private_files::ensure_private_dir(&state_dir.join(CREDENTIALS_DIR), CREDENTIALS_DIR)?;
        let mut secrets = Vec::with_capacity(CredentialKind::ALL.len());
        let mut outcomes = Vec::with_capacity(CredentialKind::ALL.len());
        for kind in CredentialKind::ALL {
            let path = kind.path(state_dir);
            let fresh = kind.generate();
            let (bytes, outcome) =
                private_files::create_once(&path, fresh.expose().as_bytes(), kind.name())?;
            secrets.push((kind, secret_from_bytes(bytes, kind.name())?));
            outcomes.push((path, outcome));
        }
        Ok((Self { secrets }, outcomes))
    }

    /// The credential of `kind`.
    pub fn get(&self, kind: CredentialKind) -> IdentityResult<&Secret> {
        self.secrets
            .iter()
            .find(|(held, _)| *held == kind)
            .map(|(_, secret)| secret)
            .ok_or_else(|| IdentityError::SecretMissing {
                resource: kind.name().to_string(),
                path: PathBuf::from(CREDENTIALS_DIR).join(kind.name()),
            })
    }
}

/// Read one credential without generating anything.
pub fn load_one(state_dir: &Path, kind: CredentialKind) -> IdentityResult<Secret> {
    let bytes = private_files::read_private(&kind.path(state_dir), kind.name())?;
    secret_from_bytes(bytes, kind.name())
}

/// The path a managed client's Rauthy-issued secret is kept at.
pub fn client_secret_path(state_dir: &Path, client_id: &str) -> PathBuf {
    state_dir
        .join(CREDENTIALS_DIR)
        .join(format!("client_{client_id}"))
}

/// Keep a client secret Rauthy issued, once. An existing file is reused.
pub fn store_client_secret(
    state_dir: &Path,
    client_id: &str,
    secret: &Secret,
) -> IdentityResult<WriteOutcome> {
    let path = client_secret_path(state_dir, client_id);
    let resource = format!("client_{client_id}");
    private_files::create_once(&path, secret.expose().as_bytes(), &resource)
        .map(|(_, outcome)| outcome)
}

fn secret_from_bytes(mut bytes: Zeroizing<Vec<u8>>, name: &str) -> IdentityResult<Secret> {
    let owned = std::mem::take(&mut *bytes);
    let printable = !owned.is_empty() && owned.iter().all(u8::is_ascii_graphic);
    match String::from_utf8(owned) {
        Ok(text) if printable => Ok(Secret::new(text)),
        Ok(mut text) => {
            text.zeroize();
            Err(not_a_credential(name))
        }
        Err(error) => {
            error.into_bytes().zeroize();
            Err(not_a_credential(name))
        }
    }
}

fn not_a_credential(name: &str) -> IdentityError {
    IdentityError::InvalidConfigValue {
        field: format!("{CREDENTIALS_DIR}/{name}"),
        value: String::new(),
        reason: "is not a generated credential (printable ASCII, no whitespace)",
    }
}

#[cfg(test)]
#[path = "credentials_tests.rs"]
mod tests;
