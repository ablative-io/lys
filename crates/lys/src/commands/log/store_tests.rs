#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;

fn init_store(dir: &Path) -> LogStore {
    LogStore::init(dir, "example.com/lys/store-test").unwrap();
    LogStore::open(dir).unwrap()
}

#[test]
fn init_creates_layout_and_open_round_trips() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("log");
    let store = init_store(&dir);
    assert_eq!(store.origin(), "example.com/lys/store-test");
    assert_eq!(store.tree().len(), 0);
    assert!(dir.join("log.json").is_file());
    assert!(dir.join("state.json").is_file());
    assert!(dir.join("leaves").is_dir());
}

#[test]
fn init_refuses_to_reinitialize() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("log");
    init_store(&dir);
    let err = LogStore::init(&dir, "example.com/other").unwrap_err();
    assert!(matches!(err, CliError::LogDirInvalid { .. }), "{err}");
    assert!(err.to_string().contains("already initialized"), "{err}");
}

#[test]
fn init_rejects_invalid_origin() {
    let tmp = tempfile::tempdir().unwrap();
    for bad in ["", "has space", "has+plus"] {
        let dir = tmp.path().join(format!("log-{}", bad.len()));
        let err = LogStore::init(&dir, bad).unwrap_err();
        assert!(matches!(err, CliError::Trust(_)), "{bad:?}: {err}");
    }
}

#[test]
fn open_missing_dir_is_log_dir_missing_with_remedy() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("nope");
    let err = LogStore::open(&dir).unwrap_err();
    assert!(matches!(err, CliError::LogDirMissing { .. }), "{err}");
    assert!(err.to_string().contains("lys log init"), "{err}");
}

#[test]
fn append_reopen_reproduces_root_and_golden_leaf_hash() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("log");
    let mut store = init_store(&dir);
    let (index0, hash0) = store.append(b"leaf-0").unwrap();
    let (index1, _hash1) = store.append(b"leaf-1").unwrap();
    assert_eq!((index0, index1), (0, 1));
    // Golden vector: SHA-256(0x00 || "leaf-0").
    assert_eq!(
        crate::commands::hex::hex_lower(&hash0),
        "305df59f9590c3c9ac63d2b2743c388e3792449078cebf7fb3dbe6471643b2b7"
    );
    assert_eq!(hash0, raw_leaf_hash(b"leaf-0"));
    let root_before = store.tree().root();
    let reopened = LogStore::open(&dir).unwrap();
    assert_eq!(reopened.tree().root(), root_before);
    assert_eq!(reopened.leaf_bytes(0), Some(b"leaf-0".as_slice()));
    assert_eq!(reopened.leaf_bytes(1), Some(b"leaf-1".as_slice()));
    assert_eq!(reopened.leaf_bytes(2), None);
}

#[test]
fn leaf_files_hold_raw_bytes_verbatim() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("log");
    let mut store = init_store(&dir);
    store.append(b"raw \x00 bytes").unwrap();
    let on_disk = std::fs::read(dir.join("leaves").join("0".repeat(20))).unwrap();
    assert_eq!(on_disk, b"raw \x00 bytes");
}

#[test]
fn empty_leaf_is_legal() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("log");
    let mut store = init_store(&dir);
    let (_index, hash) = store.append(b"").unwrap();
    assert_eq!(hash, raw_leaf_hash(b""));
    assert_eq!(LogStore::open(&dir).unwrap().tree().len(), 1);
}

#[test]
fn tampered_leaf_byte_is_detected() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("log");
    let mut store = init_store(&dir);
    store.append(b"leaf-0").unwrap();
    std::fs::write(dir.join("leaves").join("0".repeat(20)), b"leaf-X").unwrap();
    let err = LogStore::open(&dir).unwrap_err();
    assert!(matches!(err, CliError::LogDirInvalid { .. }), "{err}");
}

