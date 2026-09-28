//! Refusals from signing: a key file that cannot be loaded, a signer that
//! declines, and a checkpoint the anchor could not sign over its own log.

use lys_core::error::TrustError;

/// Why the anchor could not load its key or produce a signature.
///
/// `#[non_exhaustive]` for the reason [`AnchorError`](super::AnchorError)
/// is: a new precondition in this family gets its own name without a major
/// version bump.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SigningError {
    /// The anchor's signing key could not be loaded from its file.
    ///
    /// The path is carried because `lys-core`'s own reason does not name it —
    /// `std::fs` errors do not include the path they were raised for — and an
    /// operator holding "failed to read identity key: No such file or
    /// directory" has been told everything except the one fact they need.
    ///
    /// **A missing file reaches here rather than being repaired.** Generating a
    /// key for a caller who asked to load one produces an anchor that publishes
    /// under an identity nobody was ever told about, and reports success while
    /// doing it.
    #[error("failed to load the anchor's signing key from {path}: {source}")]
    SignerKey {
        /// The key file path, as it was given.
        path: String,
        /// `lys-core`'s reason for refusing the key file.
        source: TrustError,
    },

    /// A [`Signer`](crate::keys::Signer) declined to produce a signature.
    ///
    /// **This variant exists because the trait promises a failure this enum
    /// could not express.** `Signer::sign` is documented as fallible "for the
    /// remote custody this trait is shaped for, where the network, the device or
    /// the operator's authorization can all decline" — but every other variant
    /// here describes the operator's *own* log or key file, so a remote signer
    /// had no honest name for its own refusal and had to borrow
    /// [`SignerKey`](Self::SignerKey), which reports a key-file problem for
    /// something that is not one.
    ///
    /// The gap stopped being hypothetical when genesis-as-delegation landed
    /// bounded on `Signer` rather than `InProcessSigner`: an offline or remote
    /// root signer is a real, reachable path now, and it is the one signing call
    /// in this crate that a network or a human can refuse.
    ///
    /// `#[non_exhaustive]` on this enum means a downstream implementor **cannot**
    /// add a variant themselves, so a trait whose contract promises a failure
    /// mode obliges the crate that owns the error type to name it. Free text
    /// rather than a structured cause because the reasons are the implementor's
    /// domain — an HSM's refusal, a declined touch, a timeout — and inventing a
    /// taxonomy for devices this crate has never seen would be guessing at
    /// somebody else's failure modes.
    #[error("the signer declined to sign: {reason}")]
    SignerDeclined {
        /// The signer's own account of why it refused.
        reason: String,
    },

    /// The anchor could not sign a checkpoint over its own log.
    ///
    /// Every precondition `lys-core` checks here is already satisfied by
    /// construction — the origin was validated when the store was created, and
    /// the body is machine-generated — so this variant reports something
    /// genuinely unexpected rather than a routine refusal. It is propagated
    /// with its cause instead of being treated as impossible, because a
    /// precondition that "cannot" fail is exactly the one nobody notices
    /// changing.
    #[error("failed to publish a checkpoint for {origin}: {source}")]
    Checkpoint {
        /// The origin the checkpoint was being published for.
        origin: String,
        /// `lys-core`'s reason for refusing to encode or sign it.
        source: TrustError,
    },
}
