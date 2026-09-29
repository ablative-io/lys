//! Pure action-nonce bindings. No random value is generated or sent by this code.

use sha2::{Digest, Sha256};

use super::{Choice, Owner, Refusal, Request, exact, same};

pub(super) fn digest(secret: &str) -> [u8; 32] {
    Sha256::digest(secret.as_bytes()).into()
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Target {
    Decision {
        request: Box<Request>,
        choice: Choice,
    },
    Revocation {
        owner: Owner,
        consent: String,
    },
}

/// Server-held action binding containing a digest only. A future route must
/// generate a fresh random secret, enforce origin/session ownership, return it
/// only to that owner, and atomically consume this record on first append.
/// Cloning this pure value does not make it a durable one-use nonce store.
#[derive(Debug, Clone)]
pub struct ActionNonce {
    target: Target,
    proof: [u8; 32],
    expires_at: u64,
}

impl ActionNonce {
    /// Bind a server-provided secret to the exact displayed request and choice.
    ///
    /// # Errors
    /// Refuses malformed request, absent secret or a nonce ending after its request.
    pub fn for_decision(
        request: Request,
        choice: Choice,
        secret: &str,
        expires_at: u64,
    ) -> Result<Self, Refusal> {
        request.validate()?;
        if secret.is_empty() || expires_at <= request.issued_at || expires_at > request.expires_at {
            return Err(Refusal::NonceRefused);
        }
        Ok(Self {
            target: Target::Decision {
                request: Box::new(request),
                choice,
            },
            proof: digest(secret),
            expires_at,
        })
    }

    /// Bind a separate revocation secret to current owner/session, consent and
    /// revoke action. The acquisition route must first authenticate that owner.
    ///
    /// # Errors
    /// Refuses absent identifiers or proof. Current owner and expiry are checked
    /// again by the revocation planner using trusted live facts.
    pub fn for_revocation(
        owner: Owner,
        consent: String,
        secret: &str,
        expires_at: u64,
    ) -> Result<Self, Refusal> {
        if !owner.valid() || !exact(&consent) || secret.is_empty() || expires_at == 0 {
            return Err(Refusal::NonceRefused);
        }
        Ok(Self {
            target: Target::Revocation { owner, consent },
            proof: digest(secret),
            expires_at,
        })
    }

    fn check(&self, target: &Target, proof: &[u8; 32], at: u64) -> Result<(), Refusal> {
        if at >= self.expires_at {
            return Err(Refusal::Expired);
        }
        if &self.target != target || !same(&self.proof, proof) {
            return Err(Refusal::NonceRefused);
        }
        Ok(())
    }

    pub(super) fn decision(
        &self,
        request: &Request,
        choice: Choice,
        proof: &[u8; 32],
        at: u64,
    ) -> Result<(), Refusal> {
        self.check(
            &Target::Decision {
                request: Box::new(request.clone()),
                choice,
            },
            proof,
            at,
        )
    }

    pub(super) fn revocation(
        &self,
        owner: &Owner,
        consent: &str,
        proof: &[u8; 32],
        at: u64,
    ) -> Result<(), Refusal> {
        self.check(
            &Target::Revocation {
                owner: owner.clone(),
                consent: consent.to_owned(),
            },
            proof,
            at,
        )
    }
}
