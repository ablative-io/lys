//! Refusals of an OAuth grant's refresh. A refresh keeps the client the
//! grant was consented to; a grant for another client is a new consent,
//! which the person makes by reconnecting the grant by name.

/// Why an OAuth grant was not refreshed.
#[derive(Debug, thiserror::Error)]
pub enum OAuthRefusal {
    /// A refresh named a client other than the one the grant was consented
    /// to, and no reconnect named the new client.
    #[error(
        "ReconnectRequired: the grant {secret} was consented to client {consented} and the refresh names client {presented} (act: reconnect the grant by name for the new client)"
    )]
    ReconnectRequired {
        /// The secret the grant is sealed as.
        secret: String,
        /// The client the grant was consented to.
        consented: String,
        /// The client the refresh named.
        presented: String,
    },
}

impl OAuthRefusal {
    /// The refusal's name.
    pub fn name(&self) -> &'static str {
        match self {
            Self::ReconnectRequired { .. } => "ReconnectRequired",
        }
    }
}
