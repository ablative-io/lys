//! The deployment's credential material: generated once, owner-only, reused
//! on every run after.
//!
//! # Invariants
//!
//! - A [`Secret`] never formats its value. `Debug` and `Display` print its
//!   declared name and `[redacted]`, so a secret cannot reach a log, an error
//!   or `health` output by being formatted.
//! - Every buffer that holds a value is `Zeroizing`, wiped when dropped, and
//!   is allocated with room for the whole value so it never reallocates and
//!   leaves an unwiped copy behind.
//! - A credential is generated only while the deployment has never been
//!   prepared. Once it has, an absent credential is `secret_missing`: a new
//!   encryption key or database password would substitute an identity or
//!   strand the data it protects.
//! - A value is letters and digits, or `<id>/<base64>` for the encryption
//!   key, so it passes unquoted through the compose environment file and a
//!   `PostgreSQL` connection URI.

use std::fmt;
use std::path::Path;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use rand::Rng;
use rand::distr::Alphanumeric;
use zeroize::Zeroizing;

use super::error::IdentityError;
use super::private_files;

/// How a declared credential's value is formed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Form {
    /// Exactly that many ASCII letters and digits.
    Alphanumeric(usize),
    /// Rauthy's `ENC_KEYS` entry: `<8-character id>/<base64 of 32 random bytes>`.
    EncryptionKey,
}

/// One credential the deployment declares.
#[derive(Debug, Clone, Copy)]
pub struct CredentialSpec {
    /// Its name, which is also its file name under `secrets/`.
    pub name: &'static str,
    /// The environment variable deploy/identity/compose.yaml reads it from.
    pub env: &'static str,
    /// The form of its value.
    pub form: Form,
}

const fn spec(name: &'static str, env: &'static str, form: Form) -> CredentialSpec {
    CredentialSpec { name, env, form }
}

/// Every credential the deployment declares, in the order they are written.
pub const DECLARED: [CredentialSpec; 9] = [
    spec(
        "pg_admin_password",
        "LYS_IDENTITY_PG_ADMIN_PASSWORD",
        Form::Alphanumeric(40),
    ),
    spec(
        "rauthy_db_password",
        "LYS_IDENTITY_RAUTHY_DB_PASSWORD",
        Form::Alphanumeric(40),
    ),
    spec(
        "spicedb_db_password",
        "LYS_IDENTITY_SPICEDB_DB_PASSWORD",
        Form::Alphanumeric(40),
    ),
    spec(
        "rauthy_enc_key",
        "LYS_IDENTITY_RAUTHY_ENC_KEY",
        Form::EncryptionKey,
    ),
    spec(
        "rauthy_hql_secret_raft",
        "LYS_IDENTITY_RAUTHY_HQL_SECRET_RAFT",
        Form::Alphanumeric(40),
    ),
    spec(
        "rauthy_hql_secret_api",
        "LYS_IDENTITY_RAUTHY_HQL_SECRET_API",
        Form::Alphanumeric(40),
    ),
    spec(
        "rauthy_admin_password",
        "LYS_IDENTITY_RAUTHY_ADMIN_PASSWORD",
        Form::Alphanumeric(32),
    ),
    // Rauthy requires a bootstrap API key secret of at least 64 alphanumerics.
    spec(
        "rauthy_api_key_secret",
        "LYS_IDENTITY_RAUTHY_API_KEY_SECRET",
        Form::Alphanumeric(64),
    ),
    spec(
        "spicedb_preshared_key",
        "LYS_IDENTITY_SPICEDB_PRESHARED_KEY",
        Form::Alphanumeric(48),
    ),
];

/// The credential configure authenticates to Rauthy's API with.
pub const API_KEY_SECRET: &str = "rauthy_api_key_secret";

/// The credential holding Rauthy's encryption key.
pub const ENC_KEY: &str = "rauthy_enc_key";

/// Room for the longest value: an 8-character id, `/`, and 44 base64 characters.
const CAPACITY: usize = 64;

/// One credential's value. Formats as its name only.
pub struct Secret {
    name: &'static str,
    value: Zeroizing<String>,
}

impl Secret {
    /// The value, for the one place that must hand it on: the environment
    /// file, a connection, or an authorization header.
    pub fn expose(&self) -> &str {
        &self.value
    }

    /// For the encryption key, its id: the part before `/`, which names the
    /// key and is not secret.
    pub fn key_id(&self) -> Option<&str> {
        self.value.split_once('/').map(|(id, _)| id)
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Secret")
            .field("name", &self.name)
            .field("value", &"[redacted]")
            .finish()
    }
}

impl fmt::Display for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} [redacted]", self.name)
    }
}

