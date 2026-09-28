//! The golden design vectors reproduced byte for byte, and a third party
//! verifying with only the artifacts, the leaf and the verifier key.

use lys_core::merkle::raw_leaf_hash;

use super::{
    GOLDEN_LEAF0_HASH, GOLDEN_NOTE, GOLDEN_ORIGIN, GOLDEN_ROOT3_HEX, GOLDEN_SEED,
    GOLDEN_VERIFIER_SPEC, assert_success, build_proven_log, field, hex_lower, path_str, run_lys,
    stdout_of,
};

// ------------------------------------------------------- golden design vectors

#[test]
fn golden_log_reproduces_design_vectors_byte_for_byte() {
    let log = build_proven_log(GOLDEN_ORIGIN, Some(GOLDEN_SEED));

    // Leaf 0's printed hash is the golden RFC 6962 raw-leaf hash,
    // reproducible with `(printf '\x00'; cat leaf-file) | shasum -a 256`.
    assert_eq!(
        field(&log.append_stdouts[0], "leaf hash (sha256, rfc6962):"),
        GOLDEN_LEAF0_HASH
    );
    assert_eq!(hex_lower(&raw_leaf_hash(b"leaf-0")), GOLDEN_LEAF0_HASH);

    // The size-3 root and the verifier key match the design vectors.
    assert_eq!(
        field(&log.append_stdouts[2], "root hash (sha256):"),
        GOLDEN_ROOT3_HEX
    );
    let checkpoint_stdout = &log.checkpoint_stdout;
    assert_eq!(
        field(checkpoint_stdout, "root hash (sha256):"),
        GOLDEN_ROOT3_HEX
    );
    assert_eq!(field(checkpoint_stdout, "tree size:"), "3");
    assert_eq!(field(checkpoint_stdout, "origin:"), GOLDEN_ORIGIN);
    assert_eq!(log.verifier, GOLDEN_VERIFIER_SPEC);

    // The checkpoint note file is the golden note, byte for byte, and the
    // signature-line prefix is the exact em-dash bytes E2 80 94 20.
    let note_bytes = std::fs::read(&log.checkpoint_file).unwrap();
    assert_eq!(note_bytes, GOLDEN_NOTE.as_bytes());
    assert!(
        note_bytes.windows(4).any(|w| w == [0xe2, 0x80, 0x94, 0x20]),
        "note must contain the em-dash-space signature prefix bytes"
    );
    log.close().unwrap();
}

// ---------------------------------------------------- third-party verification

#[test]
fn e2e_third_party_verifies_with_only_artifacts_leaf_and_verifier_key() {
    let log = build_proven_log("example.com/lys/e2e", None);
    let seed_hex = hex_lower(&std::fs::read(&log.key).unwrap());

    // "Process B": a fresh directory holding ONLY the two artifacts and the
    // one proven leaf file. No log directory, no key.
    let third_party = tempfile::tempdir().unwrap();
    let inclusion = third_party.path().join("inclusion.json");
    let consistency = third_party.path().join("consistency.json");
    let leaf = third_party.path().join("leaf-1.bin");
    std::fs::copy(&log.inclusion_artifact, &inclusion).unwrap();
    std::fs::copy(&log.consistency_artifact, &consistency).unwrap();
    std::fs::copy(&log.leaf_files[1], &leaf).unwrap();
    let entries_before = std::fs::read_dir(third_party.path()).unwrap().count();
    assert_eq!(
        entries_before, 3,
        "third-party dir must hold exactly 3 files"
    );

    let verify_inclusion = run_lys(&[
        "log",
        "verify",
        "inclusion",
        "--artifact",
        path_str(&inclusion),
        "--leaf",
        path_str(&leaf),
        "--verifier-key",
        &log.verifier,
    ]);
    assert_success(&verify_inclusion);
    let stdout = stdout_of(&verify_inclusion);
    assert!(stdout.contains("inclusion verified"), "{stdout}");
    assert_eq!(field(&stdout, "origin:"), "example.com/lys/e2e");
    assert_eq!(field(&stdout, "tree size:"), "3");
    assert_eq!(field(&stdout, "leaf index:"), "1");
    assert_eq!(
        field(&stdout, "root hash (sha256):"),
        field(&log.checkpoint_stdout, "root hash (sha256):"),
        "verified root must match the checkpointed root"
    );

    let verify_consistency = run_lys(&[
        "log",
        "verify",
        "consistency",
        "--artifact",
        path_str(&consistency),
        "--verifier-key",
        &log.verifier,
    ]);
    assert_success(&verify_consistency);
    let stdout = stdout_of(&verify_consistency);
    assert!(stdout.contains("consistency verified"), "{stdout}");
    assert_eq!(field(&stdout, "origin:"), "example.com/lys/e2e");
    assert_eq!(field(&stdout, "old tree size:"), "2");
    assert_eq!(field(&stdout, "new tree size:"), "3");

    // The third-party directory gained nothing: still exactly 3 files, no
    // log directory materialized.
    let entries_after: Vec<_> = std::fs::read_dir(third_party.path())
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(entries_after.len(), 3, "verify must not create files");

    // The verify subcommands accept no --dir flag at all (clap exit 2).
    let with_dir = run_lys(&[
        "log",
        "verify",
        "inclusion",
        "--artifact",
        path_str(&inclusion),
        "--leaf",
        path_str(&leaf),
        "--verifier-key",
        &log.verifier,
        "--dir",
        path_str(&log.dir),
    ]);
    assert_eq!(with_dir.status.code(), Some(2), "--dir must be rejected");
    let with_dir_consistency = run_lys(&[
        "log",
        "verify",
        "consistency",
        "--artifact",
        path_str(&consistency),
        "--verifier-key",
        &log.verifier,
        "--dir",
        path_str(&log.dir),
    ]);
    assert_eq!(with_dir_consistency.status.code(), Some(2));

    // No private key material in any output from either side.
    for transcript in log
        .transcripts
        .iter()
        .map(String::as_str)
        .chain([stdout.as_str()])
    {
        assert!(
            !transcript.contains(&seed_hex),
            "private seed leaked into output"
        );
    }
    log.close().unwrap();
}
