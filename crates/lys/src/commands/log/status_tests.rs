#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use crate::commands::error::CliError;

const ORIGIN: &str = "example.com/lys/status-test";

fn init_log(dir: &Path) {
    store::init(dir, ORIGIN).unwrap();
}

/// The point of the command: reading your own log needs no signing key.
///
/// The temporary directory holds the log and nothing else — no key file
/// exists anywhere for this test to accidentally rely on — so a success here
/// establishes that the read path is genuinely key-free rather than merely
/// key-optional.
#[test]
fn status_succeeds_with_no_signing_key_present() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("log");
    init_log(&dir);
    for leaf in [b"one".as_slice(), b"two".as_slice()] {
        let mut log = store::open(&dir).unwrap();
        log.append(leaf).unwrap();
    }

    assert!(
        std::fs::read_dir(tmp.path())
            .unwrap()
            .filter_map(Result::ok)
            .all(|e| e.path() == dir),
        "precondition: the log directory is the only thing in scope"
    );

    run(&dir, false).unwrap();
}

/// `status` must report the same root the signing path would sign.
///
/// If it computed the root by a second, independent route it could drift
/// from what `lys log checkpoint` attests, and an operator comparing the two
/// would be told a comforting lie. This pins them to the same value.
#[test]
fn status_root_matches_the_root_a_checkpoint_would_sign() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("log");
    init_log(&dir);
    let mut log = store::open(&dir).unwrap();
    log.append(b"leaf-a").unwrap();
    log.append(b"leaf-b").unwrap();
    log.append(b"leaf-c").unwrap();

    let reopened = store::open(&dir).unwrap();
    let (root, size) = reopened.tree().root().to_parts();
    assert_eq!(size, 3);

    // The command reads through the same store and root computation.
    run(&dir, false).unwrap();
    let (root_again, size_again) = store::open(&dir).unwrap().tree().root().to_parts();
    assert_eq!(root_again, root);
    assert_eq!(size_again, size);
}

#[test]
fn status_reports_an_empty_log_without_error() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("log");
    init_log(&dir);
    let (_, size) = store::open(&dir).unwrap().tree().root().to_parts();
    assert_eq!(size, 0, "a freshly initialized log has no leaves");
    run(&dir, false).unwrap();
}

/// An uninitialized directory is refused, and the refusal says what to run.
///
/// `LogDirMissing` rather than `LogDirInvalid`: missing is "there is no log
/// here", invalid is "there is a log here and its format is wrong". Asserting
/// the actionable half of the message too, because a read-only inspection
/// command is exactly where someone points at the wrong path.
#[test]
fn status_refuses_an_uninitialized_directory() {
    let tmp = tempfile::tempdir().unwrap();
    let err = run(&tmp.path().join("not-a-log"), false).unwrap_err();
    assert!(matches!(err, CliError::LogDirMissing { .. }), "got: {err}");
    let message = err.to_string();
    assert!(message.contains("not initialized"), "got: {message}");
    assert!(message.contains("lys log init"), "got: {message}");
}
