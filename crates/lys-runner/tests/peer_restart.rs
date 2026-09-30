#![cfg(test)]
//! Restart is admitted as a peer act without trusting a session claim.

use lys_runner::peer::PeerRequest;
use std::error::Error;

#[test]
fn a_restart_request_carries_an_operation_without_trusting_its_session_claim()
-> Result<(), Box<dyn Error>> {
    let line = r#"{"version":1,"peer":{"act":"restart","operation":"own-restart","session":"someone-else"}}"#;
    let request = serde_json::from_str::<PeerRequest>(line)?;
    assert_eq!(request.version, lys_runner::PROTOCOL_VERSION);
    Ok(())
}
