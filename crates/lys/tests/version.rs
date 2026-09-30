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

/// Exercises the stamp without the workspace's dependencies.
fn probe(
    tree: &std::path::Path,
    name: &str,
    commit: Option<&std::ffi::OsStr>,
) -> Result<std::process::Output, Box<dyn Error>> {
    if !tree.join("Cargo.toml").exists() {
        std::fs::create_dir_all(tree.join("src"))?;
        std::fs::write(
            tree.join("Cargo.toml"),
            "[package]\nname = \"stamp-probe\"\nversion = \"0.0.0\"\nedition = \"2024\"\n\n[workspace]\n",
        )?;
        std::fs::copy(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("..")
                .join(name)
                .join("build.rs"),
            tree.join("build.rs"),
        )?;
        std::fs::write(
            tree.join("src").join("main.rs"),
            "fn main() {\n    println!(\"{}\", env!(\"LYS_BUILD\"));\n}\n",
        )?;
    }
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let mut command = Command::new(cargo);
    command
        .args(["run", "--quiet", "--offline"])
        .current_dir(tree)
        .env_remove("LYS_BUILD_COMMIT")
        .env("GIT_CEILING_DIRECTORIES", tree.parent().ok_or("no parent")?);
    if let Some(commit) = commit {
        command.env("LYS_BUILD_COMMIT", commit);
    }
    Ok(command.output()?)
}

fn built(output: std::process::Output) -> Result<String, Box<dyn Error>> {
    assert!(
        output.status.success(),
        "the probe did not build: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(String::from_utf8(output.stdout)?.trim().to_string())
}

fn refused(output: &std::process::Output) {
    assert!(!output.status.success(), "an invalid stated commit built");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("LYS_BUILD_COMMIT"),
        "the refusal did not name its input: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn git(tree: &std::path::Path, args: &[&str]) -> Result<String, Box<dyn Error>> {
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
        .current_dir(tree)
        .env("GIT_CEILING_DIRECTORIES", tree.parent().ok_or("no parent")?)
        .output()?;
    assert!(output.status.success(), "git {args:?} failed");
    Ok(String::from_utf8(output.stdout)?.trim().to_string())
}

fn committed(tree: &std::path::Path, name: &str) -> Result<String, Box<dyn Error>> {
    std::fs::create_dir_all(tree)?;
    git(tree, &["init", "--quiet"])?;
    std::fs::write(tree.join(".gitignore"), "/target\n")?;
    built(probe(tree, name, None)?)?;
    git(tree, &["add", "."])?;
    git(tree, &["commit", "--quiet", "-m", "probe"])?;
    let commit = git(tree, &["rev-parse", "HEAD"])?;
    assert_eq!(commit.len(), 40);
    Ok(commit)
}

const STATED: &str = "0123456789abcdef0123456789abcdef01234567";
const OTHER: &str = "abcdef0123456789abcdef0123456789abcdef01";

#[test]
fn a_build_from_an_exported_tree_says_it_has_no_commit() -> TestResult {
    for name in STAMPED {
        let scratch = tempfile::tempdir()?;
        let tree = scratch.path().join("exported");
        assert_eq!(built(probe(&tree, name, None)?)?, NO_COMMIT, "{name}");
        assert!(!tree.join(".git").exists());
    }
    Ok(())
}

#[test]
fn an_export_names_a_stated_commit_and_rebuilds_when_it_changes() -> TestResult {
    for name in STAMPED {
        let scratch = tempfile::tempdir()?;
        let tree = scratch.path().join("exported");
        for commit in [STATED, OTHER] {
            assert_eq!(
                built(probe(&tree, name, Some(commit.as_ref()))?)?,
                format!("{commit}; stated"),
                "{name}"
            );
        }
        assert_eq!(built(probe(&tree, name, None)?)?, NO_COMMIT, "{name}");
    }
    Ok(())
}

#[test]
fn an_invalid_stated_commit_is_refused_for_every_binary() -> TestResult {
    for name in STAMPED {
        let scratch = tempfile::tempdir()?;
        let tree = scratch.path().join("exported");
        for commit in [
            "",
            &STATED[..39],
            &format!("{STATED}0"),
            &STATED.to_uppercase(),
            &format!("{STATED}\n"),
            "gggggggggggggggggggggggggggggggggggggggg",
        ] {
            refused(&probe(&tree, name, Some(commit.as_ref()))?);
        }
    }
    Ok(())
}

#[test]
fn a_build_from_a_commit_names_it_and_its_dirty_state() -> TestResult {
    let scratch = tempfile::tempdir()?;
    let tree = scratch.path().join("committed");
    let commit = committed(&tree, "lys")?;
    assert_eq!(built(probe(&tree, "lys", None)?)?, commit);
    std::fs::write(tree.join("src").join("extra.txt"), "a change\n")?;
    git(&tree, &["add", "src/extra.txt"])?;
    assert_eq!(
        built(probe(&tree, "lys", None)?)?,
        format!("{commit}; dirty")
    );
    Ok(())
}

#[test]
fn a_stated_commit_must_agree_with_git_for_every_binary() -> TestResult {
    for name in STAMPED {
        let scratch = tempfile::tempdir()?;
        let tree = scratch.path().join("committed");
        let commit = committed(&tree, name)?;
        assert_eq!(
            built(probe(&tree, name, Some(commit.as_ref()))?)?,
            format!("{commit}; stated"),
            "{name}"
        );
        let refusal = probe(&tree, name, Some(STATED.as_ref()))?;
        refused(&refusal);
        assert!(String::from_utf8_lossy(&refusal.stderr).contains("disagrees"));
        std::fs::write(tree.join("src").join("extra.txt"), "a change\n")?;
        git(&tree, &["add", "src/extra.txt"])?;
        assert_eq!(
            built(probe(&tree, name, Some(commit.as_ref()))?)?,
            format!("{commit}; stated; dirty"),
            "{name}"
        );
    }
    Ok(())
}

#[cfg(unix)]
#[test]
fn a_non_unicode_stated_commit_is_refused_for_every_binary() -> TestResult {
    use std::os::unix::ffi::OsStrExt;

    for name in STAMPED {
        let scratch = tempfile::tempdir()?;
        refused(&probe(
            &scratch.path().join("exported"),
            name,
            Some(std::ffi::OsStr::from_bytes(&[0xff])),
        )?);
    }
    Ok(())
}
