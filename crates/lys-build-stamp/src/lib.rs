//! The build stamp every Lys binary carries in `LYS_BUILD`, which its
//! `--version` prints as `NAME VERSION (LYS_BUILD)`.
//!
//! The value is the 40-character commit `git rev-parse HEAD` names, followed
//! by `; dirty` when `git status --porcelain` lists any change. A build with
//! no git tree to read, or whose manifest the tree it finds does not track
//! (an exported tree unpacked inside some other repository), says
//! [`NO_COMMIT`]: never an empty or invented value. The stamp is taken again
//! when the build script, HEAD, the branch it names, the packed refs or the
//! index moves.
//!
//! Invariant: this is the only place the stamp is computed. Each stamped
//! crate lists this crate under `build-dependencies` and its `build.rs` is
//! the one call [`emit`]. It uses the standard library only, so a crate
//! built from an exported tree builds it the same way.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// The words a build with no commit to read carries.
pub const NO_COMMIT: &str = "not built from a git commit";

/// A build's stamp and the files whose change should take it again.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stamp {
    /// The commit, perhaps followed by `; dirty`, or [`NO_COMMIT`].
    pub value: String,
    /// The git files whose change makes the stamp stale, absolute.
    pub watched: Vec<PathBuf>,
}

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

/// The stamp for a crate whose manifest is in `dir`.
pub fn stamp(dir: &Path) -> Stamp {
    let tracked = git(dir, &["ls-files", "--error-unmatch", "Cargo.toml"]).is_some();
    let commit = git(dir, &["rev-parse", "HEAD"])
        .filter(|commit| commit.len() == 40 && commit.bytes().all(|b| b.is_ascii_hexdigit()));
    let Some(commit) = commit.filter(|_| tracked) else {
        return Stamp {
            value: NO_COMMIT.to_string(),
            watched: Vec::new(),
        };
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
            watched.push(PathBuf::from(found));
        }
    }
    let dirty = git(dir, &["status", "--porcelain"]).is_some_and(|changes| !changes.is_empty());
    let value = if dirty {
        format!("{commit}; dirty")
    } else {
        commit
    };
    Stamp { value, watched }
}

/// The lines a build script prints for `stamp`: rerun when the build
/// script or any watched file that exists changes, and `LYS_BUILD`.
pub fn cargo_lines(stamp: &Stamp) -> Vec<String> {
    let mut lines = vec!["cargo:rerun-if-changed=build.rs".to_string()];
    for path in stamp.watched.iter().filter(|path| path.exists()) {
        lines.push(format!("cargo:rerun-if-changed={}", path.display()));
    }
    lines.push(format!("cargo:rustc-env=LYS_BUILD={}", stamp.value));
    lines
}

/// The whole of a stamped crate's `build.rs`: stamps the crate whose
/// manifest directory cargo names and prints its lines.
pub fn emit() {
    let dir = std::env::var_os("CARGO_MANIFEST_DIR").map_or_else(|| ".".into(), PathBuf::from);
    for line in cargo_lines(&stamp(&dir)) {
        println!("{line}");
    }
}
