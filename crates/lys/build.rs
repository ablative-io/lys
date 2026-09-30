//! Records build provenance so exported sources remain distinguishable.
//! Explicit provenance is validated and marked as stated; when a tracked
//! checkout supplies independent provenance, the two must agree.
//! All binaries share this implementation to keep provenance consistent.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// The words a build with no commit to read carries.
const NO_COMMIT: &str = "not built from a git commit";

/// Reads independent provenance only when the command succeeds.
fn git(dir: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .args(["--no-optional-locks"])
        .args(args)
        .current_dir(dir)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout)
        .ok()
        .map(|text| text.trim().to_string())
}

/// Watches repository changes so cached builds retain current provenance.
fn watched(dir: &Path) -> Vec<String> {
    let mut paths = vec![
        "HEAD".to_string(),
        "index".to_string(),
        "packed-refs".to_string(),
        "refs/heads".to_string(),
    ];
    if let Some(branch) = git(dir, &["symbolic-ref", "-q", "HEAD"]) {
        paths.push(branch);
    }
    let mut watched = Vec::new();
    for path in paths {
        let args = [
            "rev-parse",
            "--path-format=absolute",
            "--git-path",
            path.as_str(),
        ];
        if let Some(found) = git(dir, &args) {
            watched.push(found);
        }
    }
    watched
}

/// Validates explicit provenance before it can enter the stamp.
fn stated() -> Result<Option<String>, String> {
    let commit = match std::env::var("LYS_BUILD_COMMIT") {
        Ok(commit) => commit,
        Err(std::env::VarError::NotPresent) => return Ok(None),
        Err(std::env::VarError::NotUnicode(_)) => {
            return Err("LYS_BUILD_COMMIT must be 40 lowercase hexadecimal characters".into());
        }
    };
    if commit.len() != 40
        || !commit
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
    {
        return Err("LYS_BUILD_COMMIT must be 40 lowercase hexadecimal characters".into());
    }
    Ok(Some(commit))
}

/// Distinguishes explicit provenance from independently read provenance.
fn stamp(dir: &Path) -> Result<(String, Vec<String>), String> {
    let stated = stated()?;
    let watched = watched(dir);
    let tracked = git(dir, &["ls-files", "--error-unmatch", "Cargo.toml"]).is_some();
    let commit = git(dir, &["rev-parse", "HEAD"])
        .filter(|commit| commit.len() == 40 && commit.bytes().all(|b| b.is_ascii_hexdigit()));
    let Some(commit) = commit.filter(|_| tracked) else {
        let value = stated.map_or_else(
            || NO_COMMIT.to_string(),
            |commit| format!("{commit}; stated"),
        );
        return Ok((value, watched));
    };
    let mut value = if let Some(stated) = stated {
        if stated != commit {
            return Err("LYS_BUILD_COMMIT disagrees with the Git HEAD commit".into());
        }
        format!("{commit}; stated")
    } else {
        commit
    };
    let dirty = git(dir, &["status", "--porcelain"]).is_some_and(|changes| !changes.is_empty());
    if dirty {
        value.push_str("; dirty");
    }
    Ok((value, watched))
}

fn main() -> Result<(), String> {
    let dir = std::env::var_os("CARGO_MANIFEST_DIR").map_or_else(|| ".".into(), PathBuf::from);
    println!("cargo:rerun-if-env-changed=LYS_BUILD_COMMIT");
    let (value, watched) = stamp(&dir)?;
    println!("cargo:rerun-if-changed=build.rs");
    for path in watched {
        if Path::new(&path).exists() {
            println!("cargo:rerun-if-changed={path}");
        }
    }
    println!("cargo:rustc-env=LYS_BUILD={value}");
    Ok(())
}
