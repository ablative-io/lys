//! Known crossings retain their encoded bytes and verify across store restarts.

use std::collections::BTreeMap;
use std::error::Error;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use lys_core::Ed25519Identity;
use lys_identity_server::budgets_state::{DOMAIN, Held, Leaf};
use lys_identity_server::budgets_store::BudgetStore;
use lys_log_store::{FileLeafStore, FrontierLog, Start};

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

const LEAF: &[u8] = include_bytes!("support/context_stop_v3_leaf.json");
const SNAPSHOT: &[u8] = include_bytes!("support/context_stop_v3_snapshot.json");

fn files(dir: &Path) -> TestResult<BTreeMap<PathBuf, Vec<u8>>> {
    let mut found = BTreeMap::new();
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            found.extend(files(&path)?);
        } else {
            found.insert(path.clone(), std::fs::read(path)?);
        }
    }
    Ok(found)
}

#[test]
fn known_crossings_keep_the_old_leaf_and_snapshot_bytes_after_restart() -> TestResult {
    let leaf: Leaf = serde_json::from_slice(LEAF)?;
    assert_eq!(serde_json::to_vec(&leaf)?, LEAF);
    let mut held = Held::default();
    held.hold(leaf).map_err(std::io::Error::other)?;
    assert_eq!(held.encode().map_err(std::io::Error::other)?, SNAPSHOT);

    let temporary = tempfile::tempdir()?;
    let dir = temporary.path().join("budgets");
    let key = Arc::new(Ed25519Identity::load_or_generate(
        &temporary.path().join("key"),
    )?);
    FileLeafStore::create(&dir, "lys/identity/budgets")?;
    let (mut log, _) = FrontierLog::open(FileLeafStore::open(&dir)?)?;
    log.append(LEAF)?;
    log.write_snapshot(DOMAIN, SNAPSHOT, &key)?;
    drop(log);
    let before = files(&dir)?;

    for restart in 0..2 {
        let store = BudgetStore::open(&dir, Arc::clone(&key))?;
        assert_eq!(
            store.start(),
            &Start::Resumed {
                size: 1,
                replayed: 0
            }
        );
        assert_eq!(
            store.held().encode().map_err(std::io::Error::other)?,
            SNAPSHOT
        );
        assert_eq!(store.held().crossings.crossed.len(), 1);
        assert_eq!(store.held().uses.len(), 1);
        assert_eq!(store.held().charged.len(), 1);
        drop(store);
        assert_eq!(files(&dir)?, before, "restart {restart}");
    }
    let (_, tail) = FrontierLog::open(FileLeafStore::open(&dir)?)?;
    assert_eq!(tail.leaves, vec![LEAF.to_vec()]);
    Ok(())
}
