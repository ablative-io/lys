#![cfg(test)]
//! The state the teams seal is written by borrowing it: the bytes are exactly
//! the bytes the owned copy wrote, pinned here as they were written before.

use super::Held;

/// A sealed state as it was written before the seal borrowed what it seals.
const BEFORE: &[u8] = br#"{"format":"lys-teams-state/v1","held":{"teams":[{"created":{"id":"op-1","owner":"person-a","name":"Crew","description":"","by":{"provider":"https://issuer.test","subject":"ada"},"at":5},"members":["agent-a"],"retired":null,"changes":[{"line":"created","id":"op-1","owner":"person-a","name":"Crew","description":"","by":{"provider":"https://issuer.test","subject":"ada"},"at":5},{"line":"added","operation":"op-2","team":"op-1","member":"agent-a","by":{"provider":"https://issuer.test","subject":"ada"},"at":6}]}]}}"#;

#[test]
fn the_sealed_bytes_are_the_bytes_written_before() -> Result<(), String> {
    let held = Held::decode(BEFORE)?;
    assert_eq!(
        String::from_utf8_lossy(&held.encode()?),
        String::from_utf8_lossy(BEFORE)
    );
    Ok(())
}
