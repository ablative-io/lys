//! Credential material: generated once, held zeroizing, shown redacted, and
//! reused unchanged on every later `prepare` so a venue keeps its identity.

use std::fmt::{self, Write as _};
use std::path::Path;

use rand::RngCore;
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

use super::error::{IdentityError, IdentityResult};
use super::private_files::write_private;

/// The file inside the venue that holds every generated value, so a second
/// `prepare` renders the same services.
pub const CREDENTIALS_FILE: &str = "credentials.json";

/// A secret string. `Debug` and `Display` print a fixed marker, never the
/// value; the bytes are zeroed when dropped.
#[derive(Clone, PartialEq, Eq)]
pub struct Secret(Zeroizing<String>);

impl Serialize for Secret {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for Secret {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        String::deserialize(deserializer).map(|text| Self(Zeroizing::new(text)))
    }
}

impl Secret {
    /// The value, for the one place that writes it into a private file.
    #[must_use]
    pub fn expose(&self) -> &str {
        &self.0
    }

    /// `bytes` random bytes as lower-case hex: URL-safe, quote-safe, TOML-safe.
    #[must_use]
    pub fn hex(bytes: usize) -> Self {
        let mut buf = vec![0u8; bytes];
        rand::rng().fill_bytes(&mut buf);
        let mut text = String::with_capacity(bytes * 2);
        for byte in &buf {
            // Writing into a String cannot fail.
            let _ = write!(text, "{byte:02x}");
        }
        Self(Zeroizing::new(text))
    }

    /// `len` random characters from `[A-Za-z0-9]`, for values Rauthy requires
    /// alphanumeric.
    #[must_use]
    pub fn alphanumeric(len: usize) -> Self {
        const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
        let mut text = String::with_capacity(len);
        let mut rng = rand::rng();
        while text.len() < len {
            let byte = (rng.next_u32() & 0xff) as usize;
            if byte < ALPHABET.len() * 4 {
                text.push(char::from(ALPHABET[byte % ALPHABET.len()]));
            }
        }
        Self(Zeroizing::new(text))
    }

    /// 32 random bytes in standard base64, the shape Rauthy's encryption key
    /// takes after its id.
    #[must_use]
    pub fn key_base64() -> Self {
        use base64::Engine as _;
        let mut buf = [0u8; 32];
        rand::rng().fill_bytes(&mut buf);
        Self(Zeroizing::new(
            base64::engine::general_purpose::STANDARD.encode(buf),
        ))
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

/// Every generated value of a venue.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Credentials {
    /// `PostgreSQL`'s administrative role password.
    pub postgres_admin_password: Secret,
    /// The `rauthy` role password.
    pub rauthy_db_password: Secret,
    /// The `spicedb` role password.
    pub spicedb_db_password: Secret,
    /// `SpiceDB`'s `gRPC` preshared key.
    pub spicedb_preshared_key: Secret,
    /// Hiqlite's raft secret.
    pub rauthy_raft_secret: Secret,
    /// Hiqlite's API secret.
    pub rauthy_api_secret: Secret,
    /// Hiqlite's dashboard password, before its base64 wrapping.
    pub rauthy_dashboard_password: Secret,
    /// Rauthy's active encryption key, 32 bytes base64.
    pub rauthy_encryption_key: Secret,
    /// The first administrator's password.
    pub rauthy_admin_password: Secret,
    /// The secret of the API key `configure` uses.
    pub rauthy_api_key_secret: Secret,
}

/// The id of the one encryption key `prepare` declares.
pub const ENCRYPTION_KEY_ID: &str = "lys1";

/// The name of the API key `configure` authenticates with.
pub const API_KEY_NAME: &str = "lys-configure";

impl Credentials {
    /// Fresh values for a new venue.
    #[must_use]
    pub fn generate() -> Self {
        Self {
            postgres_admin_password: Secret::hex(24),
            rauthy_db_password: Secret::hex(24),
            spicedb_db_password: Secret::hex(24),
            spicedb_preshared_key: Secret::hex(24),
            rauthy_raft_secret: Secret::hex(24),
            rauthy_api_secret: Secret::hex(24),
            rauthy_dashboard_password: Secret::hex(24),
            rauthy_encryption_key: Secret::key_base64(),
            rauthy_admin_password: Secret::hex(12),
            rauthy_api_key_secret: Secret::alphanumeric(64),
        }
    }

    /// The venue's credentials: the ones on record when the file exists,
    /// otherwise fresh ones, written before anything else is rendered from
    /// them so a later `prepare` reuses exactly these.
    pub fn load_or_generate(venue: &Path) -> IdentityResult<(Self, bool)> {
        let path = venue.join(CREDENTIALS_FILE);
        match std::fs::read(&path) {
            Ok(bytes) => {
                let held = serde_json::from_slice(&bytes).map_err(|error| IdentityError::Config {
                    operation: "read the venue credentials",
                    path: path.clone(),
                    detail: format!(
                        "the file is not the credentials record prepare writes (line {}, column {})",
                        error.line(),
                        error.column()
                    ),
                })?;
                Ok((held, false))
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let fresh = Self::generate();
                let bytes =
                    serde_json::to_vec_pretty(&fresh).map_err(|error| IdentityError::Config {
                        operation: "write the venue credentials",
                        path: path.clone(),
                        detail: format!(
                            "the credentials could not be encoded: {:?}",
                            error.classify()
                        ),
                    })?;
                write_private(&path, &bytes)?;
                Ok((fresh, true))
            }
            Err(source) => Err(IdentityError::Io {
                operation: "read the venue credentials",
                path,
                source,
            }),
        }
    }

    /// The API key as Rauthy's `Authorization` header wants it.
    #[must_use]
    pub fn api_key_header(&self) -> Secret {
        Secret(Zeroizing::new(format!(
            "API-Key {API_KEY_NAME}${}",
            self.rauthy_api_key_secret.expose()
        )))
    }

    /// Read the API key header back from an existing venue, for `configure`.
    pub fn api_key_header_from(venue: &Path) -> IdentityResult<Secret> {
        let (held, _) = Self::load_or_generate(venue)?;
        Ok(held.api_key_header())
    }
}
