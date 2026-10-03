//! A damaged record inside the pinned prefix and a leaf handed to the
//! standalone verifier, as `lys log` reports them (LYSLOGSTORE-008 R1: the
//! leaves live in segment records, so a damaged leaf is a damaged record).

use super::*;

/// The stderr lines of `output` that start with `prefix`.
fn stderr_lines_starting(output: &Output, prefix: &str) -> Vec<String> {
    stderr_of(output)
        .lines()
        .filter(|line| line.starts_with(prefix))
        .map(str::to_string)
        .collect()
}

/// `lys log init --dir <tmp>/log --origin example.com/log`, returning the
/// tempdir and the log directory.
fn init_example_log() -> (tempfile::TempDir, PathBuf) {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("log");
    let init = run_lys(&[
        "log",
        "init",
        "--dir",
        path_str(&dir),
        "--origin",
        "example.com/log",
    ]);
    assert_success(&init);
    (tmp, dir)
}

/// `lys log append --dir <dir> --leaf <file holding bytes>`.
fn append_leaf(tmp: &Path, dir: &Path, bytes: &[u8]) {
    let leaf = tmp.join("leaf.bin");
    std::fs::write(&leaf, bytes).unwrap();
    let append = run_lys(&[
        "log",
        "append",
        "--dir",
        path_str(dir),
        "--leaf",
        path_str(&leaf),
    ]);
    assert_success(&append);
}

/// The standalone Python verifier: `LYS_PYTHON_BIN` if set, else `python3`.
fn python3() -> String {
    std::env::var("LYS_PYTHON_BIN").unwrap_or_else(|_| "python3".to_string())
}

#[test]
fn a_leaf_from_a_store_written_now_verifies_with_the_standalone_python_verifier() {
    let (tmp, dir) = init_example_log();
    let key = tmp.path().join("operator.key");
    assert_success(&run_lys(&["key", "generate", "--out", path_str(&key)]));
    append_leaf(tmp.path(), &dir, b"hello");
    let artifact = tmp.path().join("inclusion.json");
    assert_success(&run_lys(&[
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
    ]));
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/verify_inclusion.py");
    let verified = Command::new(python3())
        .arg(script)
        .arg(&artifact)
        .arg(tmp.path().join("leaf.bin"))
        .output()
        .expect("failed to spawn python3");
    assert_eq!(
        verified.status.code(),
        Some(0),
        "{}{}",
        stdout_of(&verified),
        stderr_of(&verified)
    );
}

/// Every file under `dir` with its bytes.
fn dir_bytes(dir: &Path) -> std::collections::BTreeMap<PathBuf, Vec<u8>> {
    let mut files = std::collections::BTreeMap::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(next) = pending.pop() {
        for entry in std::fs::read_dir(&next).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                pending.push(path);
            } else {
                let bytes = std::fs::read(&path).unwrap();
                files.insert(path, bytes);
            }
        }
    }
    files
}

#[test]
fn log_status_refuses_a_damaged_record_inside_the_pinned_prefix() {
    let (tmp, dir) = init_example_log();
    for leaf in [b"leaf-0".as_slice(), b"leaf-1", b"leaf-2"] {
        append_leaf(tmp.path(), &dir, leaf);
    }
    // Byte 4 of the first segment is the first byte of leaf 0: a record
    // inside the pinned prefix, never the last, so the damage cannot read as
    // a torn tail.
    let segment = dir
        .join("leaves")
        .join("segments")
        .join(format!("{:020}", 0));
    let mut bytes = std::fs::read(&segment).unwrap();
    bytes[4] ^= 0x01;
    std::fs::write(&segment, bytes).unwrap();
    let before = dir_bytes(&dir);
    let status = run_lys(&["log", "status", "--dir", path_str(&dir)]);
    assert_eq!(status.status.code(), Some(1), "{}", stderr_of(&status));
    let errors = stderr_lines_starting(&status, "error: ");
    assert_eq!(errors.len(), 1, "{}", stderr_of(&status));
    let expected_start = format!(
        "error: log directory invalid: {}: corrupt record: ",
        dir.display()
    );
    assert!(errors[0].starts_with(&expected_start), "{}", errors[0]);
    assert!(errors[0].contains(" at offset 0: "), "{}", errors[0]);
    assert_eq!(dir_bytes(&dir), before, "a refused open writes nothing");
}
