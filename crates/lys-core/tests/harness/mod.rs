//! Shared Go-toolchain harness for the conformance gates.
//!
//! Both `cose_conformance` and `bundle_conformance` shell out to the same
//! vendored tool in `tests/cose-conformance`, and the part that must never
//! drift between them is the **environment contract**: every invocation runs
//! with `GOFLAGS=-mod=vendor GOPROXY=off GOTOOLCHAIN=local` and a throwaway
//! `GOCACHE`, so the gates need zero network and cannot silently pick up a
//! different dependency than the vendored, pinned one. Two copies of that
//! contract is two chances for one of them to lose a flag, which is why it
//! lives here once.
//!
//! The skip policy is the other shared invariant. A missing Go toolchain is a
//! developer-machine skip; a missing toolchain with `LYS_REQUIRE_GO` set is a
//! hard failure, so a conformance gate can never quietly degrade to "passed" in
//! CI. A toolchain that is present but *broken* is always a hard failure —
//! [`build_go_tool`] panics rather than treating it as absent.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::ffi::OsStr;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Locates the Go toolchain: `LYS_GO_BIN` override, then the pinned absolute
/// path, then `go` on `PATH`. `None` means no usable toolchain was found.
fn find_go() -> Option<PathBuf> {
    if let Ok(overridden) = std::env::var("LYS_GO_BIN") {
        return Some(PathBuf::from(overridden));
    }
    let pinned = Path::new("/usr/local/go/bin/go");
    if pinned.exists() {
        return Some(pinned.to_path_buf());
    }
    let on_path = Command::new("go")
        .arg("version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    match on_path {
        Ok(status) if status.success() => Some(PathBuf::from("go")),
        _ => None,
    }
}

/// Resolves the Go toolchain, or returns `None` after announcing the skip.
///
/// `gate` names the gate in the skip notice and in the failure message.
///
/// # Panics
///
/// Panics if no toolchain is found while `LYS_REQUIRE_GO` is set — in that
/// environment the gate is required, and skipping it would report a pass for a
/// cross-check that never ran.
pub fn go_or_skip(gate: &str) -> Option<PathBuf> {
    if let Some(go) = find_go() {
        return Some(go);
    }
    assert!(
        std::env::var_os("LYS_REQUIRE_GO").is_none(),
        "LYS_REQUIRE_GO is set but no Go toolchain was found — \
         the {gate} gate must not skip in this environment"
    );
    eprintln!("skipping {gate}: no Go toolchain found");
    None
}

/// Compiles the Go tool once, to `out`.
///
/// Every gate uses the built binary rather than `go run .`, which re-links on
/// each invocation: one spawn path shared by all three gates, and a sweep of
/// hundreds of cases costs seconds instead of minutes.
///
/// `go run .` re-links on every invocation, which turns a many-case sweep into
/// minutes of linking; a single build plus fast execs costs seconds.
///
/// # Panics
///
/// Panics if the toolchain cannot be spawned or the build fails. A toolchain
/// that is present but broken is deliberately a hard failure, never a skip.
pub fn build_go_tool(go: &Path, gocache: &Path, out: &Path) {
    let scaffold_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/cose-conformance");
    let status = Command::new(go)
        .arg("build")
        .arg("-o")
        .arg(out)
        .arg(".")
        .current_dir(&scaffold_dir)
        .env("GOFLAGS", "-mod=vendor")
        .env("GOPROXY", "off")
        .env("GOTOOLCHAIN", "local")
        .env("GOCACHE", gocache)
        .status()
        .expect("failed to spawn the Go toolchain (present but broken is a hard failure)");
    assert!(status.success(), "go build of the conformance tool failed");
}

/// Runs the pre-built tool with `input` on stdin; returns `(exit_success,
/// stdout_bytes)`.
///
/// The tool's stderr is inherited, so a rejection explains itself in the test
/// output — which matters because every negative case here asserts only that
/// the tool refused, and the reason is the evidence it refused for the right
/// one.
///
/// # Panics
///
/// Panics if the built tool cannot be spawned or its stdin cannot be written.
pub fn run_built_tool<S: AsRef<OsStr>>(bin: &Path, args: &[S], input: &[u8]) -> (bool, Vec<u8>) {
    let mut child = Command::new(bin)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("failed to spawn the built conformance tool");
    child
        .stdin
        .take()
        .expect("child stdin is piped")
        .write_all(input)
        .expect("failed to write to the tool's stdin");
    let output = child
        .wait_with_output()
        .expect("failed to wait for the tool");
    (output.status.success(), output.stdout)
}
