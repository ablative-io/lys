//! Refusals of a secret's accounts: none in service to open, or none to
//! move to. Both carry the one name `AccountsRested`, and the act that
//! answers either is to bring an account back.

/// Why no account of a secret could serve.
#[derive(Debug, thiserror::Error)]
pub enum AccountsRefusal {
    /// Every account of the secret is resting, so a use, a spawn or an ask
    /// for the next account finds none in service.
    #[error("AccountsRested: every account of {secret} is resting (act: bring an account back)")]
    AllResting {
        /// The secret.
        secret: String,
    },
    /// Every account but the one in use is resting, so an ask for the next
    /// account has none to move to; the one in use stays in service.
    #[error(
        "AccountsRested: every other account of {secret} is resting, so there is none to move to (act: bring an account back)"
    )]
    NoneToMoveTo {
        /// The secret.
        secret: String,
    },
}

impl AccountsRefusal {
    /// The refusal's name.
    pub fn name(&self) -> &'static str {
        match self {
            Self::AllResting { .. } | Self::NoneToMoveTo { .. } => "AccountsRested",
        }
    }
}
