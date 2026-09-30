#![cfg(test)]
//! `lys --version` and `-V` name the crate version and the commit the
//! binary was built from, read here from git as a second party to the stamp
//! `build.rs` took.

use std::error::Error;
use std::process::Command;

type TestResult = Result<(), Box<dyn Error>>;

/// The words a build with no commit to read carries.
const NO_COMMIT: &str = "not built from a git commit";

/// What git says now about the tree this crate is in: its HEAD commit, or
/// the words when there is no git tree tracking this crate.
fn head() -> String {
    let git = |args: &[&str]| {
        Command::new("git")
            .arg("--no-optional-locks")
            .args(args)
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .output()
            .ok()
            .filter(|output| output.status.success())
            .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
    };
    let tracked = git(&["ls-files", "--error-unmatch", "Cargo.toml"]).is_some();
    match git(&["rev-parse", "HEAD"]).filter(|_| tracked) {
        Some(commit) => {
            assert_eq!(commit.len(), 40, "{commit}");
            commit
        }
        None => NO_COMMIT.to_string(),
    }
}

#[test]
fn version_names_the_crate_version_and_the_head_commit() -> TestResult {
    let expected = format!("lys {} ({}", env!("CARGO_PKG_VERSION"), head());
    let cwd = tempfile::tempdir()?;
    for flag in ["--version", "-V"] {
        let output = Command::new(env!("CARGO_BIN_EXE_lys"))
            .arg(flag)
            .current_dir(cwd.path())
            .output()?;
        assert!(
            output.status.success(),
            "lys {flag} exited {}",
            output.status
        );
        let line = String::from_utf8(output.stdout)?;
        let line = line.trim_end();
        assert!(
            line == format!("{expected})") || line == format!("{expected}; dirty)"),
            "lys {flag} said `{line}`, not `{expected})`"
        );
    }
    Ok(())
}

/// The four crates whose binaries carry the stamp.
const STAMPED: [&str; 4] = ["lys", "lys-identity-server", "lys-secrets", "lys-home"];

#[test]
fn every_stamped_crate_calls_shared_stamp_without_copied_logic() -> TestResult {
    let crates = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut compared = 0;
    for name in STAMPED {
        let source = std::fs::read_to_string(crates.join(name).join("build.rs"))?;
        let code = source
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");
        assert_eq!(
            code, "fn main() {\nlys_build_stamp::emit();\n}",
            "{name}/build.rs must call the shared stamp"
        );
        compared += 1;
    }
    assert_eq!(compared, STAMPED.len());
    Ok(())
}
