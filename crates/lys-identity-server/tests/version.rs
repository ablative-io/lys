#![cfg(test)]
//! `lys-identity-server --version` and `-V` name the crate version and the commit the
//! binary was built from, read here from git as a second party to the stamp
//! `build.rs` took.

use std::error::Error;
use std::process::Command;

#[path = "support/build_head.rs"]
mod build_head;

type TestResult = Result<(), Box<dyn Error>>;

#[test]
fn version_names_the_crate_version_and_the_head_commit() -> TestResult {
    let head = build_head::at_build()?;
    let expected = format!("lys-identity-server {} ({head}", env!("CARGO_PKG_VERSION"));
    let cwd = tempfile::tempdir()?;
    // A file named like the flag, holding no configuration: the server
    // reading its first argument as a configuration path would fail on it.
    std::fs::write(cwd.path().join("--version"), "not a configuration")?;
    std::fs::write(cwd.path().join("-V"), "not a configuration")?;
    for flag in ["--version", "-V"] {
        build_head::unchanged(&head)?;
        let output = Command::new(env!("CARGO_BIN_EXE_lys-identity-server"))
            .arg(flag)
            .current_dir(cwd.path())
            .output()?;
        build_head::unchanged(&head)?;
        assert!(
            output.status.success(),
            "lys-identity-server {flag} exited {}",
            output.status
        );
        let line = String::from_utf8(output.stdout)?;
        let line = line.trim_end();
        assert!(
            line == format!("{expected})") || line == format!("{expected}; dirty)"),
            "lys-identity-server {flag} said `{line}`, not `{expected})`"
        );
    }
    Ok(())
}
