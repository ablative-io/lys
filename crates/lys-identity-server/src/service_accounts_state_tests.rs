#![cfg(test)]
//! The state the service accounts seal is written by borrowing it: the bytes are exactly
//! the bytes the owned copy wrote, pinned here as they were written before.

use super::Held;

/// A sealed state as it was written before the seal borrowed what it seals.
const BEFORE: &[u8] = br#"{"format":"lys-service-accounts-state/v1","held":{"accounts":[{"created":{"id":"op-1","owner":"person-a","name":"builder","description":"builds","by":{"provider":"https://issuer.test","subject":"ada"},"at":5},"retired":null},{"created":{"id":"op-2","owner":"person-a","name":"old","description":"","by":{"provider":"https://issuer.test","subject":"ada"},"at":6},"retired":{"operation":"op-3","account":"op-2","by":{"provider":"https://issuer.test","subject":"ada"},"at":7}}]}}"#;

#[test]
fn the_sealed_bytes_are_the_bytes_written_before() -> Result<(), String> {
    let held = Held::decode(BEFORE)?;
    assert_eq!(
        String::from_utf8_lossy(&held.encode()?),
        String::from_utf8_lossy(BEFORE)
    );
    Ok(())
}
