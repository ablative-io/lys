//! Refusals of an app's virtual client credential.

/// Why an app's virtual client credential was not issued or not confirmed.
#[derive(Debug, thiserror::Error)]
pub enum AppClientRefusal {
    /// No sealed client secret is held for the app.
    #[error(
        "AppClientNoCustody: no client secret is sealed for the app {app} (act: approve the app, which seals its client secret, before issuing it a credential)"
    )]
    NoCustody {
        /// The app.
        app: String,
    },
    /// The presented value is not a live credential issued for the app.
    #[error(
        "AppClientCredentialRefused: the credential presented is not a live credential issued for the app {app} (act: present a credential an administrator issued for this app and has not revoked)"
    )]
    Refused {
        /// The app.
        app: String,
    },
    /// The sealed client secret is not the one the app's approval records.
    #[error(
        "AppClientCustodyMismatch: the client secret sealed for the app {app} is not the one its approval records (act: read the broker's audit for the app's client entry; nothing is confirmed until they agree)"
    )]
    CustodyMismatch {
        /// The app.
        app: String,
    },
}

impl AppClientRefusal {
    /// The refusal's name.
    pub fn name(&self) -> &'static str {
        match self {
            Self::NoCustody { .. } => "AppClientNoCustody",
            Self::Refused { .. } => "AppClientCredentialRefused",
            Self::CustodyMismatch { .. } => "AppClientCustodyMismatch",
        }
    }
}
