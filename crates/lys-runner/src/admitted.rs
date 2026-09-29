//! An agent's tool-boundary policy as a launch carries it: the policy
//! itself and the digest the server admitted it under, both inside the
//! server's signed start act.
//!
//! A policy has one canonical encoding: a domain line, then its JSON with
//! its members in their declared order and its rules in the order given.
//! Its digest is the lowercase hex SHA-256 of that encoding. The runner
//! never takes the digest on trust: it encodes the policy it was given
//! again and refuses the start when the two differ, so the policy the judge
//! holds is the one the server admitted, byte for byte.
//!
//! This digest covers the tool policy alone. The containment policy's
//! digest, under its own domain, binds more and is a different value.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::RunnerError;
use crate::judge::Policy;
use crate::protocol::hex;

/// The domain line a policy's canonical encoding begins with.
pub const ENCODING: &str = "lys-agent-policy/v1";

/// A policy and the digest it was admitted under.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Admitted {
    /// The policy the session's judge holds.
    pub policy: Policy,
    /// The digest of its canonical encoding, as the server computed it.
    pub digest: String,
}

/// The policy a session is judged under, as its status names it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JudgedUnder {
    /// The policy's version.
    pub version: u64,
    /// The digest it was admitted under.
    pub digest: String,
}

/// `policy`'s canonical encoding.
///
/// # Errors
///
/// `policy_invalid` when the policy cannot be encoded.
pub fn canonical(policy: &Policy) -> Result<Vec<u8>, RunnerError> {
    let mut bytes = ENCODING.as_bytes().to_vec();
    bytes.push(b'\n');
    serde_json::to_writer(&mut bytes, policy).map_err(|error| {
        RunnerError::refused(
            "policy_invalid",
            format!("the policy could not be encoded: {error}"),
        )
    })?;
    Ok(bytes)
}

/// `policy`'s digest: the lowercase hex SHA-256 of its canonical encoding.
///
/// # Errors
///
/// As [`canonical`].
pub fn digest(policy: &Policy) -> Result<String, RunnerError> {
    Ok(hex(&Sha256::digest(canonical(policy)?)))
}

impl Admitted {
    /// `policy` with the digest computed from it.
    ///
    /// # Errors
    ///
    /// As [`canonical`].
    pub fn of(policy: Policy) -> Result<Self, RunnerError> {
        let digest = digest(&policy)?;
        Ok(Self { policy, digest })
    }

    /// The version and digest a status names it by.
    #[must_use]
    pub fn judged_under(&self) -> JudgedUnder {
        JudgedUnder {
            version: self.policy.version,
            digest: self.digest.clone(),
        }
    }

    /// The policy, once its shape is checked and its digest recomputed and
    /// found equal to the one admitted.
    ///
    /// # Errors
    ///
    /// The policy's own refusal when its shape is wrong, and
    /// `policy_digest_mismatch` when its bytes are not the ones admitted.
    pub fn verified(self) -> Result<Policy, RunnerError> {
        let policy = self
            .policy
            .checked()
            .map_err(|refused| RunnerError::refused(refused.refusal, refused.words))?;
        let computed = digest(&policy)?;
        if computed != self.digest {
            return Err(RunnerError::refused(
                "policy_digest_mismatch",
                format!(
                    "policy {} of {} encodes to {computed}, not the admitted {}",
                    policy.version, policy.agent, self.digest
                ),
            ));
        }
        Ok(policy)
    }
}
