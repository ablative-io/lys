//! Refusals of a signing key: a seed that is not one, a purpose outside the
//! closed list, a signature asked for a purpose the key does not sign for,
//! and a value asked of a key that gives none out. No message carries a
//! byte of the seed; the one that names a seed names its length only.

/// Why a signing key was not sealed, or not used as asked.
#[derive(Debug, thiserror::Error)]
pub enum SigningRefusal {
    /// A signing key was sealed with something other than a 32-byte
    /// Ed25519 seed.
    #[error(
        "signing_key_invalid: the signing key {secret} {reason} (act: seal the key's 32-byte Ed25519 seed with seal-signing-key)"
    )]
    KeyInvalid {
        /// The signing key.
        secret: String,
        /// What is wrong with it, naming a seed's length and never a byte.
        reason: String,
    },
    /// A purpose outside the closed list of signing purposes was named.
    #[error(
        "signing_purpose_unknown: {purpose} is not a signing purpose (act: name agent_request)"
    )]
    PurposeUnknown {
        /// The purpose named.
        purpose: String,
    },
    /// A signature was asked for a purpose other than the one the key
    /// signs for.
    #[error(
        "signing_purpose_mismatch: the key {secret} signs for {held} and the call asked for {asked} (act: ask the key for its own purpose, or use a key sealed for {asked})"
    )]
    PurposeMismatch {
        /// The key asked.
        secret: String,
        /// The one purpose it signs for; `none` for a secret that is not a
        /// signing key.
        held: &'static str,
        /// The purpose the call asked for.
        asked: &'static str,
    },
    /// A signing key was asked for its value, which it never gives out.
    #[error(
        "not_a_value_secret: {secret} is a signing key and gives no value out (act: ask the broker for a signature at POST /_lys/signature)"
    )]
    NotAValueSecret {
        /// The signing key.
        secret: String,
    },
}

impl SigningRefusal {
    /// The refusal's name.
    pub fn name(&self) -> &'static str {
        match self {
            Self::KeyInvalid { .. } => "signing_key_invalid",
            Self::PurposeUnknown { .. } => "signing_purpose_unknown",
            Self::PurposeMismatch { .. } => "signing_purpose_mismatch",
            Self::NotAValueSecret { .. } => "not_a_value_secret",
        }
    }
}
