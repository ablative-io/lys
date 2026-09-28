//! [`Credential`]: generated or received secret material, wiped on drop and
//! redacted wherever it is formatted.
//!
//! A credential is reused, never rotated, once it exists: preparation reads
//! the file it was written to and hands back the same bytes, so a second
//! `lys identity prepare` leaves every service able to reach the others.

use std::fmt;

use base64::Engine;
use rand::{Rng, RngCore};
use zeroize::Zeroizing;

use super::error::{ErrorKind, IdentityError, IdentityResult};

/// The encryption key id Rauthy is given beside its generated key.
pub const ENCRYPTION_KEY_ID: &str = "lys01";

/// The form a credential takes, which fixes how it is generated and checked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    /// ASCII letters and digits, of exactly this many characters.
    Alphanumeric(usize),
    /// Rauthy's encryption key: `lys01/` then 32 random bytes in base64.
    EncryptionKey,
}

/// Secret material. Its `Debug` and `Display` forms name it and never show
/// a byte of it; the bytes are zeroed when it is dropped.
pub struct Credential {
    name: String,
    value: Zeroizing<String>,
}

impl Credential {
    /// Generates a fresh credential of `shape` from the thread CSPRNG.
    pub fn generate(name: &str, shape: Shape) -> Self {
        let mut value = Zeroizing::new(String::new());
        match shape {
            Shape::Alphanumeric(length) => {
                let mut rng = rand::rng();
                value.reserve(length);
                for _ in 0..length {
                    value.push(char::from(rng.sample(rand::distr::Alphanumeric)));
                }
            }
            Shape::EncryptionKey => {
                let mut key = Zeroizing::new([0_u8; 32]);
                rand::rng().fill_bytes(key.as_mut());
                value.push_str(ENCRYPTION_KEY_ID);
                value.push('/');
                base64::engine::general_purpose::STANDARD.encode_string(key.as_ref(), &mut value);
            }
        }
        Self {
            name: name.to_string(),
            value,
        }
    }

    /// Reads a credential back from the bytes of the file that holds it,
    /// refusing anything that is not of `shape`. A trailing newline an
    /// operator's editor added is tolerated; nothing else is.
    pub fn from_stored(name: &str, bytes: &[u8], shape: Shape) -> IdentityResult<Self> {
        let trimmed = bytes.strip_suffix(b"\n").unwrap_or(bytes);
        let text = std::str::from_utf8(trimmed).map_err(|utf8| {
            IdentityError::new(
                ErrorKind::SecretMalformed,
                "read credential",
                name,
                format!("not UTF-8 after byte {}", utf8.valid_up_to()),
            )
        })?;
        let credential = Self {
            name: name.to_string(),
            value: Zeroizing::new(text.to_string()),
        };
        credential.check(shape)?;
        Ok(credential)
    }

    /// Wraps a credential another service generated, such as a client
    /// secret Rauthy returns.
    pub fn received(name: &str, value: Zeroizing<String>) -> Self {
        Self {
            name: name.to_string(),
            value,
        }
    }

    /// The credential's name, which is safe to print.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The secret itself, for the one place that must hand it on.
    pub fn expose(&self) -> &str {
        &self.value
    }

    fn check(&self, shape: Shape) -> IdentityResult<()> {
        let refuse = |detail: String| {
            IdentityError::new(
                ErrorKind::SecretMalformed,
                "read credential",
                self.name.clone(),
                detail,
            )
        };
        match shape {
            Shape::Alphanumeric(length) => {
                if self.value.len() != length {
                    return Err(refuse(format!(
                        "expected {length} characters, found {}",
                        self.value.len()
                    )));
                }
                if !self.value.bytes().all(|byte| byte.is_ascii_alphanumeric()) {
                    return Err(refuse("expected only ASCII letters and digits".to_string()));
                }
            }
            Shape::EncryptionKey => {
                let Some(encoded) = self
                    .value
                    .strip_prefix(ENCRYPTION_KEY_ID)
                    .and_then(|rest| rest.strip_prefix('/'))
                else {
                    return Err(refuse(format!(
                        "expected the key id {ENCRYPTION_KEY_ID} and a slash"
                    )));
                };
                let decoded = Zeroizing::new(
                    base64::engine::general_purpose::STANDARD
                        .decode(encoded)
                        .map_err(|decode| refuse(format!("key is not base64: {decode}")))?,
                );
                if decoded.len() != 32 {
                    return Err(refuse(format!(
                        "expected a 32-byte key, found {} bytes",
                        decoded.len()
                    )));
                }
            }
        }
        Ok(())
    }
}

impl fmt::Debug for Credential {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Credential")
            .field("name", &self.name)
            .field("value", &"[redacted]")
            .finish()
    }
}

impl fmt::Display for Credential {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} [redacted]", self.name)
    }
}

#[cfg(test)]
#[path = "credentials_tests.rs"]
mod tests;
