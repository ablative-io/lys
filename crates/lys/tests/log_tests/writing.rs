//! Writing a log: `lys log init`, `checkpoint` and `append`, an interrupted
//! append recovered, and a corrupted directory refused.

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use lys_core::merkle::raw_leaf_hash;

use super::{
    EMPTY_ROOT_HEX, GOLDEN_SEED, ProvenLog, assert_success, build_proven_log, field, hex_lower,
    load_json, path_str, run_lys, stderr_of, stdout_of,
};

// ------------------------------------------------------------------ log init

#[test]
fn log_init_pins_origin_and_prints_empty_root() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("log");
    let output = run_lys(&[
        "log",
        "init",
        "--dir",
        path_str(&dir),
        "--origin",
        "example.com/lys/init-test",
    ]);
    assert_success(&output);
    let stdout = stdout_of(&output);
    assert_eq!(field(&stdout, "origin:"), "example.com/lys/init-test");
    assert_eq!(field(&stdout, "tree size:"), "0");
    assert_eq!(field(&stdout, "root hash (sha256):"), EMPTY_ROOT_HEX);

    // Re-init is refused: the origin is pinned exactly once.
    let again = run_lys(&[
        "log",
        "init",
        "--dir",
        path_str(&dir),
        "--origin",
        "example.com/lys/other",
    ]);
    assert_eq!(again.status.code(), Some(1));
    let stderr = stderr_of(&again);
    assert!(stderr.contains("log directory invalid"), "{stderr}");
    assert!(stderr.contains("already initialized"), "{stderr}");
}

#[test]
fn log_init_rejects_invalid_origins() {
    let tmp = tempfile::tempdir().unwrap();
    for (bad, tag) in [("has space", "space"), ("has+plus", "plus"), ("", "empty")] {
        let dir = tmp.path().join(format!("log-{tag}"));
        let output = run_lys(&["log", "init", "--dir", path_str(&dir), "--origin", bad]);
        assert_eq!(output.status.code(), Some(1), "origin {bad:?} was accepted");
        assert!(
            !dir.join("log.json").exists(),
            "invalid origin {bad:?} must not initialize a log"
        );
    }
}

// ------------------------------------------------------------ log checkpoint

#[test]
fn log_checkpoint_with_missing_key_fails_and_writes_nothing() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("log");
    let init = run_lys(&[
        "log",
        "init",
        "--dir",
        path_str(&dir),
        "--origin",
        "example.com/lys/ckpt-fail",
    ]);
    assert_success(&init);

    let missing_key = tmp.path().join("no-such.key");
    let out = tmp.path().join("checkpoint.txt");
    let output = run_lys(&[
        "log",
        "checkpoint",
        "--dir",
        path_str(&dir),
        "--key",
        path_str(&missing_key),
        "--out",
        path_str(&out),
    ]);
    assert_eq!(output.status.code(), Some(1));
    let stderr = stderr_of(&output);
    assert!(stderr.contains("identity key file not found"), "{stderr}");
    assert!(stderr.contains("lys key generate"), "{stderr}");
    assert!(!out.exists(), "no checkpoint on failure");
    assert!(
        !missing_key.exists(),
        "checkpoint must never mint key material"
    );
}

#[test]
fn log_checkpoint_on_uninitialized_dir_fails_and_writes_nothing() {
    let tmp = tempfile::tempdir().unwrap();
    let key = tmp.path().join("signer.key");
    let generate = run_lys(&["key", "generate", "--out", path_str(&key)]);
    assert_success(&generate);

    let out = tmp.path().join("checkpoint.txt");
    let output = run_lys(&[
        "log",
        "checkpoint",
        "--dir",
        path_str(&tmp.path().join("never-initialized")),
        "--key",
        path_str(&key),
        "--out",
        path_str(&out),
    ]);
    assert_eq!(output.status.code(), Some(1));
    let stderr = stderr_of(&output);
    assert!(stderr.contains("log init"), "remedy missing: {stderr}");
    assert!(!out.exists(), "no checkpoint on failure");
}

// ------------------------------------------------------------- append behavior

#[test]
fn log_append_to_uninitialized_dir_names_remedy() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("no-log");
    let leaf = tmp.path().join("leaf.bin");
    std::fs::write(&leaf, b"data").unwrap();
    let output = run_lys(&[
        "log",
        "append",
        "--dir",
        path_str(&dir),
        "--leaf",
        path_str(&leaf),
    ]);
    assert_eq!(output.status.code(), Some(1));
    let stderr = stderr_of(&output);
    assert!(stderr.contains("log directory not initialized"), "{stderr}");
    assert!(stderr.contains("lys log init"), "{stderr}");
}

