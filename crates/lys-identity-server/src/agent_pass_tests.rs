#![cfg(test)]
use crate::agent_pass_recovery::holds_pass;
use crate::agent_pass_store::Passes;
use crate::error::ServerError;
use crate::runtime_state::{Report, Reported, Tracked};
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
    let mut reopened = Passes::open(file)?;
    assert!(reopened.lookup(&pass).is_err());
    reopened.reconcile(|_, _, _| true)?;
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
        Passes::open(file.clone())?.lookup(&pass),
        Err(ServerError::AgentPassRefused { .. })
    ));
    let mut reopened = Passes::open(file.clone())?;
    reopened.reconcile(|_, _, _| false)?;
    assert!(reopened.lookup(&pass).is_err());
    assert!(Passes::open(file)?.lookup(&pass).is_err());
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

#[test]
fn agent_pass_failed_end_does_not_revive_on_reopen() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let file = dir.path().join("passes.json");
    let mut passes = Passes::open(file.clone())?;
    let pass = passes.issue(AgentId::from_bytes([1; 16]), "launch-one", "session-one")?;
    let earlier = std::fs::read(&file)?;
    std::fs::remove_file(&file)?;
    std::fs::create_dir(&file)?;
    assert!(passes.end_session("session-one").is_err());
    std::fs::remove_dir(&file)?;
    std::fs::write(&file, earlier)?;
    assert!(matches!(
        Passes::open(file.clone())?.lookup(&pass),
        Err(ServerError::AgentPassRefused { .. })
    ));
    let mut reopened = Passes::open(file.clone())?;
    reopened.reconcile(|_, _, _| false)?;
    assert!(reopened.lookup(&pass).is_err());
    assert!(Passes::open(file)?.lookup(&pass).is_err());
    Ok(())
}

#[test]
fn agent_pass_full_capacity_refuses_before_dropping_an_existing_binding()
-> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let file = dir.path().join("passes.json");
    let agent = AgentId::from_bytes([1; 16]);
    let pass = "a".repeat(43);
    let key = format!("{:x}", Sha256::digest(pass.as_bytes()));
    let mut entries = serde_json::Map::new();
    for index in 0..1024 {
        let digest = if index == 0 {
            key.clone()
        } else {
            format!("{:x}", Sha256::digest(index.to_string().as_bytes()))
        };
        entries.insert(digest, serde_json::json!({"agent":agent.to_string(), "launch":format!("launch-{index}"), "session":format!("session-{index}")}));
    }
    std::fs::write(
        &file,
        serde_json::to_vec(&serde_json::json!({"format":"lys-agent-passes/v1", "passes":entries}))?,
    )?;
    let before = std::fs::read(&file)?;
    let mut passes = Passes::open(file.clone())?;
    passes.reconcile(|_, _, _| true)?;
    assert!(
        passes
            .issue(agent, "launch-0", "session-replacement")
            .is_err()
    );
    assert_eq!(passes.lookup(&pass)?, agent);
    assert_eq!(std::fs::read(file)?, before);
    Ok(())
}

#[test]
fn agent_pass_second_issue_for_a_held_session_is_refused_and_the_first_kept()
-> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let mut passes = Passes::open(dir.path().join("passes.json"))?;
    let agent = AgentId::from_bytes([1; 16]);
    let first = passes
        .issue_unless_present(agent, "launch-one", "session-one")?
        .ok_or("the first issue was refused")?;
    let second = passes.issue_unless_present(agent, "launch-one", "session-one")?;
    assert!(
        second.is_none(),
        "a second issue for a held session was not refused"
    );
    assert_eq!(passes.lookup(&first)?, agent);
    Ok(())
}

fn tracked(agent: &str, states: &[Reported]) -> Tracked {
    Tracked {
        session: "session-one".to_owned(),
        agent: Some(agent.to_owned()),
        machine: "machine-one".to_owned(),
        reports: states
            .iter()
            .enumerate()
            .map(|(at, state)| Report {
                operation: format!("op-{at}"),
                session: "session-one".to_owned(),
                agent: Some(agent.to_owned()),
                machine: "machine-one".to_owned(),
                state: *state,
                what: String::new(),
                confirmation: String::new(),
                reported_by: "tester".to_owned(),
                at: u64::try_from(at).unwrap_or(u64::MAX),
                launch: None,
            })
            .collect(),
    }
}

#[test]
fn agent_pass_recovery_drops_a_pass_whose_session_never_ran() {
    let agent = AgentId::from_bytes([1; 16]).to_string();
    assert!(!holds_pass(&tracked(&agent, &[Reported::Starting]), &agent));
    assert!(holds_pass(
        &tracked(&agent, &[Reported::Starting, Reported::Running]),
        &agent
    ));
    assert!(!holds_pass(
        &tracked(&agent, &[Reported::Running, Reported::StopAsked]),
        &agent
    ));
    assert!(!holds_pass(
        &tracked("someone-else", &[Reported::Running]),
        &agent
    ));
}
