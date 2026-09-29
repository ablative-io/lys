//! Pure consent decision planning. These types deliberately have no wire/log
//! serialization, token output, nonce generator or durable append operation.

use std::collections::BTreeSet;

use lys_identity::PersonId;

use super::{ActionNonce, Binding, Refusal, exact};

/// A closed proposed operation vocabulary, not a runtime route allowlist.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Operation {
    /// Read the original person's own identity.
    SelfIdentity,
    /// Read their own person/agent records.
    OwnPeople,
    /// Read directory people only when the original actor is admitted.
    DirectoryPeople,
    /// Read teams under existing visibility.
    Teams,
    /// Read grants under existing standing/visibility.
    Grants,
    /// Read the grant model under existing admission.
    GrantModel,
    /// Read the exact existing caller-visible identity set.
    VisibleIdentities,
    /// Proposed live Status reconciliation; requires separately reviewed provenance.
    LiveSessions,
}

/// Already-authenticated person and public session identity; never a cookie.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Owner {
    /// The resolved person.
    pub person: PersonId,
    /// The session store's public id, 16 random bytes in lowercase hex.
    pub session: String,
}

impl Owner {
    pub(super) fn valid(&self) -> bool {
        self.session.len() == 32
            && self
                .session
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    }
}

/// Candidate consent words. All authority coordinates are server-resolved;
/// deserializing untrusted values into these facts must never admit a caller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    /// Stable pending-request/consent identity.
    pub consent: String,
    /// Original person and session.
    pub owner: Owner,
    /// Exact approved-app revision binding.
    pub binding: Binding,
    /// Closed operation identifiers to display and request.
    pub operations: BTreeSet<Operation>,
    /// When the request was created.
    pub issued_at: u64,
    /// No later than the original session's end.
    pub expires_at: u64,
}

impl Request {
    /// Structural validation only. No actor is authenticated and nothing is issued.
    ///
    /// # Errors
    /// Refuses malformed identities, empty operation sets or invalid lifetime.
    pub fn validate(&self) -> Result<(), Refusal> {
        if !exact(&self.consent)
            || !self.owner.valid()
            || self.operations.is_empty()
            || self.expires_at <= self.issued_at
        {
            return Err(Refusal::RecordInvalid);
        }
        Ok(())
    }
}

/// Fresh facts an eventual service resolver must obtain from trusted live stores.
/// Constructing this value from request claims would bypass the whole boundary.
pub struct Current<'a> {
    /// Current authenticated session's person/public id.
    pub owner: &'a Owner,
    /// Whether the resolved subject remains active.
    pub active: bool,
    /// Current live session end instant.
    pub session_ends_at: u64,
    /// Fresh approved-app binding, absent when no longer approved.
    pub binding: Option<&'a Binding>,
    /// Check instant supplied by the service, never the remote app.
    pub at: u64,
}

impl Current<'_> {
    fn authenticate(&self) -> Result<(), Refusal> {
        if !self.owner.valid() || !self.active {
            return Err(Refusal::OwnerRefused);
        }
        if self.at >= self.session_ends_at {
            return Err(Refusal::Expired);
        }
        Ok(())
    }

    fn request(&self, request: &Request) -> Result<(), Refusal> {
        self.authenticate()?;
        request.validate()?;
        if self.owner != &request.owner {
            return Err(Refusal::OwnerRefused);
        }
        if self.binding != Some(&request.binding) {
            return Err(Refusal::BindingChanged);
        }
        if self.at < request.issued_at
            || self.at >= request.expires_at
            || request.expires_at > self.session_ends_at
        {
            return Err(Refusal::Expired);
        }
        Ok(())
    }
}

/// The explicit decision; neither value alone grants data access.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Choice {
    /// The person agreed to the displayed words.
    Approve,
    /// The person declined them.
    Decline,
}

/// Canonical comparison inputs for one decision attempt, without a raw nonce.
/// Wire encoding and signed-record version are intentionally not defined yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecisionCommand {
    operation: String,
    request: Request,
    choice: Choice,
    proof: [u8; 32],
}

impl DecisionCommand {
    /// Keep exact decision words and only the nonce digest for retry comparison.
    ///
    /// # Errors
    /// Refuses malformed request/operation or an absent proof.
    pub fn new(
        operation: String,
        request: Request,
        choice: Choice,
        nonce: &str,
    ) -> Result<Self, Refusal> {
        request.validate()?;
        if !exact(&operation) || nonce.is_empty() {
            return Err(Refusal::RecordInvalid);
        }
        Ok(Self {
            operation,
            request,
            choice,
            proof: super::nonce::digest(nonce),
        })
    }
}

/// Historical decision receipt; not evidence that consent is currently standing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecisionRecord {
    command: DecisionCommand,
    at: u64,
}