#[test]
fn leaf_gap_is_detected() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("log");
    let mut store = init_store(&dir);
    store.append(b"leaf-0").unwrap();
    store.append(b"leaf-1").unwrap();
    store.append(b"leaf-2").unwrap();
    std::fs::remove_file(dir.join("leaves").join(format!("{:020}", 1))).unwrap();
    let err = LogStore::open(&dir).unwrap_err();
    assert!(err.to_string().contains("not contiguous"), "{err}");
}

#[test]
fn unexpected_leaves_entry_is_detected_but_dotfiles_are_ignored() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("log");
    let mut store = init_store(&dir);
    store.append(b"leaf-0").unwrap();
    std::fs::write(dir.join("leaves").join(".DS_Store"), b"junk").unwrap();
    assert!(LogStore::open(&dir).is_ok(), "dotfiles must be ignored");
    std::fs::write(dir.join("leaves").join("stray.txt"), b"junk").unwrap();
    let err = LogStore::open(&dir).unwrap_err();
    assert!(err.to_string().contains("unexpected entry"), "{err}");
}

#[test]
fn tampered_state_root_is_detected() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("log");
    let mut store = init_store(&dir);
    store.append(b"leaf-0").unwrap();
    let state = std::fs::read_to_string(dir.join("state.json")).unwrap();
    let tampered = state.replacen("\"tree_size\": 1", "\"tree_size\": 2", 1);
    assert_ne!(state, tampered);
    std::fs::write(dir.join("state.json"), tampered).unwrap();
    let err = LogStore::open(&dir).unwrap_err();
    assert!(matches!(err, CliError::LogDirInvalid { .. }), "{err}");
}

#[test]
fn malformed_state_json_is_detected() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("log");
    init_store(&dir);
    std::fs::write(dir.join("state.json"), "{\"tree_size\": 0}").unwrap();
    let err = LogStore::open(&dir).unwrap_err();
    assert!(err.to_string().contains("state.json is malformed"), "{err}");
}

#[test]
fn crash_recovery_repairs_one_extra_contiguous_leaf() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("log");
    let mut store = init_store(&dir);
    store.append(b"leaf-0").unwrap();
    let state_after_one = std::fs::read(dir.join("state.json")).unwrap();
    store.append(b"leaf-1").unwrap();
    // Simulate a crash between the leaf write and the state write.
    std::fs::write(dir.join("state.json"), &state_after_one).unwrap();
    let recovered = LogStore::open(&dir).unwrap();
    assert_eq!(recovered.tree().len(), 2);
    // The state file was repaired on disk.
    let reread = LogStore::open(&dir).unwrap();
    assert_eq!(reread.tree().len(), 2);
}

#[test]
fn crash_recovery_does_not_mask_a_tampered_prefix() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("log");
    let mut store = init_store(&dir);
    store.append(b"leaf-0").unwrap();
    let state_after_one = std::fs::read(dir.join("state.json")).unwrap();
    store.append(b"leaf-1").unwrap();
    // Stale state AND a tampered pinned prefix: must NOT recover.
    std::fs::write(dir.join("leaves").join("0".repeat(20)), b"leaf-X").unwrap();
    std::fs::write(dir.join("state.json"), &state_after_one).unwrap();
    let err = LogStore::open(&dir).unwrap_err();
    assert!(matches!(err, CliError::LogDirInvalid { .. }), "{err}");
}

#[test]
fn prefix_tree_matches_directly_built_tree_and_bounds_checked() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("log");
    let mut store = init_store(&dir);
    store.append(b"leaf-0").unwrap();
    store.append(b"leaf-1").unwrap();
    store.append(b"leaf-2").unwrap();
    let prefix = store.prefix_tree(2).unwrap();
    let direct = AppendOnlyTree::<RawLeaf>::reconstruct_from_raw_leaves([b"leaf-0", b"leaf-1"]);
    assert_eq!(prefix.root(), direct.root());
    assert!(store.prefix_tree(4).is_err());
}
