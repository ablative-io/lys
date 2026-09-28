#![cfg(test)]
//! The state the certificates seal is written by borrowing it: the bytes are exactly
//! the bytes the owned copy wrote, pinned here as they were written before.

use super::Held;

/// A sealed state as it was written before the seal borrowed what it seals.
const BEFORE: &[u8] = br#"{"format":"lys-certificates-state/v1","held":{"certificates":{"01":{"issued":{"serial":"01","agent":"agent-a","person":"person-a","claims":{"roles":["reader"],"state":"active"},"der":"MAo=","issued_at":5},"leaf":0,"withdrawn":null},"02":{"issued":{"serial":"02","agent":"agent-b","person":"person-a","claims":{},"der":"MAo=","issued_at":6},"leaf":1,"withdrawn":{"serial":"02","by":"person-a","reason":"lost","withdrawn_at":7}}}}}"#;

#[test]
fn the_sealed_bytes_are_the_bytes_written_before() -> Result<(), String> {
    let held = Held::decode(BEFORE)?;
    assert_eq!(
        String::from_utf8_lossy(&held.encode()?),
        String::from_utf8_lossy(BEFORE)
    );
    Ok(())
}
