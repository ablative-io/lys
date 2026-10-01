//! Signed request bodies may carry run credentials and are never formatted.

use serde::{Deserialize, Serialize};

/// A request as it is sent.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    /// The protocol version.
    pub version: u32,
    /// The runner it is made for, as its greeting named it.
    pub runner: String,
    /// The challenge of the connection it is made on.
    pub challenge: String,
    /// The act's JSON, as signed.
    pub act: String,
    /// Lowercase hex of the signature.
    #[serde(default)]
    pub signature: String,
}

impl std::fmt::Debug for Request {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Request")
            .field("version", &self.version)
            .field("runner", &self.runner)
            .field("challenge", &self.challenge)
            .finish_non_exhaustive()
    }
}
