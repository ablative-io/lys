//! The redacting type every credential, handle and key byte lives in.

use std::fmt;

use zeroize::Zeroizing;

/// Secret bytes.
///
/// `Debug` and `Display` print `[redacted]` and never a byte of the value.
/// The buffer is overwritten with zeroes when the value is dropped. The type
/// is deliberately not `Clone`, so a copy is always an explicit act through
/// [`Secret::expose`].
pub struct Secret(Zeroizing<Vec<u8>>);

/// The text `Debug` and `Display` print in place of the value.
pub const REDACTED: &str = "[redacted]";

impl Secret {
    /// Takes ownership of `bytes`; the vector's buffer becomes the secret's.
    pub fn new(bytes: Vec<u8>) -> Self {
        Self(Zeroizing::new(bytes))
    }

    /// Copies `bytes` into a new secret buffer of exactly their length.
    pub fn from_slice(bytes: &[u8]) -> Self {
        Self(Zeroizing::new(bytes.to_vec()))
    }

    /// The secret bytes. Every call site is a place the value is used, and
    /// the only such place in the broker is the forwarding closure.
    pub fn expose(&self) -> &[u8] {
        &self.0
    }

    /// The number of secret bytes.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether the secret holds no bytes.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(REDACTED)
    }
}

impl fmt::Display for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(REDACTED)
    }
}

#[cfg(test)]
#[path = "secret_tests.rs"]
mod tests;
