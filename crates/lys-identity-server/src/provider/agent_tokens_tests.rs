#![cfg(test)]

use super::Passes;
use crate::error::ServerError;
use crate::error_provider::ProviderError;
use lys_identity::AgentId;

#[test]
fn an_old_install_gains_a_digest_table_and_audience_revocation_survives_reopen()
-> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let old_key = dir.path().join("provider.key");
    std::fs::write(&old_key, [6u8; 32])?;
    let path = dir.path().join("provider.agent-passes.json");
    let agent = AgentId::from_bytes([1; 16]);
    let other = AgentId::from_bytes([2; 16]);
    let mut passes = Passes::open(path.clone(), 10)?;
    passes.issue("first-fixture-pass", agent, "liminal", 30, 10)?;
    passes.issue("second-fixture-pass", agent, "other", 30, 10)?;
    passes.issue("third-fixture-pass", other, "liminal", 30, 10)?;
    let written = std::fs::read_to_string(&path)?;
    assert!(!written.contains("first-fixture-pass"));
    assert_eq!(std::fs::read(old_key)?, [6u8; 32]);
    drop(passes);
    let mut passes = Passes::open(path.clone(), 10)?;
    assert_eq!(passes.revoke(agent, "liminal")?, 1);
    drop(passes);
    let mut passes = Passes::open(path, 10)?;
    assert!(matches!(
        passes.admits("first-fixture-pass", agent, "liminal", 10),
        Err(ServerError::Provider(ProviderError::TokenUnknown))
    ));
    passes.admits("second-fixture-pass", agent, "other", 10)?;
    passes.admits("third-fixture-pass", other, "liminal", 10)?;
    assert_eq!(passes.revoke(agent, "liminal")?, 0);
    assert!(
        passes
            .admits("second-fixture-pass", agent, "other", 30)
            .is_err()
    );
    Ok(())
}

#[test]
fn a_failed_audience_revoke_refuses_cached_admission() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("passes.json");
    let agent = AgentId::from_bytes([1; 16]);
    let mut passes = Passes::open(path.clone(), 10)?;
    passes.issue("fixture-pass", agent, "liminal", 30, 10)?;
    std::fs::remove_file(&path)?;
    std::fs::create_dir(&path)?;
    assert!(matches!(
        passes.revoke(agent, "liminal"),
        Err(ServerError::Provider(ProviderError::Unavailable { .. }))
    ));
    assert!(matches!(
        passes.admits("fixture-pass", agent, "liminal", 10),
        Err(ServerError::Provider(ProviderError::Unavailable { .. }))
    ));
    Ok(())
}
