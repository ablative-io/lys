//! Refusals of a lease's bounds: a window past the grant it counts against,
//! and a handle presented for a secret outside the one it was cut for.

/// Why a lease was not cut, or a handle not admitted, for its bounds.
#[derive(Debug, thiserror::Error)]
pub enum LeaseRefusal {
    /// A lease asked for with a `not_after` later than the window of the
    /// grant it counts against.
    #[error(
        "LeaseBeyondGrant: the lease on {secret} was asked to end at {not_after_ms} ms, past the grant's window ending at {grant_ends_ms} ms (act: set the lease's not_after within the grant's window)"
    )]
    BeyondGrant {
        /// The secret the lease is on.
        secret: String,
        /// The end asked for, in milliseconds since the epoch.
        not_after_ms: i64,
        /// The end of the grant's window, in milliseconds since the epoch.
        grant_ends_ms: i64,
    },
    /// A handle presented for a secret other than the one its lease names.
    #[error(
        "OutsideScope: handle {handle} is on {secret} and was presented for {asked} (act: ask the grant's owner for a grant that covers this scope)"
    )]
    OutsideScope {
        /// The handle id.
        handle: String,
        /// The secret the lease names.
        secret: String,
        /// The secret asked for.
        asked: String,
    },
}

impl LeaseRefusal {
    /// The refusal's name.
    pub fn name(&self) -> &'static str {
        match self {
            Self::BeyondGrant { .. } => "LeaseBeyondGrant",
            Self::OutsideScope { .. } => "OutsideScope",
        }
    }
}
