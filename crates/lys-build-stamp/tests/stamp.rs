#![cfg(test)]
//! The build stamp's three forms, each read from a crate built through a
//! `build.rs` that is the one call every Lys binary makes, and the four
//! stamped crates held to that one call.
//!
//! The expected commit comes from git, run here as a second party to the
//! stamp the build script took.

use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::Command;

use lys_build_stamp::NO_COMMIT;

type TestResult = Result<(), Box<dyn Error>>;

/// The crates whose binaries carry the stamp.
const STAMPED: [&str; 4] = ["lys", "lys-identity-server", "lys-secrets", "lys-home"];

/// The whole of a stamped crate's build script, comments aside.
const ONE_CALL: &str = "fn main() {\nlys_build_stamp::emit();\n}";

fn crates_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

/// `text` without its comment lines, each line trimmed, blank lines dropped.
fn code_of(text: &str) -> String {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn every_stamped_crate_calls_the_one_stamp_and_holds_none_of_its_logic() -> TestResult {
    let mut checked = 0;
    for name in STAMPED {
        let dir = crates_dir().join(name);
        let script = std::fs::read_to_string(dir.join("build.rs"))?;
        assert_eq!(code_of(&script), ONE_CALL, "{name}/build.rs is not the one call");
        let manifest = std::fs::read_to_string(dir.join("Cargo.toml"))?;
        let section = manifest
            .split("\n[build-dependencies]\n")
            .nth(1)
            .ok_or_else(|| format!("{name} lists no build-dependencies"))?;
        let listed = section
            .lines()
            .take_while(|line| !line.starts_with('['))
            .any(|line| line.trim() == "lys-build-stamp.workspace = true");
        assert!(listed, "{name} does not build with lys-build-stamp");
        checked += 1;
    }
    assert_eq!(checked, STAMPED.len());
    Ok(())
}

/// Builds, in `tree`, a crate whose build script is `lys`'s own, and
/// returns what the built binary says its build is.
fn probe(tree: &Path) -> Result<String, Box<dyn Error>> {
    std::fs::create_dir_all(tree.join("src"))?;
    let stamp_crate = Path::new(env!("CARGO_MANIFEST_DIR")).canonicalize()?;
    let manifest = format!(
        "[package]\nname = \"stamp-probe\"\nversion = \"0.0.0\"\nedition = \"2024\"\n\n\
         [build-dependencies]\nlys-build-stamp = {{ path = {:?} }}\n\n[workspace]\n",
        stamp_crate.display().to_string()
    );
    std::fs::write(tree.join("Cargo.toml"), manifest)?;
    std::fs::copy(crates_dir().join("lys").join("build.rs"), tree.join("build.rs"))?;
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
fn a_build_from_a_fresh_commit_names_it_and_a_change_marks_it_dirty() -> TestResult {
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
    std::fs::write(tree.join(".gitignore"), "/target\nCargo.lock\n")?;
    // The probe writes its files, then they are committed and built clean.
    probe(&tree)?;
    git(&["add", "."])?;
    git(&["commit", "--quiet", "-m", "probe"])?;
    let commit = git(&["rev-parse", "HEAD"])?;
    assert_eq!(commit.len(), 40);
    assert_eq!(probe(&tree)?, commit, "a clean build of a fresh commit");
    std::fs::write(tree.join("src").join("extra.txt"), "a change\n")?;
    git(&["add", "src/extra.txt"])?;
    assert_eq!(probe(&tree)?, format!("{commit}; dirty"), "a build with a change");
    Ok(())
}

#[test]
fn the_cargo_lines_rerun_on_the_script_and_carry_the_value() {
    let stamp = lys_build_stamp::Stamp {
        value: NO_COMMIT.to_string(),
        watched: vec![PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml")],
    };
    let lines = lys_build_stamp::cargo_lines(&stamp);
    assert_eq!(lines.len(), 3, "{lines:?}");
    assert_eq!(lines[0], "cargo:rerun-if-changed=build.rs");
    assert!(lines[1].ends_with("Cargo.toml"), "{lines:?}");
    assert_eq!(lines[2], format!("cargo:rustc-env=LYS_BUILD={NO_COMMIT}"));
}
