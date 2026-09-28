//! Refusals by the rules of lending: who may lend, how far a derived
//! handle may reach, and who may receive one.

/// Why a handle was not lent or handed over.
#[derive(Debug, thiserror::Error)]
pub enum LendingRefusal {
    /// A derived handle asked for by a holder who neither owns the secret
    /// nor holds the right to lend it.
    #[error(
        "LendingNotPermitted: {holder} neither owns {secret} nor holds the right to lend it (act: ask the secret's owner for the lend relation)"
    )]
    NotPermitted {
        /// The holder asking to lend.
        holder: String,
        /// The secret's name.
        secret: String,
    },
    /// A handle asked to be ended by someone other than the person its
    /// holder acts for.
    #[error(
        "LendingNotPermitted: {person} is not the person the holder of handle {handle} acts for (act: ask the person the holder acts for to end it)"
    )]
    NotActedFor {
        /// The one asking to end it.
        person: String,
        /// The handle.
        handle: String,
    },
    /// A derived handle asked for with a bound past the handle above it.
    #[error(
        "BeyondAncestry: a handle derived from {handle} must stay within its uses, window and spend (act: ask for bounds within the handle it is derived from)"
    )]
    BeyondAncestry {
        /// The handle derived from.
        handle: String,
    },
    /// A derived handle asked for below the deepest line the broker counts.
    #[error(
        "LendingTooDeep: {handle} already stands {depth} handles deep, the deepest a line is counted (act: lend from a handle nearer the one first issued)"
    )]
    TooDeep {
        /// The handle derived from.
        handle: String,
        /// Its depth.
        depth: usize,
    },
    /// A handle asked for on a secret whose recipient policy excludes the
    /// recipient.
    #[error(
        "RecipientRefused: {secret} may be handed to people only and {recipient} is not a person (act: hand it to a person, or ask its owner to change the recipient policy)"
    )]
    RecipientRefused {
        /// The recipient asked for.
        recipient: String,
        /// The secret's name.
        secret: String,
    },
}

impl LendingRefusal {
    /// The refusal's name.
    pub fn name(&self) -> &'static str {
        match self {
            Self::NotPermitted { .. } | Self::NotActedFor { .. } => "LendingNotPermitted",
            Self::BeyondAncestry { .. } => "BeyondAncestry",
            Self::TooDeep { .. } => "LendingTooDeep",
            Self::RecipientRefused { .. } => "RecipientRefused",
        }
    }
}
