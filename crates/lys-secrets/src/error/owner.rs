//! Refusals of an owner change's operation id.

/// Why an owner change of scope or recipients was refused for its
/// operation id.
#[derive(Debug, thiserror::Error)]
pub enum OwnerChangeRefusal {
    /// The change carried no operation id, or one of the wrong shape.
    #[error(
        "OperationMissing: the change of {secret} {reason} (act: send an operation id of 16 to 64 characters from A-Z, a-z, 0-9, _ and -, made once for this change)"
    )]
    Missing {
        /// The secret the change was for.
        secret: String,
        /// What was wrong with the operation id.
        reason: String,
    },
    /// The operation id was already applied to the secret with another body.
    #[error(
        "OperationReused: operation {operation} was already applied to {secret} with another change (act: send a new change under a new operation id)"
    )]
    Reused {
        /// The operation id.
        operation: String,
        /// The secret it was applied to.
        secret: String,
    },
}

impl OwnerChangeRefusal {
    /// The refusal's name.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Missing { .. } => "OperationMissing",
            Self::Reused { .. } => "OperationReused",
        }
    }
}
