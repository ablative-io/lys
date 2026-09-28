//! The operation id a caller gives each change.
//!
//! A caller names every change with an operation id and keeps it across
//! retries. The id is carried in the signed event, so a retry of a change that
//! was recorded finds the event it made instead of making a second one.

use std::fmt;
use std::str::FromStr;

use crate::error::IdentityError;
use crate::id::{ID_LEN, from_hex, random_bytes, to_hex};

/// A change's operation id. Its text form is `op-` and 32 hex digits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct OperationId([u8; ID_LEN]);

impl OperationId {
    const PREFIX: &'static str = "op-";

    /// A new operation id from the secure random source.
    pub fn generate() -> Result<Self, IdentityError> {
        random_bytes().map(Self)
    }

    /// The operation id these bytes are, as read back from a signed event.
    pub fn from_bytes(bytes: [u8; ID_LEN]) -> Self {
        Self(bytes)
    }

    /// The operation id's bytes, as a signed event carries them.
    pub fn as_bytes(&self) -> &[u8; ID_LEN] {
        &self.0
    }
}

impl fmt::Display for OperationId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", Self::PREFIX, to_hex(&self.0))
    }
}

impl FromStr for OperationId {
    type Err = IdentityError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        text.strip_prefix(Self::PREFIX)
            .and_then(from_hex)
            .map(Self)
            .ok_or_else(|| IdentityError::IdentifierMalformed {
                kind: "operation",
                text: text.to_owned(),
            })
    }
}
