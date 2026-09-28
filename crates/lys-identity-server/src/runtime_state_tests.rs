#![cfg(test)]
//! The state the runtime reports seal is written by borrowing it: the bytes are exactly
//! the bytes the owned copy wrote, pinned here as they were written before.

use super::Held;

/// A sealed state as it was written before the seal borrowed what it seals.
const BEFORE: &[u8] = br#"{"format":"lys-runtime-state/v1","held":{"sessions":[{"session":"s-1","agent":"agent-a","machine":"m-1","reports":[{"operation":"op-1","session":"s-1","agent":"agent-a","machine":"m-1","state":"starting","what":"","confirmation":"","reported_by":"agent-a","at":5,"launch":{"agent":"agent-a","executed":false}},{"operation":"op-2","session":"s-1","agent":"agent-a","machine":"m-1","state":"stopped","what":"exited","confirmation":"exited 0","reported_by":"agent-a","at":6}]},{"session":"s-2","agent":null,"machine":"m-1","reports":[{"operation":"op-3","session":"s-2","agent":null,"machine":"m-1","state":"running","what":"","confirmation":"","reported_by":"person-a","at":7}]}]}}"#;

#[test]
fn the_sealed_bytes_are_the_bytes_written_before() -> Result<(), String> {
    let held = Held::decode(BEFORE)?;
    assert_eq!(
        String::from_utf8_lossy(&held.encode()?),
        String::from_utf8_lossy(BEFORE)
    );
    Ok(())
}
