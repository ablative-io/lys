//! The stable encoded form of a refusal the grant book keeps by name.
//!
//! A committed event the book refuses is kept with its refusal, and a
//! snapshot of the book carries it. Each refusal the book can make has a
//! fixed code and its members in a fixed order; the code of a variant is
//! never reused. A refusal the book cannot make has no code, and a book
//! holding one is not written to a snapshot.

use ciborium::Value;

use super::codec::recipient_code;
use super::error::GrantError;
use super::projection::USE_BY_HOLDER;
use super::types::RecipientKind;
use crate::state_value::{Unreadable, array, list, read_text, read_uint, text, uint};

const OPERATION_REUSED: u64 = 1;
const GRANT_EXISTS: u64 = 2;
const SOURCE_UNKNOWN: u64 = 3;
const ISSUER_NOT_HOLDER: u64 = 4;
const RESOURCE_OUTSIDE: u64 = 5;
const USE_ONLY: u64 = 6;
const RECIPIENT_REFUSED: u64 = 7;
const ACTIONS_OUTSIDE: u64 = 8;
const PASS_ON_BEYOND_SOURCE: u64 = 9;
const EXPIRY_BEYOND_SOURCE: u64 = 10;
const GRANT_UNKNOWN: u64 = 11;
const ALREADY_REVOKED: u64 = 12;
const EVENT_MISMATCH: u64 = 13;

/// The reasons an `EventMismatch` the book keeps may give.
const MISMATCH_REASONS: [&str; 1] = [USE_BY_HOLDER];

fn coded(code: u64, members: Vec<Value>) -> Value {
    let mut items = vec![uint(code)];
    items.extend(members);
    array(items)
}

/// The refusal's code and members, or why it has no stable form.
pub(crate) fn encode_refusal(refusal: &GrantError) -> Result<Value, Unreadable> {
    Ok(match refusal {
        GrantError::OperationReused { operation } => coded(OPERATION_REUSED, vec![text(operation)]),
        GrantError::GrantExists { grant } => coded(GRANT_EXISTS, vec![text(grant)]),
        GrantError::SourceUnknown { grant } => coded(SOURCE_UNKNOWN, vec![text(grant)]),
        GrantError::IssuerNotHolder {
            grant,
            source_grant,
        } => coded(ISSUER_NOT_HOLDER, vec![text(grant), text(source_grant)]),
        GrantError::ResourceOutside {
            requested,
            source_grant,
        } => coded(RESOURCE_OUTSIDE, vec![text(requested), text(source_grant)]),
        GrantError::UseOnly { grant } => coded(USE_ONLY, vec![text(grant)]),
        GrantError::RecipientRefused { kind, grant } => coded(
            RECIPIENT_REFUSED,
            vec![uint(recipient_code(*kind)), text(grant)],
        ),
        GrantError::ActionsOutside {
            relation,
            outside,
            model_version,
        } => coded(
            ACTIONS_OUTSIDE,
            vec![text(relation), text(outside), uint(*model_version)],
        ),
        GrantError::PassOnBeyondSource { source_grant } => {
            coded(PASS_ON_BEYOND_SOURCE, vec![text(source_grant)])
        }
        GrantError::ExpiryBeyondSource {
            requested,
            source_grant,
            source_ends,
        } => coded(
            EXPIRY_BEYOND_SOURCE,
            vec![text(requested), text(source_grant), uint(*source_ends)],
        ),
        GrantError::GrantUnknown { grant } => coded(GRANT_UNKNOWN, vec![text(grant)]),
        GrantError::AlreadyRevoked { grant } => coded(ALREADY_REVOKED, vec![text(grant)]),
        GrantError::EventMismatch { reason } => coded(EVENT_MISMATCH, vec![text(reason)]),
        other => {
            return Err(format!(
                "the book holds a refusal with no stable encoded form: {other}"
            ));
        }
    })
}

/// Reads the next member of a refusal as text.
fn next_text(members: &mut std::vec::IntoIter<Value>) -> Result<String, Unreadable> {
    read_text(
        members.next().ok_or("a refusal is missing a member")?,
        "a refusal's member",
    )
}

/// Reads the next member of a refusal as an unsigned integer.
fn next_uint(members: &mut std::vec::IntoIter<Value>) -> Result<u64, Unreadable> {
    read_uint(
        &members.next().ok_or("a refusal is missing a member")?,
        "a refusal's member",
    )
}

