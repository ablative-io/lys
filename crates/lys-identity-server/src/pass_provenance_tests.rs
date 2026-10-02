#![cfg(test)]

use crate::agent_pass_store::Passes;
use crate::error::ServerError;
use lys_identity::{AgentId, Provenance};
use sha2::{Digest, Sha256};
use std::error::Error;

#[test]
fn a_cached_pass_keeps_its_run_from_the_old_install() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("passes.json");
    let agent = AgentId::from_bytes([1; 16]);
    let agent_id = agent.to_string();
    let pass = "p".repeat(43);
    let digest = crate::routes::hex(&Sha256::digest(pass.as_bytes()));
    let old = serde_json::json!({"format":"lys-agent-passes/v1", "passes":{
        digest:{"agent":agent.to_string(), "launch":"launch", "session":"run"}
    }});
    std::fs::write(&path, serde_json::to_vec(&old)?)?;
    let expected = (agent, Provenance::by_pass(agent, "launch", "run")?);
    for file in [path.clone(), path] {
        let mut cached = Passes::open(file)?;
        assert!(matches!(
            cached.lookup_run(&pass),
            Err(ServerError::AgentPassRefused { reason })
                if reason == "agent pass store unavailable: agent pass table awaits authoritative startup reconciliation"
        ));
        cached.reconcile(|identity, launch, session| {
            identity == agent_id.as_str() && launch == "launch" && session == "run"
        })?;
        assert_eq!(cached.lookup_run(&pass)?, expected);
        assert_eq!(cached.lookup_run(&pass)?, expected);
    }
    Ok(())
}
