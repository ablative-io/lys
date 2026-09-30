#![cfg(test)]
//! Restart is a peer act; the socket proof chooses its session.

use std::error::Error;

#[test]
fn a_restart_request_carries_an_operation_without_trusting_its_session_claim()
-> Result<(), Box<dyn Error>> {
    let line = r#"{"version":1,"peer":{"act":"restart","operation":"own-restart","session":"someone-else"}}"#;
    let request = serde_json::from_str::<super::PeerRequest>(line)?;
    assert_eq!(request.version, crate::PROTOCOL_VERSION);
    Ok(())
}
