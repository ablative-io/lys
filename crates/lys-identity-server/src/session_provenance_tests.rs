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
