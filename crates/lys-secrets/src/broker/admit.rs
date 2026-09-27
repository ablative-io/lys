//! Admission: the checks a presented handle passes, in the one order that
//! gives one named refusal per attack.

use crate::encoding::{ct_eq, unhex};
use crate::error::SecretsError;
use crate::handle::{HandleToken, Presentation, check_operation_id};
use crate::permission::PermissionCheck;

use super::{Broker, HandleRecord, PRESENTATION_SKEW_MS};

pub(super) enum Admission {
    Retry(String),
    Fresh {
        id: String,
        identity: String,
        secret: String,
        uses_left: u64,
        used: u64,
    },
}

impl<P: PermissionCheck> Broker<P> {
    pub(super) fn find(&self, token: &HandleToken) -> Option<&HandleRecord> {
        let presented = token.digest();
        let mut found = None;
        for record in self.handles.values() {
            let held: Option<[u8; 32]> =
                unhex(&record.digest).and_then(|bytes| bytes.try_into().ok());
            if held.is_some_and(|held| ct_eq(&held, &presented)) {
                found = Some(record);
            }
        }
        found
    }

    pub(super) fn admit(
        &self,
        token: &HandleToken,
        presentation: &Presentation,
        operation: &str,
    ) -> Result<Admission, SecretsError> {
        let record = self.find(token).ok_or(SecretsError::HandleUnknown)?;
        let key: [u8; 32] = unhex(&record.holder_key)
            .and_then(|bytes| bytes.try_into().ok())
            .ok_or_else(|| SecretsError::StoreCorrupt {
                reason: format!("handle {} holds no holder key", record.id),
            })?;
        if presentation.handle_id.as_str() != record.id {
            return Err(SecretsError::PresentationInvalid {
                handle: record.id.clone(),
            });
        }
        presentation.verify(&key)?;
        check_operation_id(&presentation.operation_id)?;
        if let Some(outcome) = record.operations.get(operation) {
            return Ok(Admission::Retry(outcome.clone()));
        }
        let now = (self.clock)();
        let skew = now.saturating_sub(presentation.signed_at_ms);
        if skew.abs() > PRESENTATION_SKEW_MS {
            return Err(SecretsError::PresentationStale {
                skew_ms: skew,
                limit_ms: PRESENTATION_SKEW_MS,
            });
        }
        if record.dropped {
            return Err(SecretsError::HandleDropped {
                handle: record.id.clone(),
            });
        }
        if now > record.not_after_ms {
            return Err(SecretsError::LeaseWindowClosed {
                handle: record.id.clone(),
            });
        }
        if record.used >= record.max_uses {
            return Err(SecretsError::LeaseExhausted {
                handle: record.id.clone(),
            });
        }
        if let Err(denied) = self.permissions.may_use(&record.identity, &record.secret) {
            return Err(SecretsError::PermissionDenied {
                holder: record.identity.clone(),
                secret: record.secret.clone(),
                reason: denied.reason,
            });
        }
        let used = record.used.saturating_add(1);
        Ok(Admission::Fresh {
            id: record.id.clone(),
            identity: record.identity.clone(),
            secret: record.secret.clone(),
            uses_left: record.max_uses.saturating_sub(used),
            used,
        })
    }
}
