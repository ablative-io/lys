//! Refusals of a screen service's word for a person.

/// Why a screen service's request on a person's behalf was refused.
#[derive(Debug, thiserror::Error)]
pub enum ServiceRefusal {
    /// The service is not one the broker trusts.
    #[error(
        "ServiceUnknown: {service} is not a screen service this broker trusts (act: add its public key with trust-service)"
    )]
    Unknown {
        /// The service named.
        service: String,
    },
    /// The signature is not the service's over this request.
    #[error(
        "ServiceSignatureInvalid: the signature from {service} is not its signature over this request (act: sign the request as it is sent, with the service's own key)"
    )]
    SignatureInvalid {
        /// The service named.
        service: String,
    },
    /// The operation id was already used inside the freshness window.
    #[error(
        "ServiceReplayed: {service} already used this operation id (act: sign each request with a new operation id)"
    )]
    Replayed {
        /// The service named.
        service: String,
    },
    /// The person named is not an identity.
    #[error(
        "ServicePersonInvalid: {service} named {person:?}, which is not an identity (act: name the signed-in person as the directory names them)"
    )]
    PersonInvalid {
        /// The service named.
        service: String,
        /// What it named.
        person: String,
    },
}

impl ServiceRefusal {
    /// The refusal's name.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Unknown { .. } => "ServiceUnknown",
            Self::SignatureInvalid { .. } => "ServiceSignatureInvalid",
            Self::Replayed { .. } => "ServiceReplayed",
            Self::PersonInvalid { .. } => "ServicePersonInvalid",
        }
    }
}
