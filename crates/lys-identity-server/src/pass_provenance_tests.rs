#![cfg(test)]

use crate::agent_pass_store::Passes;
use lys_identity::{AgentId, Provenance};
use sha2::{Digest, Sha256};
use std::error::Error;

#[test]
fn a_cached_pass_keeps_its_run_from_the_old_install() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("passes.json");
    let agent = AgentId::from_bytes([1; 16]);
    let pass = "p".repeat(43);
    let digest = crate::routes::hex(&Sha256::digest(pass.as_bytes()));
    let old = serde_json::json!({"format":"lys-agent-passes/v1", "passes":{
        digest:{"agent":agent.to_string(), "launch":"launch", "session":"run"}
    }});
    std::fs::write(&path, serde_json::to_vec(&old)?)?;
    let expected = (agent, Provenance::by_pass(agent, "launch", "run")?);
    let cached = Passes::open(path.clone())?;
    assert_eq!(cached.lookup_run(&pass)?, expected);
    assert_eq!(cached.lookup_run(&pass)?, expected);
    assert_eq!(Passes::open(path)?.lookup_run(&pass)?, expected);
    Ok(())
}
