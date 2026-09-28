//! Its virtual credentials are valid: read from the handle record SECRETS-002
//! keeps (its R1, ADR-001), through the one seam [`HandleRecords`].
//!
//! The seam answers ids and validity only. No method of it returns a
//! credential value, and no type it answers has a field that could hold
//! one: a record whose answer carried a value is refused as
//! `credential_value_in_answer`, naming the credential and the field and
//! never the value, and no id is taken from that answer. On a pass the
//! valid credential ids are handed to the launch record. While SECRETS-002's
//! record does not exist the check answers `check_record_missing`.

use crate::start::checks::Check;
use crate::start::error::{Refusal, StartError};

/// One virtual credential the agent holds: its id and whether it is valid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeldCredential {
    /// The credential's id, which is not the credential.
    pub id: String,
    /// Whether it is valid now.
    pub valid: bool,
}

/// What the handle record answers for an agent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HandleAnswer {
    /// The agent's virtual credentials, by id, with their validity.
    Held(Vec<HeldCredential>),
    /// No handle record exists for the agent.
    RecordMissing,
    /// The answer carried a credential value; nothing is taken from it.
    ValueInAnswer {
        /// The credential whose record carried a value.
        record: String,
        /// The field that carried it.
        field: String,
    },
    /// The record could not be read, for a reason that never quotes the answer.
    Unreadable {
        /// What failed.
        reason: String,
    },
}

impl HandleAnswer {
    /// The refusal this answer is, when it carried a credential value.
    pub fn refusal(&self) -> Option<Refusal> {
        match self {
            Self::ValueInAnswer { record, field } => Some(Refusal::CredentialValueInAnswer {
                record: record.clone(),
                field: field.clone(),
            }),
            Self::Held(_) | Self::RecordMissing | Self::Unreadable { .. } => None,
        }
    }
}

/// The handle record SECRETS-002 keeps: the one seam this check reads it through.
pub trait HandleRecords {
    /// The ids of `agent`'s virtual credentials and whether each is valid,
    /// or that the record does not exist, or the refusal of an answer that
    /// carried a value.
    fn handles(&self, agent: &str) -> HandleAnswer;
}

/// Its virtual credentials are valid: the valid credential ids to hand on,
/// or the refusal. Fails only when the record could not be read at all.
pub fn check(
    records: &dyn HandleRecords,
    agent: &str,
) -> Result<Result<Vec<String>, Refusal>, StartError> {
    match records.handles(agent) {
        HandleAnswer::Held(held) => {
            let valid: Vec<String> = held
                .into_iter()
                .filter(|credential| credential.valid)
                .map(|credential| credential.id)
                .collect();
            if valid.is_empty() {
                return Ok(Err(Refusal::VirtualCredentialsNotValid {
                    agent: agent.to_owned(),
                }));
            }
            Ok(Ok(valid))
        }
        HandleAnswer::RecordMissing => Ok(Err(Check::VirtualCredentialsAreValid.record_missing())),
        HandleAnswer::ValueInAnswer { record, field } => {
            Ok(Err(Refusal::CredentialValueInAnswer { record, field }))
        }
        HandleAnswer::Unreadable { reason } => Err(StartError::Unavailable {
            reason: format!(
                "the handle record SECRETS-002 keeps could not be read for {agent}: {reason}"
            ),
        }),
    }
}
