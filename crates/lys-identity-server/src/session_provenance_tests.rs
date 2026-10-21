#![cfg(test)]

use crate::session::SessionEntry;
use lys_identity::{Actor, AgentId, AuthMethod, LoginBinding, Provenance};
use std::collections::HashMap;
use std::error::Error;

#[test]
fn historical_session_actor_bytes_keep_their_time_and_shape() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("sessions.json");
    for (method, agent) in [
        ("oidc", serde_json::Value::Null),
        (
            "agent_signature",
            serde_json::json!(AgentId::from_bytes([1; 16]).to_string()),
        ),
        (
            "service_account_bearer",
            serde_json::json!(lys_identity::ServiceAccountId::from_bytes([1; 16]).to_string()),
        ),
        (
            "agent_pass",
            serde_json::json!(AgentId::from_bytes([1; 16]).to_string()),
        ),
    ] {
        let old = serde_json::json!({"format":super::FORMAT,"sessions":[{
            "key":"digest", "id":"id", "started_at":1, "ends_at":100,
            "actor":{"issuer":"https://issuer.test", "subject":"subject", "method":method,
                "agent":agent, "authenticated_at":7}
        }]});
        std::fs::write(&path, serde_json::to_vec(&old)?)?;
        let live = super::load(&path, 2)?;
        assert_eq!(
            live["digest"].actor.provenance().authenticated_at(),
            Some(7)
        );
        assert_eq!(live["digest"].actor.provenance().run(), None);
        super::save(&path, &live)?;
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&std::fs::read(&path)?)?,
            old
        );
    }
    Ok(())
}

#[test]
fn pass_session_actor_round_trips_without_a_time() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("sessions.json");
    let agent = AgentId::from_bytes([1; 16]);
    let actor = Actor::new(
        LoginBinding::new("https://issuer.test", "agent")?,
        Provenance::by_pass(agent, "launch", "run")?,
    );
    let live = HashMap::from([(
        "digest".to_owned(),
        SessionEntry {
            id: "id".to_owned(),
            actor: actor.clone(),
            started_at: 1,
            ends_at: 100,
        },
    )]);
    super::save(&path, &live)?;
    let stored: serde_json::Value = serde_json::from_slice(&std::fs::read(&path)?)?;
    assert!(
        stored["sessions"][0]["actor"]
            .get("authenticated_at")
            .is_none()
    );
    let reopened = super::load(&path, 2)?;
    assert_eq!(reopened["digest"].actor, actor);
    assert_eq!(
        reopened["digest"].actor.provenance().method(),
        AuthMethod::AgentPass(agent)
    );
    Ok(())
}

/// A session begun by a handed link (DIRECTORY-112 R2) is stored as
/// `handed_link` and reads back as the person signed in.
#[test]
fn handed_link_session_actor_reads_back_as_the_person_signed_in() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("sessions.json");
    let stored = serde_json::json!({"format":super::FORMAT,"sessions":[{
        "key":"digest", "id":"id", "started_at":1, "ends_at":100,
        "actor":{"issuer":"https://issuer.test", "subject":"subject", "method":"handed_link",
            "agent":null, "authenticated_at":7}
    }]});
    std::fs::write(&path, serde_json::to_vec(&stored)?)?;
    let live = super::load(&path, 2)?;
    let method = live["digest"].actor.provenance().method();
    assert!(method.is_person_signed_in(), "{method:?}");
    assert_eq!(live["digest"].actor.provenance().agent(), None);
    assert_eq!(
        live["digest"].actor.provenance().authenticated_at(),
        Some(7)
    );
    super::save(&path, &live)?;
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&std::fs::read(&path)?)?,
        stored
    );
    Ok(())
}

/// A session begun by a proving link (DIRECTORY-111 R2) reads back from
/// the store as the person signed in, and a founding actor never holds a
/// session.
#[test]
fn address_proof_session_actor_round_trips_and_founding_is_refused() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("sessions.json");
    let actor = Actor::new(
        LoginBinding::new("https://issuer.test", "subject")?,
        Provenance::new(AuthMethod::AddressProof, 7),
    );
    let live = HashMap::from([(
        "digest".to_owned(),
        SessionEntry {
            id: "id".to_owned(),
            actor: actor.clone(),
            started_at: 1,
            ends_at: 100,
        },
    )]);
    super::save(&path, &live)?;
    let stored: serde_json::Value = serde_json::from_slice(&std::fs::read(&path)?)?;
    assert_eq!(stored["sessions"][0]["actor"]["method"], "address_proof");
    let reopened = super::load(&path, 2)?;
    assert_eq!(reopened["digest"].actor, actor);
    assert!(
        reopened["digest"]
            .actor
            .provenance()
            .method()
            .is_person_signed_in()
    );
    let founding = Actor::new(
        LoginBinding::new("https://issuer.test", "founding:op-1")?,
        Provenance::new(AuthMethod::Founding, 7),
    );
    let refused = HashMap::from([(
        "founding".to_owned(),
        SessionEntry {
            id: "id".to_owned(),
            actor: founding,
            started_at: 1,
            ends_at: 100,
        },
    )]);
    assert!(super::save(&path, &refused).is_err());
    Ok(())
}
