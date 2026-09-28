//! Proving and verifying: the edges of `lys log prove`, and every tamper class
//! refused with one identical message.

use std::path::Path;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;

use super::{
    CONSISTENCY_FAIL_STDERR, INCLUSION_FAIL_STDERR, assert_success, build_proven_log, load_json,
    path_str, run_lys, stderr_of, write_json,
};

// ------------------------------------------------------------- prove edge cases

#[test]
fn prove_inclusion_out_of_range_index_is_actionable() {
    let log = build_proven_log("example.com/lys/range", None);
    let out = log.dir.join("../oob.json");
    let output = run_lys(&[
        "log",
        "prove",
        "inclusion",
        "--dir",
        path_str(&log.dir),
        "--key",
        path_str(&log.key),
        "--leaf-index",
        "3",
        "--out",
        path_str(&out),
    ]);
    assert_eq!(output.status.code(), Some(1));
    let stderr = stderr_of(&output);
    assert!(stderr.contains("leaf index 3"), "{stderr}");
    assert!(stderr.contains("3 leaves"), "{stderr}");
}

#[test]
fn prove_consistency_enforces_strict_size_rules() {
    let log = build_proven_log("example.com/lys/sizes", None);
    let out = log.dir.join("../c.json");
    // old_size == current size: vacuous, refused with an actionable message.
    let equal = run_lys(&[
        "log",
        "prove",
        "consistency",
        "--dir",
        path_str(&log.dir),
        "--key",
        path_str(&log.key),
        "--old-size",
        "3",
        "--out",
        path_str(&out),
    ]);
    assert_eq!(equal.status.code(), Some(1));
    assert!(
        stderr_of(&equal).contains("strictly below"),
        "{}",
        stderr_of(&equal)
    );
    // old_size == 0: rejected by clap's range parser (exit 2).
    let zero = run_lys(&[
        "log",
        "prove",
        "consistency",
        "--dir",
        path_str(&log.dir),
        "--key",
        path_str(&log.key),
        "--old-size",
        "0",
        "--out",
        path_str(&out),
    ]);
    assert_eq!(zero.status.code(), Some(2));
}

// -------------------------------------------------- tamper matrix — inclusion

/// Runs `lys log verify inclusion` against a mutated artifact and returns
/// `(exit code, stderr)`.
fn verify_inclusion_raw_output(
    artifact: &Path,
    leaf: &Path,
    verifier: &str,
) -> (Option<i32>, String) {
    let output = run_lys(&[
        "log",
        "verify",
        "inclusion",
        "--artifact",
        path_str(artifact),
        "--leaf",
        path_str(leaf),
        "--verifier-key",
        verifier,
    ]);
    (output.status.code(), stderr_of(&output))
}

