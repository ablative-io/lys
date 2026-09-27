//! A screen service's word for a person. A service the broker trusts, such
//! as the identity service a person signed in to, asks on that person's
//! behalf: it signs the person's identity, an operation id, the time and
//! the digest of the one request it is for, with its own key. The broker
//! answers as that person, exactly as it answers a handle's identity. The
//! service's key never leaves the service, and no handle is involved.

use std::collections::BTreeMap;

use lys_core::Ed25519Identity;
use lys_core::attestation::{Attestation, sign_attestation, verify_attestation_by_signer};
use serde::{Deserialize, Serialize};

use crate::broker::PRESENTATION_SKEW_MS;
use crate::encoding::{Canonical, hex, unhex};
use crate::error::{SecretsError, ServiceRefusal};
use crate::handle::check_operation_id;
use crate::store::Recipients;

/// The domain every service payload opens with.
pub const SERVICE_DOMAIN: &str = "lys-secrets/on-behalf/v1";

/// A screen service the broker trusts: its name and its public key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceKey {
    /// The name it signs as.
    pub name: String,
    /// Its Ed25519 public key, in hex.
    pub public_key: String,
}

/// A service's signed request on a person's behalf.
#[derive(Debug, Clone)]
pub struct OnBehalf {
    /// The service asking.
    pub service: String,
    /// The person it asks for.
    pub person: String,
    /// The call's operation id.
    pub operation_id: Vec<u8>,
    /// When it was signed, in milliseconds since the epoch.
    pub signed_at_ms: i64,
    /// The digest of the request it is for, computed from the request.
    pub request: [u8; 32],
    /// The service's signature over the payload.
    pub attestation: Attestation,
}

impl OnBehalf {
    /// Signs a request for `person` as `service` with `key`.
    ///
    /// # Errors
    ///
    /// `OperationIdTooShort` and `Encoding`.
    pub fn sign(
        service: &str,
        person: &str,
        operation_id: &[u8],
        signed_at_ms: i64,
        request: [u8; 32],
        key: &Ed25519Identity,
    ) -> Result<Self, SecretsError> {
        check_operation_id(operation_id)?;
        let payload = payload(service, person, operation_id, signed_at_ms, &request)?;
        Ok(Self {
            service: service.to_owned(),
            person: person.to_owned(),
            operation_id: operation_id.to_vec(),
            signed_at_ms,
            request,
            attestation: sign_attestation(&payload, key),
        })
    }

    /// The request as the five text values it travels in: service, person,
    /// operation id in hex, signing time, and the signature in hex.
    pub fn to_wire(&self) -> [String; 5] {
        [
            self.service.clone(),
            self.person.clone(),
            hex(&self.operation_id),
            self.signed_at_ms.to_string(),
            hex(&self.attestation.to_cose_bytes()),
        ]
    }

    /// Reads the request from its five text values and the digest of the
    /// request that carried them.
    ///
    /// # Errors
    ///
    /// `ServiceSignatureInvalid` when a value does not read.
    pub fn from_wire(wire: [&str; 5], request: [u8; 32]) -> Result<Self, SecretsError> {
        let [service, person, operation, signed_at, signature] = wire;
        let invalid = || ServiceRefusal::SignatureInvalid {
            service: service.to_owned(),
        };
        let operation_id = unhex(operation).ok_or_else(invalid)?;
        let signed_at_ms = signed_at.parse::<i64>().map_err(|_number| invalid())?;
        let cose = unhex(signature).ok_or_else(invalid)?;
        let attestation = Attestation::from_cose_bytes(&cose).map_err(|_cose| invalid())?;
        Ok(Self {
            service: service.to_owned(),
            person: person.to_owned(),
            operation_id,
            signed_at_ms,
            request,
            attestation,
        })
    }
}

/// The operation ids each service used inside the freshness window, so a
/// signed request is admitted once.
#[derive(Debug, Default)]
pub struct ServiceWindow {
    seen: BTreeMap<(String, Vec<u8>), i64>,
}

impl ServiceWindow {
    /// An empty window.
    pub fn new() -> Self {
        Self::default()
    }

    /// The person `asked` speaks for, when a trusted service signed this
    /// request freshly and for the first time.
    ///
    /// # Errors
    ///
    /// `ServiceUnknown`, `ServiceSignatureInvalid`, `OperationIdTooShort`,
    /// `PresentationStale`, `ServicePersonInvalid` and `ServiceReplayed`.
    pub fn admit(
        &mut self,
        trusted: &[ServiceKey],
        asked: &OnBehalf,
        now_ms: i64,
    ) -> Result<String, SecretsError> {
        let service = asked.service.clone();
        let entry = trusted
            .iter()
            .find(|entry| entry.name == service)
            .ok_or_else(|| ServiceRefusal::Unknown {
                service: service.clone(),
            })?;
        let invalid = || ServiceRefusal::SignatureInvalid {
            service: service.clone(),
        };
        let key: [u8; 32] = unhex(&entry.public_key)
            .and_then(|bytes| bytes.try_into().ok())
            .ok_or_else(invalid)?;
        check_operation_id(&asked.operation_id)?;
        let payload = payload(
            &asked.service,
            &asked.person,
            &asked.operation_id,
            asked.signed_at_ms,
            &asked.request,
        )?;
        verify_attestation_by_signer(&asked.attestation, &payload, &key)
            .map_err(|_invalid| invalid())?;
        let skew = now_ms.saturating_sub(asked.signed_at_ms);
        if skew.abs() > PRESENTATION_SKEW_MS {
            return Err(SecretsError::PresentationStale {
                skew_ms: skew,
                limit_ms: PRESENTATION_SKEW_MS,
            });
        }
        if asked.person.trim() != asked.person || !Recipients::PeopleOnly.admits(&asked.person) {
            return Err(ServiceRefusal::PersonInvalid {
                service,
                person: asked.person.clone(),
            }
            .into());
        }
        self.seen.retain(|_key, signed_at| {
            now_ms.saturating_sub(*signed_at) <= 2 * PRESENTATION_SKEW_MS
        });
        let key = (service.clone(), asked.operation_id.clone());
        if self.seen.contains_key(&key) {
            return Err(ServiceRefusal::Replayed { service }.into());
        }
        self.seen.insert(key, asked.signed_at_ms);
        Ok(asked.person.clone())
    }
}

fn payload(
    service: &str,
    person: &str,
    operation_id: &[u8],
    signed_at_ms: i64,
    request: &[u8; 32],
) -> Result<Vec<u8>, SecretsError> {
    let mut encoding = Canonical::new(SERVICE_DOMAIN)?;
    encoding
        .field(service.as_bytes())?
        .field(person.as_bytes())?
        .field(operation_id)?
        .field(&signed_at_ms.to_be_bytes())?
        .field(request)?;
    Ok(encoding.into_bytes())
}
