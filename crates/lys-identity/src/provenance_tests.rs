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
    assert!(super::Provenance::by_pass(agent, "launch", "").is_err());
    let (launch, session) = ("l".repeat(10_000), "s".repeat(10_000));
    let long = super::Actor::new(
        crate::LoginBinding::new("https://issuer.test", "agent").expect("binding"),
        super::Provenance::by_pass(agent, &launch, &session).expect("long identifiers"),
    );
    let mut encoded = Vec::new();
    crate::encoding::actor(&mut encoded, &long);
    assert_eq!(
        crate::encoding::decode_actor(crate::encoding::cbor(&encoded, "actor").expect("CBOR"), 2)
            .expect("actor"),
        long,
        "long run identifiers come back whole"
    );
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
        (5, None, super::AuthMethod::Founding),
        (6, None, super::AuthMethod::AddressProof),
    ] {
        assert_eq!(
            wire::method_from(code, principal).expect("historical method"),
            expected
        );
        assert_eq!(wire::method(expected), code);
    }
}

/// DIRECTORY-111: a founding leaf and a proof-begun session name no agent
/// and no service account; the proof is the person signed in, the founding
/// is not, and neither code carries a principal.
#[test]
fn founding_and_address_proof_round_trip_and_only_the_proof_is_the_person_signed_in() {
    let binding =
        crate::LoginBinding::new("https://issuer.test", "founding:op-1").expect("binding");
    for (method, signed_in) in [
        (super::AuthMethod::Founding, false),
        (super::AuthMethod::AddressProof, true),
        (super::AuthMethod::Oidc, true),
        (super::AuthMethod::Operator, false),
    ] {
        assert_eq!(method.is_person_signed_in(), signed_in, "{method:?}");
        let actor = super::Actor::new(binding.clone(), super::Provenance::new(method, 13));
        assert_eq!(actor.provenance().agent(), None);
        assert_eq!(actor.provenance().service_account(), None);
        let mut encoded = Vec::new();
        crate::encoding::actor(&mut encoded, &actor);
        let value = crate::encoding::cbor(&encoded, "actor").expect("CBOR");
        assert_eq!(
            crate::encoding::decode_actor(value, 2).expect("actor"),
            actor
        );
    }
    let agent = AgentId::from_bytes([9; 16]);
    assert!(wire::method_from(5, Some(agent)).is_err());
    assert!(wire::method_from(6, Some(agent)).is_err());
    assert!(wire::method_from(8, None).is_err());
}

/// DIRECTORY-112 R2: a session begun by a handed link is the person signed
/// in, carried as code 7 with no principal; 8 is the next free code.
#[test]
fn a_handed_link_is_code_seven_with_no_principal_and_the_person_signed_in() {
    let method = wire::method_from(7, None).expect("the handed-link method");
    assert!(method.is_person_signed_in(), "{method:?}");
    assert_eq!(wire::method(method), 7);
    let binding = crate::LoginBinding::new("https://issuer.test", "subject").expect("binding");
    let actor = super::Actor::new(binding, super::Provenance::new(method, 13));
    assert_eq!(actor.provenance().agent(), None);
    assert_eq!(actor.provenance().service_account(), None);
    let mut encoded = Vec::new();
    crate::encoding::actor(&mut encoded, &actor);
    let value = crate::encoding::cbor(&encoded, "actor").expect("CBOR");
    assert_eq!(
        crate::encoding::decode_actor(value, 2).expect("actor"),
        actor
    );
    assert!(wire::method_from(7, Some(AgentId::from_bytes([9; 16]))).is_err());
}

/// DIRECTORY-113 Amendment 2: an act a product asked for names the app it
/// came through beside the person, never in their place, under actor key 7;
/// it round-trips alone, with a draft and with a principal, and its event is
/// version 2.
#[test]
fn an_act_through_an_app_names_the_app_beside_the_person() {
    let binding = crate::LoginBinding::new("https://issuer.test", "ada").expect("binding");
    let person = super::Actor::new(binding, super::Provenance::new(super::AuthMethod::Oidc, 13));
    assert_eq!(person.through_app(), None);
    let through = person.clone().with_through_app("fixture_product");
    assert_eq!(through.through_app(), Some("fixture_product"));
    assert_eq!(through.binding(), person.binding());
    assert_eq!(through.provenance(), person.provenance());
    let drafted = through.clone().with_draft(super::DraftProvenance {
        draft: crate::OperationId::from_bytes([1; 16]),
        application: crate::OperationId::from_bytes([2; 16]),
        drafted_by: None,
    });
    let agent = AgentId::from_bytes([9; 16]);
    let signed = super::Actor::new(
        crate::LoginBinding::new("https://issuer.test", "ada").expect("binding"),
        super::Provenance::by_agent(agent, 13),
    )
    .with_through_app("fixture_product");
    for actor in [through.clone(), drafted, signed] {
        let mut encoded = Vec::new();
        crate::encoding::actor(&mut encoded, &actor);
        let value = crate::encoding::cbor(&encoded, "actor").expect("CBOR");
        assert!(matches!(&value, ciborium::Value::Map(pairs) if pairs.len() == 7));
        assert_eq!(
            crate::encoding::decode_actor(value, 2).expect("actor"),
            actor
        );
    }
    let event = crate::IdentityEvent::new(
        crate::OperationId::from_bytes([3; 16]),
        through,
        crate::IdentityId::Person(crate::PersonId::from_bytes([4; 16])),
        1,
        crate::Change::RegisterPerson {
            profile: crate::Profile::new("Bea Example").expect("profile"),
            organisation: Some(crate::OrganisationId::from_bytes([5; 16])),
        },
    )
    .expect("event");
    assert_eq!(event.version(), 2);
    let read = crate::encoding::decode_body(&crate::encoding::encode_body(&event)).expect("read");
    assert_eq!(read.actor().through_app(), Some("fixture_product"));
}
