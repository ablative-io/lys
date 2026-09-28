//! Stamps the commit this crate is built from into `LYS_BUILD`, which every
//! binary's `--version` prints as `NAME VERSION (LYS_BUILD)`.
//!
//! The value is the 40-character commit `git rev-parse HEAD` names, followed
//! by `; dirty` when `git status --porcelain` lists any change. A build with
//! no git tree to read, or whose manifest the tree it finds does not track
//! (an exported tree unpacked inside some other repository), says
//! `not built from a git commit`: never an empty or invented value. The
//! stamp is taken again when HEAD, the branch it names, or the index moves.
//!
//! The same file stands in `crates/lys`, `crates/lys-identity-server`,
//! `crates/lys-secrets` and `crates/lys-home`, byte for byte; a test holds
//! them equal. It uses the standard library only, so a crate built from an
//! exported tree builds it the same way.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// The words a build with no commit to read carries.
const NO_COMMIT: &str = "not built from a git commit";

/// What `git` says in `dir`, trimmed, when it runs and succeeds.
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

/// The stamp for a crate whose manifest is in `dir`, with the files whose
/// change should take it again.
fn stamp(dir: &Path) -> (String, Vec<String>) {
    let tracked = git(dir, &["ls-files", "--error-unmatch", "Cargo.toml"]).is_some();
    let commit = git(dir, &["rev-parse", "HEAD"])
        .filter(|commit| commit.len() == 40 && commit.bytes().all(|b| b.is_ascii_hexdigit()));
    let Some(commit) = commit.filter(|_| tracked) else {
        return (NO_COMMIT.to_string(), Vec::new());
    };
    let mut watched = Vec::new();
    let mut paths = vec![
        "HEAD".to_string(),
        "index".to_string(),
        "packed-refs".to_string(),
    ];
    if let Some(branch) = git(dir, &["symbolic-ref", "-q", "HEAD"]) {
        paths.push(branch);
    }
    for path in paths {
        let args = ["rev-parse", "--path-format=absolute", "--git-path", path.as_str()];
        if let Some(found) = git(dir, &args) {
            watched.push(found);
        }
    }
    let dirty = git(dir, &["status", "--porcelain"]).is_some_and(|changes| !changes.is_empty());
    let value = if dirty {
        format!("{commit}; dirty")
    } else {
        commit
    };
    (value, watched)
}

fn main() {
    let dir = std::env::var_os("CARGO_MANIFEST_DIR").map_or_else(|| ".".into(), PathBuf::from);
    let (value, watched) = stamp(&dir);
    println!("cargo:rerun-if-changed=build.rs");
    for path in watched {
        if Path::new(&path).exists() {
            println!("cargo:rerun-if-changed={path}");
        }
    }
    println!("cargo:rustc-env=LYS_BUILD={value}");
}
