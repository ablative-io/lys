//! The `lys.given_statement` entry: what a session was given, signed.
//!
//! A render given a key signs the canonical bytes of its `lys.given` record
//! (see [`crate::record::given`]) with lys-core's `lys/attestation/v2`, keeps
//! the `COSE_Sign1` bytes as a block, and appends one custom entry as the
//! child of the `lys.given` entry, with data exactly `{given, statement}`:
//! the given entry's id and the block's hash. The entry holds no payload
//! byte, no key byte and no document content, and it stands beside the
//! context path as the given entry does, so the head does not move. The
//! statement is checked offline with `lys verify --attestation` against the
//! canonical bytes; statements are read back with `customs_everywhere`, in
//! file order.

use lys_core::Ed25519Identity;
use lys_core::attestation::sign_attestation;
use serde::{Deserialize, Serialize};

use crate::error::HomeError;
use crate::record::Session;
use crate::record::blocks::BlockStore;
use crate::record::entries::{CUSTOM_GIVEN_STATEMENT, EntryBody};
use crate::record::given::CanonicalGiven;

/// The data of a `lys.given_statement` entry.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GivenStatement {
    /// The id of the `lys.given` entry the statement signs.
    pub given: String,
    /// The hash of the block holding the statement's `COSE_Sign1` bytes.
    pub statement: String,
}

/// A statement just signed and kept.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Signed {
    /// The id of the `lys.given_statement` entry.
    pub entry: String,
    /// The statement's `COSE_Sign1` bytes, as kept in the block.
    pub cose: Vec<u8>,
}

impl GivenStatement {
    /// Sign `canonical`, the record of the `lys.given` entry `given`, with
    /// `key`, keep the statement as a block and append the entry under
    /// `given`, leaving the head where it stands.
    pub fn sign(
        session: &mut Session,
        blocks: &BlockStore,
        given: &str,
        canonical: &CanonicalGiven,
        key: &Ed25519Identity,
    ) -> Result<Signed, HomeError> {
        let cose = sign_attestation(canonical.as_bytes(), key).to_cose_bytes();
        let data = Self {
            given: given.to_owned(),
            statement: blocks.put(&cose)?.hash.as_str().to_owned(),
        };
        let data = serde_json::to_value(&data).map_err(|source| HomeError::Json {
            context: "a given statement's entry could not be serialised",
            source,
        })?;
        let entry = session.append_under(
            given,
            EntryBody::Custom {
                custom_type: CUSTOM_GIVEN_STATEMENT.to_owned(),
                data: Some(data),
            },
        )?;
        Ok(Signed { entry, cose })
    }

    /// Every `lys.given_statement` entry of the session in file order, each
    /// with its id.
    pub fn read_all(session: &Session) -> Result<Vec<(String, Self)>, HomeError> {
        session
            .customs_everywhere(CUSTOM_GIVEN_STATEMENT)?
            .iter()
            .map(|entry| {
                let data = match &entry.body {
                    EntryBody::Custom { data, .. } => data.clone(),
                    _ => None,
                };
                let statement = serde_json::from_value(data.unwrap_or_default()).map_err(
                    |source| HomeError::Json {
                        context: "a lys.given_statement entry's data is not the statement's shape",
                        source,
                    },
                )?;
                Ok((entry.id().to_owned(), statement))
            })
            .collect()
    }
}
