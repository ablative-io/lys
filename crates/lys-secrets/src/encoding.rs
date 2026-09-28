//! Canonical length-prefixed encoding, hex, digests, secure random bytes and
//! constant-time comparison.

use rand::TryRngCore;
use rand::rngs::OsRng;
use sha2::{Digest, Sha256};

use crate::error::SecretsError;

/// Lowercase hex of `bytes`.
pub(crate) fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut text = String::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        text.push(char::from(DIGITS[usize::from(byte >> 4)]));
        text.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    text
}

/// The bytes lowercase or uppercase hex `text` spells, or `None` if it is
/// not hex of an even length.
pub(crate) fn unhex(text: &str) -> Option<Vec<u8>> {
    fn nibble(c: u8) -> Option<u8> {
        match c {
            b'0'..=b'9' => Some(c - b'0'),
            b'a'..=b'f' => Some(c - b'a' + 10),
            b'A'..=b'F' => Some(c - b'A' + 10),
            _ => None,
        }
    }
    let raw = text.as_bytes();
    if raw.len() % 2 != 0 {
        return None;
    }
    raw.chunks_exact(2)
        .map(|pair| Some((nibble(pair[0])? << 4) | nibble(pair[1])?))
        .collect()
}

/// SHA-256 of `bytes`.
pub(crate) fn sha256(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

/// Whether `a` and `b` are equal, in time independent of where they differ.
pub(crate) fn ct_eq(a: &[u8; 32], b: &[u8; 32]) -> bool {
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

/// `N` bytes from the operating system's secure random source.
pub(crate) fn random_bytes<const N: usize>() -> Result<[u8; N], SecretsError> {
    let mut bytes = [0u8; N];
    OsRng
        .try_fill_bytes(&mut bytes)
        .map_err(|source| SecretsError::Random {
            reason: source.to_string(),
        })?;
    Ok(bytes)
}

/// A canonical encoding: a domain string, then fields each prefixed with its
/// length as a big-endian `u32`.
pub(crate) struct Canonical(Vec<u8>);

impl Canonical {
    /// An encoding opened by `domain`.
    pub(crate) fn new(domain: &str) -> Result<Self, SecretsError> {
        let mut encoding = Self(Vec::new());
        encoding.field(domain.as_bytes())?;
        Ok(encoding)
    }

    /// Appends one length-prefixed field.
    pub(crate) fn field(&mut self, bytes: &[u8]) -> Result<&mut Self, SecretsError> {
        let len = u32::try_from(bytes.len())
            .ok()
            .ok_or_else(|| SecretsError::Encoding {
                context: "canonical field",
                reason: format!("a field of {} bytes exceeds the 4 GiB limit", bytes.len()),
            })?;
        self.0.extend_from_slice(&len.to_be_bytes());
        self.0.extend_from_slice(bytes);
        Ok(self)
    }

    /// Appends one `u64` field.
    pub(crate) fn number(&mut self, value: u64) -> Result<&mut Self, SecretsError> {
        self.field(&value.to_be_bytes())
    }

    /// The encoded bytes.
    pub(crate) fn into_bytes(self) -> Vec<u8> {
        self.0
    }
}

/// Reads a [`Canonical`] encoding field by field.
pub(crate) struct Reader<'a> {
    rest: &'a [u8],
    context: &'static str,
}

impl<'a> Reader<'a> {
    /// A reader over `bytes`, whose failures are reported under `context`.
    pub(crate) fn new(bytes: &'a [u8], context: &'static str) -> Self {
        Self {
            rest: bytes,
            context,
        }
    }

    fn short(&self, wanted: usize) -> SecretsError {
        SecretsError::Encoding {
            context: self.context,
            reason: format!("wanted {wanted} more bytes and {} remain", self.rest.len()),
        }
    }

    /// The next length-prefixed field.
    pub(crate) fn field(&mut self) -> Result<&'a [u8], SecretsError> {
        let (len_bytes, after_len) = self.rest.split_at_checked(4).ok_or_else(|| self.short(4))?;
        let mut len_array = [0u8; 4];
        len_array.copy_from_slice(len_bytes);
        let len = usize::try_from(u32::from_be_bytes(len_array))
            .ok()
            .ok_or_else(|| SecretsError::Encoding {
                context: self.context,
                reason: "a field length does not fit this platform".to_string(),
            })?;
        let (field, rest) = after_len
            .split_at_checked(len)
            .ok_or_else(|| self.short(len))?;
        self.rest = rest;
        Ok(field)
    }

    /// Whether every byte has been read.
    pub(crate) fn is_done(&self) -> bool {
        self.rest.is_empty()
    }

    /// The next field, read as a `u64`.
    pub(crate) fn number(&mut self) -> Result<u64, SecretsError> {
        let field = self.field()?;
        let array: [u8; 8] = field
            .try_into()
            .ok()
            .ok_or_else(|| SecretsError::Encoding {
                context: self.context,
                reason: format!("a number field holds {} bytes, not 8", field.len()),
            })?;
        Ok(u64::from_be_bytes(array))
    }
}
