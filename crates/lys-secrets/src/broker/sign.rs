//! Local signing keeps the opened key inside one admitted use.

use lys_core::Ed25519Identity;
use lys_core::attestation::sign_attestation;
use zeroize::Zeroizing;

use crate::audit::AuditKind;
use crate::encoding::hex;
use crate::handle::{HandleToken, Presentation, request_digest};
use crate::{EntryClass, PermissionCheck, SecretsError};

use super::{Admitted, Broker, Checked, Settled, Used};

/// The COSE bytes of a signature, carrying no private key material.
///
/// ```compile_fail
/// fn key_is_not_accessible(signature: lys_secrets::Signature) {
///     signature.credential();
/// }
/// ```
#[derive(Debug, PartialEq, Eq)]
pub struct Signature(Vec<u8>);

impl Signature {
    /// The `COSE_Sign1` bytes to carry to the verifier.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

/// An entry that cannot be used as a signing key.
#[derive(Debug, thiserror::Error)]
pub enum SigningRefusal {
    /// The leased entry is not a key.
    #[error("SigningKeyRequired: {secret} is not a key (act: lease a sealed key entry)")]
    KeyRequired {
        /// The leased secret.
        secret: String,
    },
    /// The key does not hold one raw Ed25519 seed.
    #[error(
        "SigningSeedInvalid: {secret} holds {length} bytes, not 32 (act: seal a raw 32-byte Ed25519 seed)"
    )]
    SeedInvalid {
        /// The leased secret.
        secret: String,
        /// The supplied seed's length.
        length: usize,
    },
}

impl SigningRefusal {
    /// The name recorded in the use's settlement.
    pub fn name(&self) -> &'static str {
        match self {
            Self::KeyRequired { .. } => "SigningKeyRequired",
            Self::SeedInvalid { .. } => "SigningSeedInvalid",
        }
    }
}

impl<P: PermissionCheck> Broker<P> {
    /// Signs `payload` with the sealed key the lease names, returning only
    /// its `COSE_Sign1` signature after the use has settled.
    ///
    /// The presentation binds `request_digest("SIGN", secret, payload)`.
    /// Local signing spends zero; a capped lease still reserves before use.
    /// The caller obtains `checked` from the use's current permission questions.
    ///
    /// # Errors
    /// The admission, forward-boundary and settlement refusals, plus
    /// `SigningKeyRequired` and `SigningSeedInvalid` for an entry that cannot sign.
    pub fn sign_use_for_checked(
        &mut self,
        token: &HandleToken,
        presentation: &Presentation,
        secret: &str,
        payload: &[u8],
        reserve: u64,
        checked: &Checked,
    ) -> Result<Used<Signature>, SecretsError> {
        let request = request_digest("SIGN", secret, payload)?;
        if presentation.request != request {
            return Err(self.refuse_sign_payload(token, presentation, &request)?);
        }
        let admitted = self.admit_use_for_checked(token, presentation, secret, reserve, checked)?;
        let ticket = match admitted {
            Admitted::Retried { outcome } => return Ok(Used::Retried { outcome }),
            Admitted::Fresh(ticket) => self.at_forward_boundary_checked(ticket, checked)?,
        };
        let signed = (|| {
            if ticket.class != EntryClass::Key {
                return Err(SigningRefusal::KeyRequired {
                    secret: ticket.secret.clone(),
                });
            }
            let bytes = ticket.credential().expose();
            if bytes.len() != 32 {
                return Err(SigningRefusal::SeedInvalid {
                    secret: ticket.secret.clone(),
                    length: bytes.len(),
                });
            }
            let mut seed = Zeroizing::new([0_u8; 32]);
            seed.copy_from_slice(bytes);
            let key = Ed25519Identity::from_seed(&seed);
            Ok(Signature(sign_attestation(payload, &key).to_cose_bytes()))
        })();
        match signed {
            Ok(answer) => {
                let uses_left = ticket.uses_left();
                self.settle_checked(ticket, Settled::Spent(0), checked)?;
                Ok(Used::Forwarded { answer, uses_left })
            }
            Err(refusal) => {
                self.close(
                    &ticket.handle,
                    (&ticket.identity, &ticket.secret),
                    (&ticket.operation, &ticket.mark),
                    ticket.reserved().map(|_| 0),
                    refusal.name(),
                )?;
                Err(refusal.into())
            }
        }
    }

    fn refuse_sign_payload(
        &mut self,
        token: &HandleToken,
        presentation: &Presentation,
        request: &[u8; 32],
    ) -> Result<SecretsError, SecretsError> {
        let found = self.find(token).map(|record| {
            (
                record.id.clone(),
                record.identity.clone(),
                record.secret.clone(),
            )
        });
        let refusal = found
            .as_ref()
            .map_or(SecretsError::HandleUnknown, |(id, _, _)| {
                SecretsError::PresentationInvalid { handle: id.clone() }
            });
        let subject = found
            .as_ref()
            .map_or((None, None, None), |(id, identity, secret)| {
                (
                    Some(id.as_str()),
                    Some(identity.as_str()),
                    Some(secret.as_str()),
                )
            });
        let operation = hex(&presentation.operation_id);
        let mark = self.request_mark(request)?;
        self.record(
            AuditKind::Use,
            subject,
            Some((&operation, &mark)),
            None,
            refusal.name(),
        )?;
        Ok(refusal)
    }
}