/// Whether this run generated a credential or found it already present.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provenance {
    /// Absent before this run, generated and written by it.
    Generated,
    /// Present, validated and reused unchanged.
    Reused,
}

impl Provenance {
    /// The word output uses for it.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Generated => "generated",
            Self::Reused => "reused",
        }
    }
}

/// The whole declared credential set, in [`DECLARED`] order.
#[derive(Debug)]
pub struct Credentials {
    entries: Vec<(CredentialSpec, Secret, Provenance)>,
}

impl Credentials {
    /// Reuses every declared credential in `dir`; generates an absent one
    /// only when `may_generate`, and otherwise refuses it as missing.
    pub fn load_or_generate(dir: &Path, may_generate: bool) -> Result<Self, IdentityError> {
        let mut entries = Vec::with_capacity(DECLARED.len());
        for spec in DECLARED {
            let path = dir.join(spec.name);
            let (value, provenance) = match private_files::read_private(&path)? {
                Some(bytes) => {
                    let text = std::str::from_utf8(&bytes).ok().ok_or_else(|| {
                        IdentityError::SecretInvalid {
                            name: spec.name,
                            path: path.clone(),
                            reason: "not UTF-8",
                        }
                    })?;
                    let mut value = Zeroizing::new(String::with_capacity(CAPACITY));
                    value.push_str(text);
                    (value, Provenance::Reused)
                }
                None if may_generate => {
                    let value = generate(spec.form);
                    private_files::write_private(&path, value.as_bytes())?;
                    (value, Provenance::Generated)
                }
                None => {
                    return Err(IdentityError::SecretMissing {
                        name: spec.name,
                        path,
                    });
                }
            };
            check_form(spec.form, &value).map_err(|reason| IdentityError::SecretInvalid {
                name: spec.name,
                path: path.clone(),
                reason,
            })?;
            let secret = Secret {
                name: spec.name,
                value,
            };
            entries.push((spec, secret, provenance));
        }
        Ok(Self { entries })
    }

    /// Every credential with its spec and where it came from.
    pub fn iter(&self) -> impl Iterator<Item = (&CredentialSpec, &Secret, Provenance)> {
        self.entries
            .iter()
            .map(|(spec, secret, provenance)| (spec, secret, *provenance))
    }

    /// The credential declared as `name`, or `secret_missing` naming where
    /// it was expected under `dir`.
    pub fn get(&self, name: &'static str, dir: &Path) -> Result<&Secret, IdentityError> {
        self.entries
            .iter()
            .find(|(spec, _, _)| spec.name == name)
            .map(|(_, secret, _)| secret)
            .ok_or_else(|| IdentityError::SecretMissing {
                name,
                path: dir.join(name),
            })
    }
}

/// A fresh value of the given form from the thread's CSPRNG.
fn generate(form: Form) -> Zeroizing<String> {
    let mut rng = rand::rng();
    match form {
        Form::Alphanumeric(length) => alphanumeric(&mut rng, length),
        Form::EncryptionKey => {
            let mut key = Zeroizing::new([0_u8; 32]);
            rng.fill(&mut key[..]);
            let mut value = alphanumeric(&mut rng, 8);
            value.push('/');
            STANDARD.encode_string(key.as_slice(), &mut value);
            value
        }
    }
}

fn alphanumeric(rng: &mut impl Rng, length: usize) -> Zeroizing<String> {
    let mut value = Zeroizing::new(String::with_capacity(CAPACITY.max(length)));
    for _ in 0..length {
        value.push(char::from(rng.sample(Alphanumeric)));
    }
    value
}

/// Checks a value against its declared form, naming only the form.
fn check_form(form: Form, value: &str) -> Result<(), &'static str> {
    match form {
        Form::Alphanumeric(length) => {
            if value.len() == length && value.bytes().all(|byte| byte.is_ascii_alphanumeric()) {
                Ok(())
            } else {
                Err("not the declared number of ASCII letters and digits")
            }
        }
        Form::EncryptionKey => {
            let (id, key) = value.split_once('/').ok_or("not <id>/<base64 key>")?;
            let id_ok =
                (2..=20).contains(&id.len()) && id.bytes().all(|b| b.is_ascii_alphanumeric());
            let key_ok = STANDARD
                .decode(key)
                .ok()
                .map(Zeroizing::new)
                .is_some_and(|bytes| bytes.len() == 32);
            if id_ok && key_ok {
                Ok(())
            } else {
                Err("not an alphanumeric id and a base64 32-byte key")
            }
        }
    }
}

#[cfg(test)]
#[path = "credentials_tests.rs"]
mod tests;
