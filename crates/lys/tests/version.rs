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

/// The five crates whose binaries carry the stamp: every binary Lys.app
/// holds.
const STAMPED: [&str; 5] = [
    "lys",
    "lys-identity-server",
    "lys-secrets",
    "lys-home",
    "lys-app",
];

#[test]
fn every_stamped_crate_carries_the_same_build_script() -> TestResult {
    let crates = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let ours = std::fs::read(crates.join("lys").join("build.rs"))?;
    let mut compared = 0;
    for name in STAMPED {
        let theirs = std::fs::read(crates.join(name).join("build.rs"))?;
        assert_eq!(theirs, ours, "{name}/build.rs differs from lys/build.rs");
        compared += 1;
    }
    assert_eq!(compared, STAMPED.len());
    Ok(())
}

/// Builds a crate whose build script is this crate's, in `tree`, and
/// returns what the built binary says its build is.
fn probe(tree: &std::path::Path) -> Result<String, Box<dyn Error>> {
    std::fs::create_dir_all(tree.join("src"))?;
    std::fs::write(
        tree.join("Cargo.toml"),
        "[package]\nname = \"stamp-probe\"\nversion = \"0.0.0\"\nedition = \"2024\"\n\n[workspace]\n",
    )?;
    std::fs::copy(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("build.rs"),
        tree.join("build.rs"),
    )?;
    std::fs::write(
        tree.join("src").join("main.rs"),
        "fn main() {\n    println!(\"{}\", env!(\"LYS_BUILD\"));\n}\n",
    )?;
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let output = Command::new(cargo)
        .args(["run", "--quiet", "--offline", "--manifest-path"])
        .arg(tree.join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", tree.join("target"))
        .env("GIT_CEILING_DIRECTORIES", tree.parent().ok_or("no parent")?)
        .output()?;
    assert!(
        output.status.success(),
        "the probe did not build: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(String::from_utf8(output.stdout)?.trim().to_string())
}

#[test]
fn a_build_from_an_exported_tree_says_it_has_no_commit() -> TestResult {
    let scratch = tempfile::tempdir()?;
    let tree = scratch.path().join("exported");
    assert_eq!(probe(&tree)?, NO_COMMIT);
    assert!(!tree.join(".git").exists());
    Ok(())
}

#[test]
fn a_build_from_a_commit_names_it_and_its_dirty_state() -> TestResult {
    let scratch = tempfile::tempdir()?;
    let tree = scratch.path().join("committed");
    std::fs::create_dir_all(&tree)?;
    let git = |args: &[&str]| -> Result<String, Box<dyn Error>> {
        let output = Command::new("git")
            .args([
                "-c",
                "user.name=Probe",
                "-c",
                "user.email=probe@example.test",
                "-c",
                "commit.gpgsign=false",
            ])
            .args(args)
            .current_dir(&tree)
            .env("GIT_CEILING_DIRECTORIES", scratch.path())
            .output()?;
        assert!(output.status.success(), "git {args:?} failed");
        Ok(String::from_utf8(output.stdout)?.trim().to_string())
    };
    git(&["init", "--quiet"])?;
    std::fs::write(tree.join(".gitignore"), "/target\n")?;
    std::fs::create_dir_all(tree.join("src"))?;
    // The probe writes its files, then they are committed and built clean.
    probe(&tree)?;
    git(&["add", "."])?;
    git(&["commit", "--quiet", "-m", "probe"])?;
    let commit = git(&["rev-parse", "HEAD"])?;
    assert_eq!(commit.len(), 40);
    assert_eq!(probe(&tree)?, commit);
    std::fs::write(tree.join("src").join("extra.txt"), "a change\n")?;
    git(&["add", "src/extra.txt"])?;
    assert_eq!(probe(&tree)?, format!("{commit}; dirty"));
    Ok(())
}