/// Pure append/readback plan. Neither variant contains a code or token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecisionPlan {
    /// Candidate to append atomically with nonce consumption after encoding review.
    Append(DecisionRecord),
    /// Original receipt only: never append, remint, or reactivate from this variant.
    Replay(DecisionRecord),
}

/// Plan a decision against its existing operation/consent receipt, if any.
/// Authenticate the same owner first; compare a recorded command before asking
/// for a fresh nonce or live app binding. Replay is historical readback only.
/// The caller must look up both operation and consent and refuse conflicting
/// records before supplying `recorded`; this pure function is not a database.
///
/// # Errors
/// Refuses a different owner/body, stale new request or absent/wrong action nonce.
pub fn plan_decision(
    command: &DecisionCommand,
    recorded: Option<&DecisionRecord>,
    nonce: Option<&ActionNonce>,
    current: &Current<'_>,
) -> Result<DecisionPlan, Refusal> {
    current.authenticate()?;
    if current.owner != &command.request.owner {
        return Err(Refusal::OwnerRefused);
    }
    if let Some(record) = recorded {
        if record.command != *command {
            return Err(Refusal::OperationReused);
        }
        return Ok(DecisionPlan::Replay(record.clone()));
    }
    current.request(&command.request)?;
    nonce.ok_or(Refusal::NonceRefused)?.decision(
        &command.request,
        command.choice,
        &command.proof,
        current.at,
    )?;
    Ok(DecisionPlan::Append(DecisionRecord {
        command: command.clone(),
        at: current.at,
    }))
}

/// Judge current consent independently of any historical replay receipt.
/// No route accepts this result yet; live-session reconciliation still requires
/// its reviewed event schema, and no encoding/issuance is enabled here.
///
/// # Errors
/// Refuses withdrawal/decline, changed ownership/binding, expiry or inactivity.
pub fn standing(
    record: &DecisionRecord,
    withdrawn: bool,
    current: &Current<'_>,
) -> Result<(), Refusal> {
    current.authenticate()?;
    if withdrawn || record.command.choice != Choice::Approve {
        return Err(Refusal::NotStanding);
    }
    current.request(&record.command.request)
}

/// An owner-authenticated revocation attempt, bound to its current session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevocationCommand {
    operation: String,
    consent: String,
    owner: Owner,
    proof: [u8; 32],
}

impl RevocationCommand {
    /// Compare a retry without storing the raw action nonce.
    ///
    /// # Errors
    /// Refuses malformed identities, operation or absent nonce.
    pub fn new(
        operation: String,
        consent: String,
        owner: Owner,
        nonce: &str,
    ) -> Result<Self, Refusal> {
        if !exact(&operation) || !exact(&consent) || !owner.valid() || nonce.is_empty() {
            return Err(Refusal::RecordInvalid);
        }
        Ok(Self {
            operation,
            consent,
            owner,
            proof: super::nonce::digest(nonce),
        })
    }
}

/// Candidate historical withdrawal; it contains no credential or issuance result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevocationRecord {
    command: RevocationCommand,
    at: u64,
}

/// Pure revocation append or exact historical readback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RevocationPlan {
    /// Append atomically; only a reviewed durable append may report revocation done.
    Append(RevocationRecord),
    /// Return the earlier receipt without writing again.
    Replay(RevocationRecord),
}

/// Plan withdrawal by the consent's person, including from a later live session.
/// A changed/retired app must not prevent its person from withdrawing consent.
/// A decision nonce can never authorize this separate action.
///
/// # Errors
/// Refuses the wrong owner/consent, changed retry body or missing revoke proof.
pub fn plan_revocation(
    command: &RevocationCommand,
    decision: &DecisionRecord,
    recorded: Option<&RevocationRecord>,
    nonce: Option<&ActionNonce>,
    current: &Current<'_>,
) -> Result<RevocationPlan, Refusal> {
    current.authenticate()?;
    if current.owner != &command.owner
        || current.owner.person != decision.command.request.owner.person
        || command.consent != decision.command.request.consent
    {
        return Err(Refusal::OwnerRefused);
    }
    if let Some(record) = recorded {
        if record.command != *command {
            return Err(Refusal::OperationReused);
        }
        return Ok(RevocationPlan::Replay(record.clone()));
    }
    if decision.command.choice != Choice::Approve {
        return Err(Refusal::NotStanding);
    }
    nonce.ok_or(Refusal::NonceRefused)?.revocation(
        &command.owner,
        &command.consent,
        &command.proof,
        current.at,
    )?;
    Ok(RevocationPlan::Append(RevocationRecord {
        command: command.clone(),
        at: current.at,
    }))
}

#[cfg(test)]
#[path = "consent_tests.rs"]
mod tests;
