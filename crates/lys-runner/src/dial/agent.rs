//! One explicitly configured agent request, signed outside the runner.

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use sha2::{Digest, Sha256};

use super::transport::{Channel, validate_request};
use crate::error::RunnerError;
use crate::protocol::{hex, nonce};

const HEX: &[u8; 16] = b"0123456789ABCDEF";

/// A holder's proof, bound by the broker to this signing payload.
pub struct Proof {
    /// The lease token, encoded in hex. It is never printed.
    pub token: String,
    /// Handle id, operation id, milliseconds and COSE presentation, in wire order.
    pub fields: [String; 4],
}

/// The caller supplies proof with a holder key distinct from the leased agent key.
pub trait LeaseProof {
    /// Bind the proof to `request_digest("SIGN", secret, payload)`.
    ///
    /// # Errors
    /// Returns the caller's named proof failure.
    fn present(&self, payload: &[u8]) -> Result<Proof, RunnerError>;
}

/// The agent and key lease supplied by the caller for this request.
pub struct AgentLease<'a> {
    /// The identity whose certificate verifies the leased key.
    pub agent: &'a str,
    /// The broker's sealed key entry.
    pub secret: &'a str,
    /// A proof made with the separate holder key.
    pub proof: &'a dyn LeaseProof,
}

/// A bounded response from the identity service.
#[derive(Debug)]
pub struct AgentResponse {
    /// The service's HTTP status.
    pub status: u16,
    /// The exact response body.
    pub body: Vec<u8>,
}

/// Persistent channels to the broker and identity service, holding no agent key.
pub struct AgentClient {
    server: Channel,
    broker: Channel,
}

fn refused(reason: impl Into<String>) -> RunnerError {
    RunnerError::refused("AgentSigningInvalid", reason)
}

fn validate_headers(headers: &[(&str, &str)]) -> Result<(), RunnerError> {
    for (name, _) in headers {
        if name.eq_ignore_ascii_case("cookie") {
            return Err(RunnerError::refused(
                "AgentCookieRefused",
                "a signed agent request cannot carry a cookie",
            ));
        }
        if [
            "lys-agent-signature",
            "host",
            "content-length",
            "transfer-encoding",
            "connection",
        ]
        .iter()
        .any(|reserved| name.eq_ignore_ascii_case(reserved))
        {
            return Err(refused(
                "a request header replaces transport or signature metadata",
            ));
        }
    }
    Ok(())
}

impl AgentClient {
    /// Configure verified TLS, or cleartext loopback, for both destinations.
    ///
    /// # Errors
    /// Returns the named dial configuration failure.
    pub fn new(server: &str, broker: &str, authority: Option<&Path>) -> Result<Self, RunnerError> {
        Ok(Self {
            server: Channel::for_server(server, authority)?,
            broker: Channel::for_server(broker, authority)?,
        })
    }

    /// Sign and send exactly this request, refusing mixed authority before signing.
    ///
    /// # Errors
    /// Returns input, holder-proof, broker or transport refusals by name.
    pub fn send(
        &self,
        lease: &AgentLease<'_>,
        method: &str,
        path: &str,
        headers: &[(&str, &str)],
        body: &[u8],
    ) -> Result<AgentResponse, RunnerError> {
        validate_headers(headers)?;
        validate_request(method, &self.server.path(path), headers)?;
        let signature = self.sign(lease.agent, lease.secret, lease.proof, method, path, body)?;
        self.send_signed(method, path, headers, body, &signature)
    }

    /// Ask the broker to sign the verifier's exact payload, including the path prefix and query.
    ///
    /// # Errors
    /// Returns input, holder-proof, broker or transport refusals by name.
    pub fn sign(
        &self,
        agent: &str,
        secret: &str,
        proof: &dyn LeaseProof,
        method: &str,
        path: &str,
        body: &[u8],
    ) -> Result<String, RunnerError> {
        if agent.is_empty()
            || !agent.bytes().all(|byte| byte.is_ascii_graphic())
            || secret.is_empty()
        {
            return Err(refused("agent or secret name is invalid"));
        }
        let path = self.server.path(path);
        validate_request(method, &path, &[])?;
        let time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| refused(error.to_string()))?
            .as_millis();
        let time = u64::try_from(time).map_err(|error| refused(error.to_string()))?;
        let nonce = nonce();
        let digest = hex(&Sha256::digest(body));
        let payload =
            format!("lys-identity/agent-request/v1\n{method}\n{path}\n{digest}\n{time}\n{nonce}");
        let Proof {
            token,
            fields: [id, operation, signed_at, presentation],
        } = proof.present(payload.as_bytes())?;
        let headers = [
            ("lys-handle", token.as_str()),
            ("lys-handle-id", id.as_str()),
            ("lys-operation", operation.as_str()),
            ("lys-signed-at", signed_at.as_str()),
            ("lys-presentation", presentation.as_str()),
        ];
        let mut route = String::from("/_lys/sign/");
        for byte in secret.bytes() {
            route.push('%');
            route.push(char::from(HEX[usize::from(byte >> 4)]));
            route.push(char::from(HEX[usize::from(byte & 15)]));
        }
        let response = self
            .broker
            .send_response("POST", &route, &headers, payload.as_bytes())?;
        if response.status != 200 {
            let value: serde_json::Value = serde_json::from_slice(&response.body)
                .map_err(|error| refused(format!("broker refusal did not read: {error}")))?;
            let name = value["refusal"]
                .as_str()
                .ok_or_else(|| refused("broker refusal has no name"))?;
            return Err(RunnerError::refused(
                name,
                value["reason"].as_str().unwrap_or("broker refused signing"),
            ));
        }
        lys_core::attestation::Attestation::from_cose_bytes(&response.body)
            .map_err(|error| refused(format!("broker signature did not read: {error}")))?;
        Ok(format!("{agent} {time} {nonce} {}", hex(&response.body)))
    }

    /// Send the signed bytes without adding ambient credentials or replaying an exchange.
    ///
    /// # Errors
    /// Returns mixed-authority, malformed-header or transport refusals.
    pub fn send_signed(
        &self,
        method: &str,
        path: &str,
        headers: &[(&str, &str)],
        body: &[u8],
        signature: &str,
    ) -> Result<AgentResponse, RunnerError> {
        validate_headers(headers)?;
        let mut headers = headers.to_vec();
        headers.push(("lys-agent-signature", signature));
        let response = self.server.send_response(method, path, &headers, body)?;
        Ok(AgentResponse {
            status: response.status,
            body: response.body,
        })
    }
}