/// The refusal a code and its members name.
pub(crate) fn decode_refusal(value: Value) -> Result<GrantError, Unreadable> {
    let mut members = list(value, "a refusal")?.into_iter();
    let code = next_uint(&mut members)?;
    let members = &mut members;
    let refusal = match code {
        OPERATION_REUSED => GrantError::OperationReused {
            operation: next_text(members)?,
        },
        GRANT_EXISTS => GrantError::GrantExists {
            grant: next_text(members)?,
        },
        SOURCE_UNKNOWN => GrantError::SourceUnknown {
            grant: next_text(members)?,
        },
        ISSUER_NOT_HOLDER => GrantError::IssuerNotHolder {
            grant: next_text(members)?,
            source_grant: next_text(members)?,
        },
        RESOURCE_OUTSIDE => GrantError::ResourceOutside {
            requested: next_text(members)?,
            source_grant: next_text(members)?,
        },
        USE_ONLY => GrantError::UseOnly {
            grant: next_text(members)?,
        },
        RECIPIENT_REFUSED => GrantError::RecipientRefused {
            kind: match next_uint(members)? {
                1 => RecipientKind::Person,
                2 => RecipientKind::Agent,
                other => return Err(format!("recipient kind {other} is not a kind")),
            },
            grant: next_text(members)?,
        },
        ACTIONS_OUTSIDE => GrantError::ActionsOutside {
            relation: next_text(members)?,
            outside: next_text(members)?,
            model_version: next_uint(members)?,
        },
        PASS_ON_BEYOND_SOURCE => GrantError::PassOnBeyondSource {
            source_grant: next_text(members)?,
        },
        EXPIRY_BEYOND_SOURCE => GrantError::ExpiryBeyondSource {
            requested: next_text(members)?,
            source_grant: next_text(members)?,
            source_ends: next_uint(members)?,
        },
        GRANT_UNKNOWN => GrantError::GrantUnknown {
            grant: next_text(members)?,
        },
        ALREADY_REVOKED => GrantError::AlreadyRevoked {
            grant: next_text(members)?,
        },
        EVENT_MISMATCH => {
            let found = next_text(members)?;
            let reason = MISMATCH_REASONS
                .into_iter()
                .find(|reason| *reason == found)
                .ok_or_else(|| format!("`{found}` is not a mismatch the book keeps"))?;
            GrantError::EventMismatch { reason }
        }
        other => {
            return Err(format!(
                "refusal code {other} is not a refusal the book keeps"
            ));
        }
    };
    if members.next().is_some() {
        return Err(format!(
            "refusal code {code} carries more members than it has"
        ));
    }
    Ok(refusal)
}

#[cfg(test)]
mod tests {
    use super::{decode_refusal, encode_refusal};
    use crate::grants::error::GrantError;
    use crate::grants::projection::USE_BY_HOLDER;
    use crate::grants::types::RecipientKind;

    #[test]
    fn every_refusal_the_book_keeps_reads_back_by_name() -> Result<(), String> {
        let refusals = [
            GrantError::OperationReused {
                operation: "op".to_owned(),
            },
            GrantError::GrantExists {
                grant: "g".to_owned(),
            },
            GrantError::SourceUnknown {
                grant: "g".to_owned(),
            },
            GrantError::IssuerNotHolder {
                grant: "g".to_owned(),
                source_grant: "s".to_owned(),
            },
            GrantError::ResourceOutside {
                requested: "r".to_owned(),
                source_grant: "s".to_owned(),
            },
            GrantError::UseOnly {
                grant: "g".to_owned(),
            },
            GrantError::RecipientRefused {
                kind: RecipientKind::Agent,
                grant: "g".to_owned(),
            },
            GrantError::ActionsOutside {
                relation: "kite".to_owned(),
                outside: "delete".to_owned(),
                model_version: 3,
            },
            GrantError::PassOnBeyondSource {
                source_grant: "s".to_owned(),
            },
            GrantError::ExpiryBeyondSource {
                requested: "no end".to_owned(),
                source_grant: "s".to_owned(),
                source_ends: 9,
            },
            GrantError::GrantUnknown {
                grant: "g".to_owned(),
            },
            GrantError::AlreadyRevoked {
                grant: "g".to_owned(),
            },
            GrantError::EventMismatch {
                reason: USE_BY_HOLDER,
            },
        ];
        for refusal in refusals {
            assert_eq!(decode_refusal(encode_refusal(&refusal)?)?, refusal);
        }
        Ok(())
    }

    #[test]
    fn a_refusal_the_book_never_keeps_has_no_encoded_form() {
        assert!(encode_refusal(&GrantError::PassOnOutside).is_err());
    }
}
