//! The `SpiceDB` schema, written only here and only at start-up.
//!
//! The schema is the grant representation ADR-078 records (C5), held in
//! `schema.zed` beside this file in the form `SpiceDB` reads it back in, so a
//! stored schema is compared byte for byte. At start-up a store holding no
//! schema is given it and read back; a store holding any other schema is
//! refused by name, `spicedb_schema_mismatch`, and never overwritten.

use sha2::{Digest, Sha256};

use super::client::{SchemaRead, SpiceDbApi};
use super::error::SpiceDbError;
use super::wire::authzed::api::v1::{WriteSchemaRequest, WriteSchemaResponse};

/// The schema the identity server runs on.
pub const SCHEMA: &str = include_str!("schema.zed");

/// The schema write the client sends.
pub type SchemaWrite = WriteSchemaRequest;
/// The answer to a schema write.
pub type SchemaWritten = WriteSchemaResponse;

/// What start-up found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Started {
    /// The store held no schema, and now holds [`SCHEMA`].
    Written,
    /// The store already held [`SCHEMA`]; nothing was written.
    Unchanged,
}

/// Give a store with no schema [`SCHEMA`] and read it back, or confirm the
/// store already holds it. Any other stored schema is refused by name and
/// left as it is.
pub fn ensure(client: &dyn SpiceDbApi) -> Result<Started, SpiceDbError> {
    let mismatch = || SpiceDbError::SchemaMismatch {
        address: client.address().to_owned(),
    };
    match client.read_schema()? {
        SchemaRead::Schema(text) if text == SCHEMA => Ok(Started::Unchanged),
        SchemaRead::Schema(_) => Err(mismatch()),
        SchemaRead::NoSchema => {
            client.write_schema(WriteSchemaRequest {
                schema: SCHEMA.to_owned(),
            })?;
            match client.read_schema()? {
                SchemaRead::Schema(text) if text == SCHEMA => Ok(Started::Written),
                SchemaRead::Schema(_) | SchemaRead::NoSchema => Err(mismatch()),
            }
        }
    }
}

/// The SHA-256 of [`SCHEMA`] in lowercase hex: the policy an answer was
/// made under.
pub fn digest() -> String {
    crate::routes::hex(&Sha256::digest(SCHEMA.as_bytes()))
}
