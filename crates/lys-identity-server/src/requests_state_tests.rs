#![cfg(test)]
//! The state the access requests seal is written by borrowing it: the bytes are exactly
//! the bytes the owned copy wrote, pinned here as they were written before.

use super::Held;

/// A sealed state as it was written before the seal borrowed what it seals.
const BEFORE: &[u8] = br#"{"format":"lys-requests-state/v1","held":{"asked":[{"id":"r-1","asked_by":"person-a","responsible":"person-b","resource_kind":"repository","resource_id":"lys","relation":"reader","ends_at":null,"why":"to read","asked_at":5},{"id":"r-2","asked_by":"person-a","responsible":"person-b","resource_kind":"repository","resource_id":"lys","relation":"writer","ends_at":90,"why":"to write","asked_at":6}],"intended":{"r-2":{"id":"r-2","by":"person-b","operation":"op-2","source":null,"note":"","intended_at":7}},"decided":{"r-1":{"id":"r-1","by":"person-b","approved":true,"note":"yes","grant":"g-1","decided_at":8}}}}"#;

#[test]
fn the_sealed_bytes_are_the_bytes_written_before() -> Result<(), String> {
    let held = Held::decode(BEFORE)?;
    assert_eq!(
        String::from_utf8_lossy(&held.encode()?),
        String::from_utf8_lossy(BEFORE)
    );
    Ok(())
}
