#![cfg(test)]

use crate::AgentId;
use crate::event::wire;

#[test]
fn pass_run_provenance_is_stable_distinct_and_has_no_authentication_time() {
    let agent = AgentId::from_bytes([7; 16]);
    let first = super::Provenance::by_pass(agent, "launch", "session").expect("run");
    assert_eq!(first.authenticated_at(), None);
    assert_eq!(first.run(), Some(("launch", "session")));
    assert_eq!(
        first,
        super::Provenance::by_pass(agent, "launch", "session").expect("retry")
    );
    assert_ne!(
        first,
        super::Provenance::by_pass(agent, "launch", "another").expect("run")
    );
    let actor = super::Actor::new(
        crate::LoginBinding::new("https://issuer.test", "agent").expect("binding"),
        first,
    );
    let mut encoded = Vec::new();
    crate::encoding::actor(&mut encoded, &actor);
    assert_eq!(
        crate::encoding::decode_actor(crate::encoding::cbor(&encoded, "actor").expect("CBOR"), 2)
            .expect("actor"),
        actor
    );
    assert!(super::Provenance::by_pass(agent, "", "session").is_err());
    assert!(super::Provenance::by_pass(agent, "launch", &"s".repeat(129)).is_err());
}

#[test]
fn pass_actor_method_round_trips_without_claiming_a_signature() {
    let agent = AgentId::from_bytes([7; 16]);
    let method = wire::method_from(4, Some(agent)).expect("pass method decodes");
    assert_eq!(wire::method(method), 4);
    let provenance = super::Provenance::new(method, 13);
    assert_eq!(provenance.authenticated_at(), Some(13));
    assert_eq!(provenance.run(), None);
    assert_eq!(provenance.agent(), Some(agent));
    assert_eq!(provenance.service_account(), None);
    assert!(wire::method_from(4, None).is_err());
    assert_ne!(method, super::AuthMethod::AgentSignature(agent));
}

#[test]
fn pass_actor_bytes_round_trip_and_historical_methods_keep_their_codes() {
    let agent = AgentId::from_bytes([7; 16]);
    let binding = crate::LoginBinding::new("https://issuer.test", "agent").expect("binding");
    let actor = super::Actor::new(
        binding,
        super::Provenance::new(super::AuthMethod::AgentPass(agent), 13),
    );
    let mut encoded = Vec::new();
    crate::encoding::actor(&mut encoded, &actor);
    let value = crate::encoding::cbor(&encoded, "actor").expect("CBOR");
    assert_eq!(
        crate::encoding::decode_actor(value, 2).expect("actor"),
        actor
    );
    for (code, principal, expected) in [
        (1, None, super::AuthMethod::Oidc),
        (2, Some(agent), super::AuthMethod::AgentSignature(agent)),
        (3, None, super::AuthMethod::Operator),
        (
            3,
            Some(agent),
            super::AuthMethod::ServiceAccountBearer(crate::ServiceAccountId::from_bytes([7; 16])),
        ),
    ] {
        assert_eq!(
            wire::method_from(code, principal).expect("historical method"),
            expected
        );
        assert_eq!(wire::method(expected), code);
    }
}