#[test]
fn verify_inclusion_rejects_every_tamper_class_with_one_identical_message() {
    let log = build_proven_log("example.com/lys/tamper-i", None);
    let tmp = tempfile::tempdir().unwrap();
    let original = load_json(&log.inclusion_artifact);
    let consistency = load_json(&log.consistency_artifact);
    let leaf1 = &log.leaf_files[1];

    let mut tampers: Vec<(&str, serde_json::Value)> = Vec::new();

    let mut v = original.clone();
    v["tree_size"] = serde_json::json!(4);
    tampers.push(("tree_size off by one", v));

    let mut v = original.clone();
    v["leaf_index"] = serde_json::json!(2);
    tampers.push(("leaf_index off by one", v));

    let mut v = original.clone();
    v["leaf_index"] = serde_json::json!(3);
    tampers.push(("leaf_index == tree_size", v));

    let mut v = original.clone();
    let entry = v["hashes"][0].as_str().unwrap().to_string();
    let flipped = if entry.starts_with('M') {
        entry.replacen('M', "N", 1)
    } else {
        format!("M{}", &entry[1..])
    };
    v["hashes"][0] = serde_json::json!(flipped);
    tampers.push(("hash entry bit-flipped", v));

    let mut v = original.clone();
    v["hashes"][0] = serde_json::json!(STANDARD.encode([0u8; 31]));
    tampers.push(("hash entry decodes to 31 bytes", v));

    let mut v = original.clone();
    v["hashes"][0] = serde_json::json!("AAA");
    tampers.push(("hash entry unpadded base64", v));

    let mut v = original.clone();
    let extra = v["hashes"][0].clone();
    v["hashes"].as_array_mut().unwrap().push(extra);
    tampers.push(("extra hash appended", v));

    let mut v = original.clone();
    v["hashes"].as_array_mut().unwrap().remove(0);
    tampers.push(("hash removed", v));

    let mut v = original.clone();
    v["checkpoint"] = consistency["checkpoint_1"].clone();
    tampers.push(("checkpoint swapped for same-log size-2 checkpoint", v));

    let mut v = original;
    v["format"] = serde_json::json!("lys/log-consistency-proof/v1");
    tampers.push(("format swapped to the other kind", v));

    let mut stderrs = Vec::new();
    for (index, (label, value)) in tampers.iter().enumerate() {
        let path = write_json(tmp.path(), &format!("tamper-{index}.json"), value);
        let (code, stderr) = verify_inclusion_raw_output(&path, leaf1, &log.verifier);
        assert_eq!(code, Some(1), "tamper {label:?} did not fail");
        assert_eq!(
            stderr, INCLUSION_FAIL_STDERR,
            "tamper {label:?} leaked detail"
        );
        stderrs.push(stderr);
    }
    // Wrong leaf bytes: same class, same message.
    let (code, stderr) =
        verify_inclusion_raw_output(&log.inclusion_artifact, &log.leaf_files[0], &log.verifier);
    assert_eq!(code, Some(1));
    assert_eq!(stderr, INCLUSION_FAIL_STDERR);
    stderrs.push(stderr);
    // Wrong (but well-formed) verifier key: same class, same message.
    let other = build_proven_log("example.com/lys/tamper-i", None);
    let (code, stderr) =
        verify_inclusion_raw_output(&log.inclusion_artifact, leaf1, &other.verifier);
    assert_eq!(code, Some(1));
    assert_eq!(stderr, INCLUSION_FAIL_STDERR);
    stderrs.push(stderr);
    // The strongest non-oracle assertion: all failures are byte-identical.
    assert!(
        stderrs.windows(2).all(|w| w[0] == w[1]),
        "tamper classes must be indistinguishable"
    );
}

#[test]
fn verify_inclusion_shape_errors_are_actionable_json_parse_failures() {
    let log = build_proven_log("example.com/lys/shape-i", None);
    let tmp = tempfile::tempdir().unwrap();
    let leaf1 = &log.leaf_files[1];

    // Unknown extra field: not valid v1 (deny_unknown_fields).
    let mut v = load_json(&log.inclusion_artifact);
    v["timestamp"] = serde_json::json!(123);
    let path = write_json(tmp.path(), "unknown-field.json", &v);
    let (code, stderr) = verify_inclusion_raw_output(&path, leaf1, &log.verifier);
    assert_eq!(code, Some(1));
    assert!(
        stderr.contains("failed to parse inclusion proof artifact JSON"),
        "{stderr}"
    );

    // Duplicate JSON key: rejected by serde.
    let text = std::fs::read_to_string(&log.inclusion_artifact).unwrap();
    let duplicated = text.replacen(
        "\"tree_size\": 3,",
        "\"tree_size\": 3,\n  \"tree_size\": 9,",
        1,
    );
    assert_ne!(text, duplicated, "fixture must contain the expected field");
    let dup_path = tmp.path().join("duplicate-key.json");
    std::fs::write(&dup_path, duplicated).unwrap();
    let (code, stderr) = verify_inclusion_raw_output(&dup_path, leaf1, &log.verifier);
    assert_eq!(code, Some(1));
    assert!(
        stderr.contains("failed to parse inclusion proof artifact JSON"),
        "{stderr}"
    );

    // Kind confusion at the shape level: a consistency artifact is not
    // shaped like an inclusion artifact.
    let (code, stderr) =
        verify_inclusion_raw_output(&log.consistency_artifact, leaf1, &log.verifier);
    assert_eq!(code, Some(1));
    assert!(
        stderr.contains("failed to parse inclusion proof artifact JSON"),
        "{stderr}"
    );

    // Malformed verifier key string: trusted operator input, actionable.
    let (code, stderr) = verify_inclusion_raw_output(&log.inclusion_artifact, leaf1, "not-a-key");
    assert_eq!(code, Some(1));
    assert!(stderr.contains("invalid note verifier key"), "{stderr}");
}

