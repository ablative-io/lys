#![cfg(test)]

//! A computer that joined another Lys is never silently turned into a
//! server of its own, and a service key the receipt counts is never minted
//! anew (DIRECTORY-107 R3).

use std::collections::BTreeMap;
use std::error::Error;
use std::path::Path;
use std::sync::Arc;

use lys_core::Ed25519Identity;
use lys_identity::stores_receipt::{Class, ComparedBy, Entry, Receipt, VERSION, write_receipt};

use super::layout::Layout;
use super::{service_key, start_runner};

type TestResult = Result<(), Box<dyn Error>>;

/// No runner program: a start that gets past its guards fails to spawn.
const NO_PROGRAM: &str = "/nonexistent/lys-for-a-guard-test";

fn key() -> Arc<Ed25519Identity> {
    Arc::new(Ed25519Identity::from_seed(&zeroize::Zeroizing::new(
        [5; 32],
    )))
}

#[test]
fn an_install_over_a_saved_join_is_refused_and_leaves_the_server_key_as_it_was() -> TestResult {
    let dir = tempfile::tempdir()?;
    let layout = Layout::at(dir.path().to_path_buf());
    std::fs::create_dir_all(layout.run_dir())?;
    let saved = layout.run_dir().join("runner-join.json");
    std::fs::write(
        &saved,
        br#"{"server":"https://lys.example.test","machine":"ward-desk","server_ca":null}"#,
    )?;
    let public = layout.run_dir().join("runner-server.pub");
    std::fs::write(&public, b"the joined server's key")?;
    let refused = start_runner(&layout, &key(), Path::new(NO_PROGRAM), &mut |_| {})
        .err()
        .ok_or("an install over a saved join started a runner")?;
    assert_eq!(refused.kind().name(), "runner_join_present");
    let words = refused.to_string();
    assert!(words.contains(&saved.display().to_string()), "{words}");
    assert!(words.contains("https://lys.example.test"), "{words}");
    assert_eq!(std::fs::read(&public)?, b"the joined server's key");
    Ok(())
}

#[test]
fn an_install_with_no_saved_join_writes_the_server_key_as_today() -> TestResult {
    let dir = tempfile::tempdir()?;
    let layout = Layout::at(dir.path().to_path_buf());
    let key = key();
    let failed = start_runner(&layout, &key, Path::new(NO_PROGRAM), &mut |_| {});
    assert!(failed.is_err(), "no runner program exists to start");
    assert_eq!(
        std::fs::read_to_string(layout.run_dir().join("runner-server.pub"))?,
        lys_runner::protocol::hex(&key.public_key_bytes())
    );
    Ok(())
}

#[test]
fn a_service_key_the_receipt_counts_is_never_minted_anew() -> TestResult {
    let dir = tempfile::tempdir()?;
    let layout = Layout::at(dir.path().to_path_buf());
    let mut stores = BTreeMap::new();
    stores.insert(
        "service-key".to_owned(),
        Entry {
            class: Class::Whole,
            compared_by: ComparedBy::Digest,
            path: layout.service_key(),
            head: 0,
            digest: "the public half's digest".to_owned(),
            read_at: lys_identity::stores_receipt::now(),
            identity: None,
        },
    );
    let receipt = Receipt {
        version: VERSION,
        written_at: lys_identity::stores_receipt::now(),
        stores,
    };
    write_receipt(&layout.install_dir().join("stores.json"), &receipt)?;
    let refused = service_key(&layout)
        .err()
        .ok_or("a counted service key was minted anew")?;
    assert_eq!(refused.kind().name(), "store_missing");
    assert!(
        refused.to_string().contains("keys/service.key"),
        "{refused}"
    );
    assert!(!layout.service_key().exists(), "no key was generated");
    Ok(())
}

#[test]
fn a_saved_join_to_this_install_itself_is_not_refused() -> TestResult {
    let dir = tempfile::tempdir()?;
    let layout = Layout::at(dir.path().to_path_buf());
    let key = key();
    let own = lys_runner::protocol::hex(&key.public_key_bytes());
    std::fs::create_dir_all(layout.run_dir())?;
    std::fs::write(
        layout.run_dir().join("runner-join.json"),
        br#"{"server":"https://127.0.0.1:8490","machine":"this-mac","server_ca":null}"#,
    )?;
    std::fs::write(layout.run_dir().join("runner-server.pub"), &own)?;
    let failed = start_runner(&layout, &key, Path::new(NO_PROGRAM), &mut |_| {})
        .err()
        .ok_or("no runner program exists to start")?;
    assert_ne!(failed.kind().name(), "runner_join_present", "{failed}");
    assert_eq!(
        std::fs::read_to_string(layout.run_dir().join("runner-server.pub"))?,
        own
    );
    Ok(())
}
