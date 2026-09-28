#![cfg(test)]
//! `lys-secrets --version` and `-V` name the crate version and the commit the
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
    let expected = format!("lys-secrets {} ({}", env!("CARGO_PKG_VERSION"), head());
    let cwd = tempfile::tempdir()?;
    for flag in ["--version", "-V"] {
        let output = Command::new(env!("CARGO_BIN_EXE_lys-secrets"))
            .arg(flag)
            .current_dir(cwd.path())
            .output()?;
        assert!(output.status.success(), "lys-secrets {flag} exited {}", output.status);
        let line = String::from_utf8(output.stdout)?;
        let line = line.trim_end();
        assert!(
            line == format!("{expected})") || line == format!("{expected}; dirty)"),
            "lys-secrets {flag} said `{line}`, not `{expected})`"
        );
    }
    Ok(())
}
