#![cfg(test)]
use crate::agent_pass_store::Passes;
use crate::error::ServerError;
use lys_identity::AgentId;
use sha2::{Digest, Sha256};
use std::error::Error;

#[test]
fn agent_pass_survives_service_restart_as_a_digest_only() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let file = dir.path().join("passes.json");
    let agent = AgentId::from_bytes([1; 16]);
    let mut passes = Passes::open(file.clone())?;
    let pass = passes.issue(agent, "launch-one", "session-one")?;
    assert_eq!(passes.lookup(&pass)?, agent);
    assert!(!std::fs::read_to_string(&file)?.contains(pass.as_str()));
    let reopened = Passes::open(file)?;
    assert_eq!(reopened.lookup(&pass)?, agent);
    Ok(())
}

#[test]
fn agent_pass_stop_end_and_withdraw_each_refuse_the_old_pass() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let file = dir.path().join("passes.json");
    let agent = AgentId::from_bytes([1; 16]);
    let mut passes = Passes::open(file.clone())?;
    let pass = passes.issue(agent, "launch-one", "session-one")?;
    passes.end_agent(agent)?;
    assert!(matches!(
        passes.lookup(&pass),
        Err(ServerError::AgentPassRefused { .. })
    ));
    let pass = passes.issue(agent, "launch-two", "session-two")?;
    passes.end_session("session-two")?;
    assert!(matches!(
        passes.lookup(&pass),
        Err(ServerError::AgentPassRefused { .. })
    ));
    let pass = passes.issue(agent, "launch-three", "session-three")?;
    passes.end_launch("launch-three")?;
    assert!(matches!(
        passes.lookup(&pass),
        Err(ServerError::AgentPassRefused { .. })
    ));
    assert!(matches!(
        Passes::open(file)?.lookup(&pass),
        Err(ServerError::AgentPassRefused { .. })
    ));
    Ok(())
}

#[test]
fn agent_pass_replacement_ends_the_prior_run_and_keeps_other_agents() -> Result<(), Box<dyn Error>>
{
    let dir = tempfile::tempdir()?;
    let mut passes = Passes::open(dir.path().join("passes.json"))?;
    let agent = AgentId::from_bytes([1; 16]);
    let other = AgentId::from_bytes([2; 16]);
    let first = passes.issue(agent, "launch-one", "session-one")?;
    let independent = passes.issue(other, "launch-other", "session-other")?;
    let second = passes.issue(agent, "launch-one", "session-two")?;
    assert_ne!(
        Sha256::digest(first.as_bytes()),
        Sha256::digest(second.as_bytes())
    );
    assert!(matches!(
        passes.lookup(&first),
        Err(ServerError::AgentPassRefused { .. })
    ));
    assert_eq!(passes.lookup(&second)?, agent);
    assert_eq!(passes.lookup(&independent)?, other);
    Ok(())
}

#[test]
fn agent_pass_failed_end_refuses_cached_admission() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let file = dir.path().join("passes.json");
    let mut passes = Passes::open(file.clone())?;
    let agent = AgentId::from_bytes([1; 16]);
    let pass = passes.issue(agent, "launch-one", "session-one")?;
    std::fs::remove_file(&file)?;
    std::fs::create_dir(&file)?;
    assert!(passes.end_session("session-one").is_err());
    assert!(matches!(
        passes.lookup(&pass),
        Err(ServerError::AgentPassRefused { .. })
    ));
    Ok(())
}
