#![cfg(test)]
//! Gates on the sealed state: a snapshot seals the bytes an owned copy of
//! the state sealed before sealing borrowed it, and reads back to the same.

use serde::Serialize;
use serde_json::json;

use super::{FORMAT, Held};

/// A snapshot sealed as it was before it borrowed: an owned copy of the
/// state beside its format.
#[derive(Serialize)]
struct Copied {
    format: String,
    held: Held,
}

#[test]
fn a_snapshot_seals_the_bytes_an_owned_copy_sealed() -> Result<(), String> {
    let held: Held = serde_json::from_value(json!({
        "accounts": [{
            "created": {
                "id": "op-1", "owner": "person-1", "name": "builder",
                "description": "builds",
                "by": { "provider": "https://id.test", "subject": "ada" },
                "at": 10,
            },
            "retired": {
                "operation": "op-2", "account": "op-1",
                "by": { "provider": "https://id.test", "subject": "ada" },
                "at": 11,
            },
        }],
    }))
    .map_err(|error| error.to_string())?;
    let copied = serde_json::to_vec(&Copied {
        format: FORMAT.to_owned(),
        held: held.clone(),
    })
    .map_err(|error| error.to_string())?;
    assert_eq!(held.encode()?, copied);
    assert_eq!(Held::decode(&copied)?, held);
    Ok(())
}
