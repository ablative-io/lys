//! Certificate decoding is shared by requests and runs after the store lock is released.

use std::sync::{Arc, OnceLock};

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use lys_core::ca::{CertificateSigningKey, certificate_signing_key};

use crate::error::ServerError;

#[derive(Debug)]
pub(crate) struct SigningCertificate {
    der: Arc<str>,
    decoded: OnceLock<Result<CertificateSigningKey, String>>,
}

impl SigningCertificate {
    pub(crate) fn new(der: &str) -> Self {
        Self {
            der: Arc::from(der),
            decoded: OnceLock::new(),
        }
    }

    pub(crate) fn key_at(&self, at: u64) -> Result<Option<[u8; 32]>, ServerError> {
        let key = self
            .decoded
            .get_or_init(|| {
                let bytes = STANDARD
                    .decode(self.der.as_bytes())
                    .map_err(|error| error.to_string())?;
                certificate_signing_key(&bytes).map_err(|error| error.to_string())
            })
            .as_ref()
            .map_err(|reason| ServerError::CertificatesUnavailable {
                reason: format!("a recorded certificate cannot be decoded: {reason}"),
            })?;
        let at = i128::from(at);
        Ok(
            (at >= i128::from(key.not_before) && at <= i128::from(key.not_after))
                .then_some(key.public_key),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn certificate_validity_bounds_are_checked_on_every_use() -> Result<(), ServerError> {
        let certificate = SigningCertificate::new(
            &STANDARD.encode(include_bytes!("../tests/fixtures/expired-agent.der")),
        );
        assert!(certificate.key_at(946_684_799)?.is_none());
        assert!(certificate.key_at(946_684_800)?.is_some());
        assert!(certificate.key_at(946_771_200)?.is_some());
        assert!(certificate.key_at(946_771_201)?.is_none());
        Ok(())
    }

    #[test]
    fn malformed_certificate_is_refused_on_every_use() {
        let certificate = SigningCertificate::new("AAAA");
        for at in [0, 1] {
            let error = certificate.key_at(at).err().unwrap();
            assert_eq!(error.name(), "CertificatesUnavailable");
        }
    }
}
