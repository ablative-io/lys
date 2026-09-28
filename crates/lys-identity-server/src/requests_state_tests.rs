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
        "asked": [{
            "id": "op-1", "asked_by": "agent-1", "responsible": "person-1",
            "resource_kind": "repository", "resource_id": "lys", "relation": "reader",
            "ends_at": 90, "why": "to read", "asked_at": 10,
        }],
        "intended": { "op-2": {
            "id": "op-2", "by": "person-1", "operation": "op-3", "source": null,
            "note": "yes", "intended_at": 11,
        }},
        "decided": { "op-4": {
            "id": "op-4", "by": "person-1", "approved": true, "note": "fine",
            "grant": "grant-1", "decided_at": 12,
        }},
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