// ------------------------------------------------ tamper matrix — consistency

/// Runs `lys log verify consistency` and returns `(exit code, stderr)`.
fn verify_consistency_raw_output(artifact: &Path, verifier: &str) -> (Option<i32>, String) {
    let output = run_lys(&[
        "log",
        "verify",
        "consistency",
        "--artifact",
        path_str(artifact),
        "--verifier-key",
        verifier,
    ]);
    (output.status.code(), stderr_of(&output))
}

#[test]
fn verify_consistency_rejects_every_tamper_class_with_one_identical_message() {
    let log = build_proven_log("example.com/lys/tamper-c", None);
    // A second log, same operator key file copied, DIFFERENT origin: its
    // checkpoints are signed by the same key but must never verify here.
    let other_origin = {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("log");
        assert_success(&run_lys(&[
            "log",
            "init",
            "--dir",
            path_str(&dir),
            "--origin",
            "example.com/lys/other-origin",
        ]));
        for leaf in &log.leaf_files {
            assert_success(&run_lys(&[
                "log",
                "append",
                "--dir",
                path_str(&dir),
                "--leaf",
                path_str(leaf),
            ]));
        }
        let note = tmp.path().join("cp.note");
        assert_success(&run_lys(&[
            "log",
            "checkpoint",
            "--dir",
            path_str(&dir),
            "--key",
            path_str(&log.key),
            "--out",
            path_str(&note),
        ]));
        std::fs::read_to_string(&note).unwrap()
    };

    let tmp = tempfile::tempdir().unwrap();
    let original = load_json(&log.consistency_artifact);

    let mut tampers: Vec<(&str, serde_json::Value)> = Vec::new();

    let mut v = original.clone();
    v["tree_size_1"] = serde_json::json!(1);
    tampers.push(("tree_size_1 off by one", v));

    let mut v = original.clone();
    v["tree_size_2"] = serde_json::json!(4);
    tampers.push(("tree_size_2 off by one", v));

    let mut v = original.clone();
    v["tree_size_1"] = serde_json::json!(3);
    tampers.push(("tree_size_1 == tree_size_2", v));

    let mut v = original.clone();
    v["tree_size_1"] = serde_json::json!(0);
    tampers.push(("tree_size_1 == 0", v));

    let mut v = original.clone();
    let cp1 = v["checkpoint_1"].clone();
    v["checkpoint_1"] = v["checkpoint_2"].clone();
    v["checkpoint_2"] = cp1;
    tampers.push(("checkpoints exchanged", v));

    let mut v = original.clone();
    let entry = v["hashes"][0].as_str().unwrap().to_string();
    let flipped = if entry.starts_with('/') {
        entry.replacen('/', "A", 1)
    } else {
        format!("/{}", &entry[1..])
    };
    v["hashes"][0] = serde_json::json!(flipped);
    tampers.push(("hash entry bit-flipped", v));

    let mut v = original.clone();
    v["checkpoint_2"] = serde_json::json!(other_origin);
    tampers.push(("checkpoint_2 from a different origin, same key", v));

    let mut v = original;
    v["format"] = serde_json::json!("lys/log-inclusion-proof/v1");
    tampers.push(("format swapped to the other kind", v));

    let mut stderrs = Vec::new();
    for (index, (label, value)) in tampers.iter().enumerate() {
        let path = write_json(tmp.path(), &format!("tamper-{index}.json"), value);
        let (code, stderr) = verify_consistency_raw_output(&path, &log.verifier);
        assert_eq!(code, Some(1), "tamper {label:?} did not fail");
        assert_eq!(
            stderr, CONSISTENCY_FAIL_STDERR,
            "tamper {label:?} leaked detail"
        );
        stderrs.push(stderr);
    }
    assert!(
        stderrs.windows(2).all(|w| w[0] == w[1]),
        "tamper classes must be indistinguishable"
    );

    // Kind confusion at the shape level for the consistency verifier.
    let (code, stderr) = verify_consistency_raw_output(&log.inclusion_artifact, &log.verifier);
    assert_eq!(code, Some(1));
    assert!(
        stderr.contains("failed to parse consistency proof artifact JSON"),
        "{stderr}"
    );
}
