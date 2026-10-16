//! Retained evidence must reproduce the authenticated request exactly.
use super::{RequestSignature,RequestEvidence,IdentityError,invalid};

impl RequestSignature {
    pub(super) fn check(&self, proof: &RequestEvidence) -> Result<(), IdentityError> {
        if !self.header.is_ascii()
            || self
                .header
                .iter()
                .any(|byte| byte.is_ascii_control() && *byte != b'\t')
        {
            return Err(invalid("the original signature header must be ASCII"));
        }
        let header = std::str::from_utf8(&self.header)
            .ok()
            .ok_or_else(|| invalid("the signature header is not UTF-8"))?;
        let words: Vec<_> = header.split_ascii_whitespace().collect();
        let [agent, at, nonce, cose] = words.as_slice() else {
            return Err(invalid("the signature header must contain four fields"));
        };
        let timestamp = at.parse::<u64>().ok();
        let cose_bytes = cose
            .as_bytes()
            .chunks_exact(2)
            .map(|pair| {
                std::str::from_utf8(pair)
                    .ok()
                    .and_then(|text| u8::from_str_radix(text, 16).ok())
            })
            .collect::<Option<Vec<_>>>();
        if *agent != proof.agent.to_string()
            || timestamp != Some(proof.signed_at_ms)
            || *nonce != proof.nonce
            || cose.len() % 2 != 0
            || cose_bytes.as_deref() != Some(proof.cose_sign1.as_slice())
        {
            return Err(invalid(
                "the retained signature header disagrees with the verified evidence",
            ));
        }
        let expected = format!(
            "lys-identity/agent-request/v1\n{}\n{}\n{}\n{}\n{}",
            proof.method,
            proof.path,
            crate::id::to_hex(&crate::encoding::payload_commitment(&proof.body)),
            proof.signed_at_ms,
            proof.nonce
        );
        if self.payload != expected.as_bytes() {
            return Err(invalid(
                "the retained payload is not the exact signed request payload",
            ));
        }
        if lys_core::attestation::verify_attestation_bytes(&proof.cose_sign1, &self.payload)
            .is_err()
        {
            return Err(invalid("the retained request signature is invalid"));
        }
        Ok(())
    }
}
