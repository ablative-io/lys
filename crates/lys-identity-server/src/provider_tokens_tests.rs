#![cfg(test)]

use std::error::Error;
use std::os::unix::fs::PermissionsExt;

use serde_json::json;

use super::{Access, FORMAT, Tokens, dropped_words};
use crate::error::ServerError;
use crate::error_provider::ProviderError;

fn access(ends: u64) -> Access {
    Access {
        session_id: "a".repeat(32),
        subject: format!("person-{}", "b".repeat(32)),
        app: "notes".to_owned(),
        profile: false,
        expires_at: ends,
    }
}

/// DIRECTORY-079 R3: a token written before the table kept the app and the
/// scope is dropped on its own at open, durably; the tokens beside it stand,
/// and the table opens.
#[test]
fn a_token_of_an_earlier_shape_is_dropped_alone_at_open() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::TempDir::new()?;
    let file = dir.path().join("provider.tokens.json");
    let old = "d".repeat(64);
    let kept = "e".repeat(64);
    std::fs::write(
        &file,
        serde_json::to_vec(&json!({
            "format": FORMAT,
            "tokens": [
                { "key": old, "access": {
                    "session_id": "a".repeat(32),
                    "subject": format!("person-{}", "b".repeat(32)),
                    "expires_at": 100,
                } },
                { "key": kept, "access": serde_json::to_value(access(100))? },
            ],
        }))?,
    )?;
    let tokens = Tokens::open(file.clone(), 10)?;
    assert!(matches!(
        tokens.get(&old, 10),
        Err(ServerError::Provider(ProviderError::TokenUnknown))
    ));
    assert_eq!(tokens.get(&kept, 10)?.app, "notes");
    // One line is said, with the count and the file, never a key or a member.
    assert_eq!(tokens.dropped(), 1);
    let said = dropped_words(tokens.dropped(), &file);
    assert!(said.contains("dropped 1 token"), "{said}");
    assert!(said.contains("provider.tokens.json"), "{said}");
    assert!(
        !said.contains(&old) && !said.contains("session_id"),
        "{said}"
    );
    drop(tokens);
    let written: serde_json::Value = serde_json::from_slice(&std::fs::read(&file)?)?;
    let keys: Vec<&str> = written["tokens"]
        .as_array()
        .ok_or("tokens")?
        .iter()
        .filter_map(|token| token["key"].as_str())
        .collect();
    assert_eq!(
        keys,
        [kept.as_str()],
        "the old token is gone from the table"
    );
    Ok(())
}

#[test]
fn an_old_key_only_install_gains_private_persistent_access_and_durable_revocation()
-> Result<(), Box<dyn Error>> {
    let dir = tempfile::TempDir::new()?;
    std::fs::write(dir.path().join("provider.key"), [7u8; 32])?;
    let file = dir.path().join("provider.tokens.json");
    let key = "c".repeat(64);
    let mut tokens = Tokens::open(file.clone(), 10)?;
    tokens.insert(key.clone(), access(100), 10)?;
    assert_eq!(
        std::fs::metadata(&file)?.permissions().mode() & 0o777,
        0o600
    );
    drop(tokens);
    let mut restarted = Tokens::open(file.clone(), 10)?;
    assert_eq!(restarted.get(&key, 10)?.session_id, "a".repeat(32));
    restarted.revoke(&key)?;
    drop(restarted);
    assert!(matches!(
        Tokens::open(file, 10)?.get(&key, 10),
        Err(ServerError::Provider(ProviderError::TokenUnknown))
    ));
    assert_eq!(std::fs::read_dir(dir.path())?.count(), 2);
    Ok(())
}

#[test]
fn a_large_table_takes_another_and_expiry_frees_space_without_waiting() -> Result<(), Box<dyn Error>>
{
    let dir = tempfile::TempDir::new()?;
    let file = dir.path().join("access.json");
    let entries: Vec<_> = (0..5000)
        .map(|index| json!({ "key": format!("{index:064x}"), "access": access(20) }))
        .collect();
    std::fs::write(
        &file,
        serde_json::to_vec(&json!({"format": FORMAT, "tokens": entries}))?,
    )?;
    let mut tokens = Tokens::open(file.clone(), 10)?;
    assert_eq!(tokens.live.len(), 5000);
    tokens.insert("e".repeat(64), access(40), 10)?;
    assert_eq!(tokens.live.len(), 5001);
    assert_eq!(Tokens::open(file.clone(), 10)?.live.len(), 5001);
    assert!(matches!(
        tokens.insert("e".repeat(64), access(40), 10),
        Err(ServerError::Provider(ProviderError::Unavailable { .. }))
    ));
    tokens.insert("f".repeat(64), access(40), 20)?;
    assert_eq!(tokens.live.len(), 2);
    assert!(matches!(
        tokens.get(&"f".repeat(64), 40),
        Err(ServerError::Provider(ProviderError::TokenUnknown))
    ));
    assert!(Tokens::open(file, 40)?.live.is_empty());
    Ok(())
}

#[test]
fn a_failed_durable_revoke_refuses_reads_instead_of_exposing_the_token()
-> Result<(), Box<dyn Error>> {
    let dir = tempfile::TempDir::new()?;
    let file = dir.path().join("access.json");
    let key = "c".repeat(64);
    let mut tokens = Tokens::open(file.clone(), 10)?;
    tokens.insert(key.clone(), access(100), 10)?;
    std::fs::remove_file(&file)?;
    std::fs::create_dir(&file)?;
    assert!(matches!(
        tokens.revoke(&key),
        Err(ServerError::Provider(ProviderError::Unavailable { .. }))
    ));
    assert!(matches!(
        tokens.get(&key, 10),
        Err(ServerError::Provider(ProviderError::Unavailable { .. }))
    ));
    assert!(matches!(
        tokens.insert("d".repeat(64), access(100), 10),
        Err(ServerError::Provider(ProviderError::Unavailable { .. }))
    ));
    assert_eq!(std::fs::read_dir(dir.path())?.count(), 1);
    Ok(())
}

#[test]
fn malformed_or_duplicate_stored_entries_are_refused_without_echoing_input()
-> Result<(), Box<dyn Error>> {
    let dir = tempfile::TempDir::new()?;
    let file = dir.path().join("access.json");
    let token = json!({"key": "c".repeat(64), "access": access(100)});
    std::fs::write(
        &file,
        serde_json::to_vec(&json!({"format": FORMAT, "tokens": [token.clone(), token]}))?,
    )?;
    assert!(matches!(
        Tokens::open(file.clone(), 10),
        Err(ServerError::Provider(ProviderError::Unavailable { .. }))
    ));
    std::fs::write(
        &file,
        br#"{"format":"lys-provider-access/v1","tokens":["sensitive-input"]}"#,
    )?;
    let Err(error) = Tokens::open(file, 10) else {
        return Err("malformed table was accepted".into());
    };
    assert_eq!(error.name(), "ProviderUnavailable");
    assert!(!error.to_string().contains("sensitive-input"));
    Ok(())
}
