//! Go-toolchain harness for `lys-anchor`'s conformance gates.
//!
//! # The Go scaffolds are `lys-core`'s, both of them
//!
//! This harness drives the *existing* vendored tools at
//! `crates/lys-core/tests/go-conformance` — the one that wraps
//! `golang.org/x/mod/sumdb/note` — and `crates/lys-core/tests/cose-conformance`,
//! which wraps `github.com/veraison/go-cose`. A second `go.mod` and a second
//! vendor tree would be a second pinned copy of a reference implementation, and
//! two copies of a reference are two things that can disagree about what the
//! reference says. So nothing here is vendored; only the spawn is local.
//!
//! Which scaffold a gate wants is a [`GoScaffold`], not a path a call site
//! writes out: a mistyped directory would be a `go build` failure rather than a
//! silent pass, but a *closed set* means a new gate cannot invent a third
//! vendored copy by accident either.
//!
//! # The duplication that remains, named rather than left to be noticed
//!
//! What *is* duplicated is the environment contract: `GOFLAGS=-mod=vendor`,
//! `GOPROXY=off`, `GOTOOLCHAIN=local`, and a throwaway `GOCACHE`, so a gate
//! needs no network and cannot silently resolve a different dependency than the
//! vendored, pinned one. `lys-core`'s own harness says why that contract lives
//! in one place: two copies is two chances for one of them to lose a flag.
//! There are now two copies, so:
//!
//! - **This side is checked by data, not by wording.** [`GO_ENV`] is the array
//!   actually passed to every spawn, and the test below asserts over that same
//!   array. A flag dropped from the spawn is a flag dropped from the assertion
//!   target.
//! - **The other side is checked lexically**, against `lys-core`'s harness
//!   source, so if that contract changes the divergence surfaces here as a
//!   failure rather than as two gates quietly running under different rules.
//! - **This side is also checked behaviourally**, for free: losing
//!   `-mod=vendor` while `GOPROXY=off` stands means the module cannot resolve
//!   at all and the gate fails hard. It cannot degrade to a pass.
//!
//! # The skip policy
//!
//! A missing Go toolchain is a developer-machine skip. A missing toolchain with
//! `LYS_REQUIRE_GO` set is a hard failure, so a conformance gate can never
//! quietly report a pass for a cross-check that never ran. A toolchain that is
//! present but *broken* is always a hard failure.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::ffi::OsStr;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// The hermeticity contract, as the data every spawn actually uses.
///
/// `GOCACHE` is not here because its value is a per-run temporary directory
/// rather than a constant; it is set alongside these on every spawn.
pub const GO_ENV: [(&str, &str); 3] = [
    ("GOFLAGS", "-mod=vendor"),
    ("GOPROXY", "off"),
    ("GOTOOLCHAIN", "local"),
];

/// Which vendored Go scaffold a gate needs — `lys-core`'s, deliberately not a
/// second copy of either.
#[derive(Clone, Copy, Debug)]
pub enum GoScaffold {
    /// `go-conformance`: `golang.org/x/mod/sumdb/note`, the C2SP signed-note
    /// reference. Judges checkpoints.
    Note,
    /// `cose-conformance`: `github.com/veraison/go-cose` plus RFC 6962's
    /// *recursive* `MTH`/`PATH` transcribed in Go. Judges receipts.
    Cose,
}

/// Every scaffold a gate may ask for. The closed set, as data, so the test
/// below checks all of them rather than the ones someone remembered.
pub const ALL_SCAFFOLDS: [GoScaffold; 2] = [GoScaffold::Note, GoScaffold::Cose];

impl GoScaffold {
    /// The scaffold's directory, relative to this crate's manifest.
    fn dir(self) -> PathBuf {
        let name = match self {
            Self::Note => "go-conformance",
            Self::Cose => "cose-conformance",
        };
        Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../lys-core/tests/{name}"))
    }
}

/// `lys-core`'s harness source, read to detect contract drift between the two.
fn lys_core_harness() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../lys-core/tests/harness/mod.rs")
}

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

/// Compiles `scaffold`'s Go tool once, to `out`.
///
/// Built once and exec'd per case rather than `go run .` per case, which
/// re-links every time and turns a sweep into minutes of linking.
///
/// # Panics
///
/// Panics if the toolchain cannot be spawned or the build fails. A toolchain
/// that is present but broken is deliberately a hard failure, never a skip —
/// and so is a scaffold that will not build offline, since `GOPROXY=off` means
/// an unvendored dependency cannot resolve at all rather than being fetched.
pub fn build_go_tool(go: &Path, scaffold: GoScaffold, gocache: &Path, out: &Path) {
    let status = Command::new(go)
        .arg("build")
        .arg("-o")
        .arg(out)
        .arg(".")
        .current_dir(scaffold.dir())
        .envs(GO_ENV)
        .env("GOCACHE", gocache)
        .status()
        .expect("failed to spawn the Go toolchain (present but broken is a hard failure)");
    assert!(status.success(), "go build of the {scaffold:?} tool failed");
}

/// Runs the pre-built tool with `input` on stdin; returns `(exit_success,
/// stdout_bytes)`.
///
/// The tool's stderr is inherited, so a rejection explains itself in the test
/// output — which matters because a negative case asserts only that the tool
/// refused, and the reason is the evidence it refused for the right one.
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
        .expect("failed to spawn the built note tool");
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

#[test]
fn the_go_environment_contract_matches_the_one_lys_core_wrote_down() {
    let path = lys_core_harness();
    let theirs = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("could not read {}: {e}", path.display()));

    // Positive control: the file that was read really is that harness, so the
    // clause assertions below are searching the intended text and not an empty
    // string or some other file that happens to exist.
    assert!(
        theirs.contains("Shared Go-toolchain harness"),
        "{} is not lys-core's conformance harness",
        path.display()
    );

    let mut checked = 0;
    for (key, value) in GO_ENV {
        let clause = format!(r#".env("{key}", "{value}")"#);
        assert!(
            theirs.contains(&clause),
            "lys-core's harness no longer sets {key}={value}; the two Go gates \
             would run under different rules — reconcile them rather than \
             loosening this check"
        );
        checked += 1;
    }
    // Count what fired: an empty contract array would satisfy the loop without
    // comparing anything at all.
    assert_eq!(checked, GO_ENV.len());
    assert_eq!(checked, 3, "the hermeticity contract is three settings");

    assert!(
        theirs.contains(r#".env("GOCACHE", gocache)"#),
        "lys-core's harness no longer uses a throwaway GOCACHE"
    );
}

#[test]
fn every_scaffold_is_vendored_where_this_harness_says_it_is() {
    // `GOPROXY=off` means an unvendored dependency cannot resolve at all, so a
    // de-vendored scaffold is already a hard build failure rather than a silent
    // network fetch. This asserts it by name anyway, because the failure a gate
    // would otherwise show is "go build failed", which reads like a broken
    // toolchain rather than like a missing vendor tree.
    let mut checked = 0;
    for scaffold in ALL_SCAFFOLDS {
        let dir = scaffold.dir();
        for required in ["go.mod", "vendor/modules.txt"] {
            let path = dir.join(required);
            assert!(
                path.exists(),
                "the {scaffold:?} scaffold has no {required} at {} — it cannot \
                 build under GOPROXY=off",
                path.display()
            );
        }
        checked += 1;
    }
    // Count what fired: an empty scaffold set would satisfy the loop while
    // checking nothing.
    assert_eq!(checked, ALL_SCAFFOLDS.len());
    assert_eq!(checked, 2, "there are two vendored scaffolds");
}
