#![cfg(test)]
//! The state the review decisions seal is written by borrowing it: the bytes are exactly
//! the bytes the owned copy wrote, pinned here as they were written before.

use super::Held;

/// A sealed state as it was written before the seal borrowed what it seals.
const BEFORE: &[u8] = br#"{"format":"lys-review-decisions-state/v1","held":{"kept":[{"grant":"g-1","kept_by":"person-a","note":"still needed","operation":"op-1","at":5,"revision":2},{"grant":"g-2","kept_by":"person-b","note":"","operation":"op-2","at":6,"revision":3}]}}"#;

#[test]
fn the_sealed_bytes_are_the_bytes_written_before() -> Result<(), String> {
    let held = Held::decode(BEFORE)?;
    assert_eq!(
        String::from_utf8_lossy(&held.encode()?),
        String::from_utf8_lossy(BEFORE)
    );
    Ok(())
}
