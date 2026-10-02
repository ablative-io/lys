//! An agent call record refuses anything but a change a route made.

use super::AgentCall;

const DIGEST: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

#[test]
fn a_change_a_route_made_is_recorded_as_given() -> Result<(), crate::IdentityError> {
    let call = AgentCall::new("POST", "/agents/a/goals?x=1", DIGEST, "agent 1 nonce sig")?;
    assert_eq!(call.method(), "POST");
    assert_eq!(call.path(), "/agents/a/goals?x=1");
    assert_eq!(call.body_sha256(), DIGEST);
    assert_eq!(call.signature(), "agent 1 nonce sig");
    Ok(())
}

#[test]
fn a_read_or_a_foreign_route_is_not_a_call() {
    assert!(AgentCall::new("GET", "/tree", DIGEST, "").is_err());
    assert!(AgentCall::new("POST", "https://foreign.invalid/x", DIGEST, "").is_err());
    assert!(AgentCall::new("POST", "//foreign.invalid/x", DIGEST, "").is_err());
    assert!(AgentCall::new("POST", "/tree", "E3B0", "").is_err());
    assert!(AgentCall::new("POST", "/tree", &DIGEST.to_uppercase(), "").is_err());
    assert!(AgentCall::new("POST", "/tree", DIGEST, &"s".repeat(4097)).is_err());
}
