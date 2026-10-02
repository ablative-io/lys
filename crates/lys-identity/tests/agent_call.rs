//! A change an agent made through MCP is a leaf of the directory log with a
//! receipt, kept only against the agent that made it, and read back the same
//! when the log is opened again.

#![cfg(unix)]

use std::error::Error;
use std::os::unix::fs::PermissionsExt;

use lys_core::Ed25519Identity;
use lys_identity::agent_call::AgentCall;
use lys_identity::{
    Actor, AuthMethod, Directory, IdentityId, LoginBinding, OperationId, Profile, Provenance,
    Transition,
};
use lys_log_store::FileLeafStore;

type TestResult = Result<(), Box<dyn Error>>;

const ISSUER: &str = "https://issuer.test";
const DIGEST: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

#[test]
fn an_agent_call_is_kept_with_a_receipt_and_read_back_on_reopen() -> TestResult {
    let dir = tempfile::TempDir::new()?;
    let log = dir.path().join("log");
    FileLeafStore::create(&log, "example.test/lys/directory")?;
    let key = dir.path().join("service.key");
    std::fs::write(&key, [11_u8; 32])?;
    std::fs::set_permissions(&key, std::fs::Permissions::from_mode(0o600))?;
    let reopened = log.clone();
    let mut directory = Directory::open(
        Box::new(move || FileLeafStore::open(&reopened)),
        Ed25519Identity::load(&key)?,
    )?;
    let binding = LoginBinding::new(ISSUER, "administrator")?;
    let administrator = Actor::new(
        binding.clone(),
        Provenance::new(AuthMethod::Oidc, 1_790_000_000),
    );
    let (person, _) = directory.setup_person(
        administrator.clone(),
        OperationId::from_bytes([1; 16]),
        Profile::new("Ada")?,
        10,
    )?;
    let (agent, _) = directory.register_agent(
        administrator.clone(),
        OperationId::from_bytes([2; 16]),
        person,
        Profile::new("Caller")?,
        11,
    )?;
    let (other, _) = directory.register_agent(
        administrator.clone(),
        OperationId::from_bytes([3; 16]),
        person,
        Profile::new("Other")?,
        12,
    )?;
    directory.transition(
        administrator,
        OperationId::from_bytes([4; 16]),
        IdentityId::Agent(agent),
        Transition::Activate,
        "",
        13,
    )?;
    let actor = Actor::new(binding, Provenance::by_agent(agent, 1_790_000_100));
    let call = AgentCall::new("POST", "/agents/x/goals", DIGEST, 200, "signed header")?;
    assert!(
        directory
            .record_agent_call(actor.clone(), other, call.clone(), 14)
            .is_err(),
        "a call is kept only against the agent that made it"
    );
    let before = directory.projection()?.record(IdentityId::Agent(agent)).ok_or("no agent")?.events().len();
    directory.record_agent_call(actor, agent, call, 14)?;
    let after = directory.projection()?.record(IdentityId::Agent(agent)).ok_or("no agent")?.events().len();
    assert_eq!(after, before + 1);
    drop(directory);
    let mut again = Directory::open(
        Box::new(move || FileLeafStore::open(&log)),
        Ed25519Identity::load(&key)?,
    )?;
    let replayed = again.projection()?.record(IdentityId::Agent(agent)).ok_or("no agent")?.events().len();
    assert_eq!(replayed, after);
    Ok(())
}
