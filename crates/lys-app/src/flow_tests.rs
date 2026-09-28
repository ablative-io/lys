#![cfg(test)]

use std::path::{Path, PathBuf};

use lys_install::error::{ErrorKind, IdentityError};

use super::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn the_browser_is_handed_to_first_run_setup_on_lys() {
    assert_eq!(setup_url(), "http://localhost:8490/setup");
}

#[test]
fn a_named_root_is_the_data_root() -> TestResult {
    let layout = layout(Some(Path::new("/tmp/lys-root")))?;
    assert_eq!(layout.root, Path::new("/tmp/lys-root"));
    Ok(())
}

#[test]
fn the_page_never_names_the_issuer() {
    assert_eq!(
        for_page("rauthy ready; Rauthy answered"),
        "sign-in ready; sign-in answered"
    );
}

/// An app on the disk image refuses before anything is read or written, and
/// the refusal is final: "Try again" cannot help until Lys is moved.
#[test]
fn an_app_on_the_disk_image_installs_nothing() -> TestResult {
    let root = tempfile::tempdir()?;
    let layout = Layout::at(root.path().join("identity"));
    let bundle = Bundle {
        binaries: PathBuf::from("/Volumes/Lys/Lys.app/Contents/MacOS"),
    };
    let board = Board::new();
    let failure = attempt(&layout, &bundle, &board)
        .err()
        .ok_or("it was not refused")?;
    assert_eq!(failure.refusal.name, "app_not_in_applications");
    assert!(!failure.retry);
    assert!(!layout.root.exists(), "the data root was made");
    Ok(())
}

/// A copy of the app without its screens refuses by name before anything
/// is installed, and "Try again" is not offered: only another copy helps.
#[test]
fn a_copy_without_screens_installs_nothing() -> TestResult {
    let root = tempfile::tempdir()?;
    let layout = Layout::at(root.path().join("identity"));
    let bundle = Bundle {
        binaries: root.path().join("Lys.app/Contents/MacOS"),
    };
    let failure = attempt(&layout, &bundle, &Board::new())
        .err()
        .ok_or("it was not refused")?;
    assert_eq!(failure.refusal.name, "screens_missing");
    assert!(!failure.retry);
    assert!(!layout.root.exists(), "the data root was made");
    Ok(())
}

/// A failed step is shown in its own words, the next thing to do is to try
/// again, and the install's own error stays out of what the page reads.
#[test]
fn a_failure_is_shown_at_its_step_with_the_next_thing_to_do() -> TestResult {
    let board = Board::new();
    let mut run = Run::new(&board, Work::Install);
    run.step(Step::SignIn);
    let error = IdentityError::new(
        ErrorKind::RauthyUnreachable,
        "call Rauthy",
        "rauthy",
        "connection refused",
    );
    let refusal = run.stopped(&error);
    assert_eq!(refusal.name, "install_step_failed");
    assert_eq!(refusal.words, "Starting sign-in did not finish.");
    assert!(refusal.next.contains("Try again"));
    let shown = Phase::failed(&refusal, true, run.steps());
    let Phase::Failed { steps, .. } = &shown else {
        return Err(format!("not a failure: {shown:?}").into());
    };
    let states: Vec<&str> = steps.iter().map(|step| step.state).collect();
    assert_eq!(
        states,
        [
            "done", "done", "now", "waiting", "waiting", "waiting", "waiting", "waiting"
        ]
    );
    let page = format!("{shown:?}").to_lowercase();
    assert!(!page.contains("rauthy"), "{page}");
    assert!(!page.contains("connection refused"), "{page}");
    Ok(())
}

/// An upgrade's steps are lines, each with the issuer called sign-in.
#[test]
fn an_upgrade_says_its_lines() -> TestResult {
    let board = Board::new();
    let mut run = Run::new(&board, Work::Upgrade);
    run.step(Step::SignIn);
    run.say("rauthy ready");
    let Phase::Working { said, steps, .. } = board.now().phase else {
        return Err("the upgrade is not shown working".into());
    };
    assert_eq!(said, ["Starting sign-in", "sign-in ready"]);
    assert!(steps.is_empty());
    Ok(())
}

/// Every program the app and the install start is named, and none of them
/// is a shell, a terminal or a script runner: the person's machine is never
/// asked to run a command on their behalf.
#[test]
fn no_shell_or_terminal_is_ever_started() -> TestResult {
    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let refused = [
        "\"sh\"",
        "\"/bin/sh\"",
        "\"bash\"",
        "\"/bin/bash\"",
        "\"zsh\"",
        "\"/bin/zsh\"",
        "Terminal",
        "osascript",
        "iTerm",
    ];
    let mut read = 0;
    let mut dirs = vec![crates.join("lys-app/src"), crates.join("lys-install/src")];
    while let Some(dir) = dirs.pop() {
        for entry in std::fs::read_dir(&dir)? {
            let path = entry?.path();
            if path.is_dir() {
                dirs.push(path);
                continue;
            }
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or_default();
            let rust = path.extension().is_some_and(|extension| extension == "rs");
            if !rust || name.ends_with("_tests.rs") {
                continue;
            }
            let text = std::fs::read_to_string(&path)?;
            for word in refused {
                assert!(!text.contains(word), "{} holds {word}", path.display());
            }
            read += 1;
        }
    }
    assert!(read >= 30, "read {read} source files");
    Ok(())
}
