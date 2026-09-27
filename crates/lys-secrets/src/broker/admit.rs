//! Admission: the checks a presented handle passes, in the one order that
//! gives one named refusal per attack.

use crate::encoding::{Canonical, ct_eq, hex, sha256, unhex};
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
        /// The reservation, on a capped lease.
        reserved: Option<u64>,
    },
}

const MARK_DOMAIN: &str = "lys-secrets/request-mark/v1";

impl<P: PermissionCheck> Broker<P> {
    /// A keyed mark of a request digest: the store key's deterministic
    /// signature over it, hashed. Only this broker can compute it, so the
    /// log can carry it without telling what the request was.
    pub(super) fn request_mark(&self, request: &[u8; 32]) -> Result<String, SecretsError> {
        let mut encoding = Canonical::new(MARK_DOMAIN)?;
        encoding.field(request)?;
        let signature = self.store_key.identity().sign(&encoding.into_bytes());
        Ok(hex(&sha256(&signature)))
    }

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

    /// The handle `token` names, when `presentation` is signed for it by the
    /// key it is bound to.
    pub(super) fn presented(
        &self,
        token: &HandleToken,
        presentation: &Presentation,
    ) -> Result<&HandleRecord, SecretsError> {
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
        Ok(record)
    }

    /// Whether the presentation is fresh, and the handle not dropped and
    /// inside its window.
    pub(super) fn live(
        &self,
        record: &HandleRecord,
        presentation: &Presentation,
    ) -> Result<(), SecretsError> {
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
        Ok(())
    }

    /// Whether the handle's identity still holds the use relation.
    pub(super) fn permitted(&self, record: &HandleRecord) -> Result<(), SecretsError> {
        self.permissions
            .may_use(&record.identity, &record.secret)
            .map(|_permit| ())
            .map_err(|denied| SecretsError::PermissionDenied {
                holder: record.identity.clone(),
                secret: record.secret.clone(),
                reason: denied.reason,
            })
    }

    pub(super) fn admit(
        &self,
        token: &HandleToken,
        presentation: &Presentation,
        (operation, mark): (&str, &str),
        reserve: u64,
    ) -> Result<Admission, SecretsError> {
        let record = self.presented(token, presentation)?;
        check_operation_id(&presentation.operation_id)?;
        if let Some((outcome, held)) = record.operations.get(operation) {
            if held != mark {
                return Err(SecretsError::OperationIdReused {
                    operation: operation.to_owned(),
                });
            }
            return Ok(Admission::Retry(outcome.clone()));
        }
        self.live(record, presentation)?;
        if record.used >= record.max_uses {
            return Err(SecretsError::LeaseExhausted {
                handle: record.id.clone(),
            });
        }
        self.permitted(record)?;
        let reserved = match record.spend_cap {
            None => None,
            Some(_cap) if reserve == 0 => {
                return Err(SecretsError::ReservationMissing {
                    handle: record.id.clone(),
                });
            }
            Some(cap) => {
                let held = record
                    .open
                    .values()
                    .fold(record.settled, |sum, open| sum.saturating_add(*open));
                let left = cap.saturating_sub(held);
                if reserve > left {
                    return Err(SecretsError::SpendCapReached {
                        handle: record.id.clone(),
                        cap,
                        left,
                        asked: reserve,
                    });
                }
                Some(reserve)
            }
        };
        let used = record.used.saturating_add(1);
        Ok(Admission::Fresh {
            id: record.id.clone(),
            identity: record.identity.clone(),
            secret: record.secret.clone(),
            uses_left: record.max_uses.saturating_sub(used),
            used,
            reserved,
        })
    }
}
