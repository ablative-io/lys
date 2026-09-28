#![cfg(test)]
//! The state the emergency stops seal is written by borrowing it: the bytes are exactly
//! the bytes the owned copy wrote, pinned here as they were written before.

use super::Held;

/// A sealed state as it was written before the seal borrowed what it seals.
const BEFORE: &[u8] = br#"{"format":"lys-stops-state/v1","held":{"stops":[{"operation":"op-1","agent":"agent-a","by":"person-a","reason":"runaway","at":5,"certificates_withdrawn":["01"],"sessions_asked":["op-2"],"credentials_ended":null,"credentials_refused":"the broker is not configured","done":true},{"operation":"op-3","agent":"agent-b","by":"person-a","reason":"","at":6,"certificates_withdrawn":[],"sessions_asked":[],"credentials_ended":["h-1"],"credentials_refused":null,"done":false}]}}"#;

#[test]
fn the_sealed_bytes_are_the_bytes_written_before() -> Result<(), String> {
    let held = Held::decode(BEFORE)?;
    assert_eq!(
        String::from_utf8_lossy(&held.encode()?),
        String::from_utf8_lossy(BEFORE)
    );
    Ok(())
}