#[test]
fn empty_leaf_appends_proves_and_verifies() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("log");
    let key = tmp.path().join("operator.key");
    std::fs::write(&key, GOLDEN_SEED).unwrap();
    assert_success(&run_lys(&[
        "log",
        "init",
        "--dir",
        path_str(&dir),
        "--origin",
        "example.com/lys/empty-leaf",
    ]));
    let leaf = tmp.path().join("empty.bin");
    std::fs::write(&leaf, b"").unwrap();
    let append = run_lys(&[
        "log",
        "append",
        "--dir",
        path_str(&dir),
        "--leaf",
        path_str(&leaf),
    ]);
    assert_success(&append);
    assert_eq!(
        field(&stdout_of(&append), "leaf hash (sha256, rfc6962):"),
        hex_lower(&raw_leaf_hash(b"")),
        "empty leaf hash must be SHA-256 of the single 0x00 prefix byte"
    );
    let artifact = tmp.path().join("inclusion.json");
    let prove = run_lys(&[
        "log",
        "prove",
        "inclusion",
        "--dir",
        path_str(&dir),
        "--key",
        path_str(&key),
        "--leaf-index",
        "0",
        "--out",
        path_str(&artifact),
    ]);
    assert_success(&prove);
    // A size-1 tree has an EMPTY inclusion path — legal and verifiable.
    let value = load_json(&artifact);
    assert_eq!(value["hashes"].as_array().unwrap().len(), 0);
    let checkpoint = tmp.path().join("cp.note");
    let checkpoint_out = run_lys(&[
        "log",
        "checkpoint",
        "--dir",
        path_str(&dir),
        "--key",
        path_str(&key),
        "--out",
        path_str(&checkpoint),
    ]);
    assert_success(&checkpoint_out);
    let verifier = field(&stdout_of(&checkpoint_out), "verifier key (signed-note):");
    let verify = run_lys(&[
        "log",
        "verify",
        "inclusion",
        "--artifact",
        path_str(&artifact),
        "--leaf",
        path_str(&leaf),
        "--verifier-key",
        &verifier,
    ]);
    assert_success(&verify);
}

// ------------------------------------------------- crash recovery / corruption

#[test]
fn interrupted_append_is_recovered_with_a_notice() {
    let log = build_proven_log("example.com/lys/recovery", None);
    // Roll state.json back one append: exactly the crash window between the
    // leaf write and the state write.
    let state_path = log.dir.join("state.json");
    let stale = serde_json::json!({
        "tree_size": 2,
        "root_hash": STANDARD.encode(prefix_root_of(&log, 2)),
    });
    std::fs::write(&state_path, serde_json::to_string_pretty(&stale).unwrap()).unwrap();

    let leaf = log.dir.join("../next-leaf.bin");
    std::fs::write(&leaf, b"leaf-3").unwrap();
    let append = run_lys(&[
        "log",
        "append",
        "--dir",
        path_str(&log.dir),
        "--leaf",
        path_str(&leaf),
    ]);
    assert_success(&append);
    let stderr = stderr_of(&append);
    assert!(
        stderr.contains("recovered interrupted append: state advanced to 3"),
        "{stderr}"
    );
    assert_eq!(field(&stdout_of(&append), "tree size:"), "4");
    log.close().unwrap();
}

/// Recomputes the root over the first `n` golden-style leaves of a log by
/// reading its leaf files directly (test-side cross-check only).
fn prefix_root_of(log: &ProvenLog, n: usize) -> [u8; 32] {
    let mut leaves = Vec::new();
    for i in 0..n {
        leaves.push(std::fs::read(log.dir.join("leaves").join(format!("{i:020}"))).unwrap());
    }
    let tree =
        lys_core::merkle::AppendOnlyTree::<lys_core::merkle::RawLeaf>::reconstruct_from_raw_leaves(
            &leaves,
        );
    let (root, _) = tree.root().to_parts();
    root
}

#[test]
fn corrupted_log_directories_are_refused() {
    // Modified leaf byte.
    let modified = build_proven_log("example.com/lys/corrupt-a", None);
    std::fs::write(modified.dir.join("leaves").join(format!("{:020}", 0)), b"leaf-X").unwrap();
    let leaf = modified.dir.join("../again.bin");
    std::fs::write(&leaf, b"more").unwrap();
    let output = run_lys(&[
        "log",
        "append",
        "--dir",
        path_str(&modified.dir),
        "--leaf",
        path_str(&leaf),
    ]);
    assert_eq!(output.status.code(), Some(1));
    assert!(
        stderr_of(&output).contains("log directory invalid"),
        "{}",
        stderr_of(&output)
    );

    // Gap: a missing middle leaf.
    let gapped = build_proven_log("example.com/lys/corrupt-b", None);
    std::fs::remove_file(gapped.dir.join("leaves").join(format!("{:020}", 1))).unwrap();
    let output = run_lys(&[
        "log",
        "append",
        "--dir",
        path_str(&gapped.dir),
        "--leaf",
        path_str(&leaf),
    ]);
    assert_eq!(output.status.code(), Some(1));
    assert!(
        stderr_of(&output).contains("log directory invalid"),
        "{}",
        stderr_of(&output)
    );

    // Extra non-dot entry in leaves/.
    let strayed = build_proven_log("example.com/lys/corrupt-c", None);
    std::fs::write(strayed.dir.join("leaves").join("stray.txt"), b"junk").unwrap();
    let output = run_lys(&[
        "log",
        "append",
        "--dir",
        path_str(&strayed.dir),
        "--leaf",
        path_str(&leaf),
    ]);
    assert_eq!(output.status.code(), Some(1));
    assert!(
        stderr_of(&output).contains("log directory invalid"),
        "{}",
        stderr_of(&output)
    );

    // Dotfiles (e.g. .DS_Store) are ignored, not corruption.
    let dotfiled = build_proven_log("example.com/lys/corrupt-d", None);
    std::fs::write(dotfiled.dir.join("leaves").join(".DS_Store"), b"junk").unwrap();
    let output = run_lys(&[
        "log",
        "append",
        "--dir",
        path_str(&dotfiled.dir),
        "--leaf",
        path_str(&leaf),
    ]);
    assert_success(&output);
    dotfiled.close().unwrap();
    strayed.close().unwrap();
    gapped.close().unwrap();
    // Closed last: `leaf`, read by every case above, lives in its directory.
    modified.close().unwrap();
}
