//! The move end to end through the binary (HOME-019 R7): a fixture home
//! rendered, shipped and fetched with the fixture secret's value in the
//! environment of every lys-home process, then every object the shipped
//! ref reaches read and searched for that value, with a control that finds
//! it where it was planted; and a render of the fetched session recording
//! the same session head as a render of the source, taken only after the
//! source's tracked files are asserted unchanged. The printed launch line
//! is never run here (ADR-007). Every remote is a path in a temporary
//! directory; no test reaches the network.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;

use lys_home::record::tracked::tracked_set;
use lys_home::{Hash, Home};

const BIN: &str = env!("CARGO_BIN_EXE_lys-home");
const TEMPLATE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/launch/template.json"
);
const SESSION: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/launch/session.jsonl"
);
const UUID: &str = "00000000-0000-4000-8000-000000000019";
const SECRET_VALUE: &str = "fixture-secret-value-0001";

type Gate = Result<(), Box<dyn Error>>;

fn text(path: &Path) -> Result<&str, Box<dyn Error>> {
    Ok(path.to_str().ok_or("a UTF-8 path")?)
}

/// Run lys-home with HOME a fresh directory under `dir`, no config
/// directory of this machine, and the fixture secret in its environment.
fn lys_home(dir: &Path, args: &[&str]) -> Result<Output, Box<dyn Error>> {
    let user_home = dir.join("user-home");
    std::fs::create_dir_all(&user_home)?;
    Ok(Command::new(BIN)
        .current_dir(dir)
        .env("HOME", &user_home)
        .env_remove("CLAUDE_CONFIG_DIR")
        .env("LYS_FIXTURE_TOKEN", SECRET_VALUE)
        .args(args)
        .output()?)
}

/// Run lys-home, refuse a non-zero exit, and return its one JSON report.
fn report(dir: &Path, args: &[&str]) -> Result<Value, Box<dyn Error>> {
    let output = lys_home(dir, args)?;
    if output.status.code() != Some(0) {
        return Err(format!("lys-home {} exited {:?}", args[0], output.status.code()).into());
    }
    Ok(serde_json::from_slice(&output.stdout)?)
}

/// Plain git's stdout, reading no configuration of this machine and
/// refusing a non-zero exit.
fn git_bytes(args: &[&str], input: Option<&[u8]>) -> Result<Vec<u8>, Box<dyn Error>> {
    use std::io::Write;
    let mut child = Command::new("git")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_AUTHOR_NAME", "fixture")
        .env("GIT_AUTHOR_EMAIL", "fixture@invalid")
        .env("GIT_COMMITTER_NAME", "fixture")
        .env("GIT_COMMITTER_EMAIL", "fixture@invalid")
        .args(args)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(input.unwrap_or_default())?;
    }
    let output = child.wait_with_output()?;
    if !output.status.success() {
        return Err(format!("git {} exited {:?}", args.join(" "), output.status.code()).into());
    }
    Ok(output.stdout)
}

fn git_text(args: &[&str], input: Option<&[u8]>) -> Result<String, Box<dyn Error>> {
    Ok(String::from_utf8(git_bytes(args, input)?)?.trim().to_owned())
}

fn render(dir: &Path, home: &Path, out: &Path) -> Result<Value, Box<dyn Error>> {
    std::fs::create_dir(out)?;
    report(
        dir,
        &[
            "render-launch",
            "--home",
            text(home)?,
            "--session",
            "fixture",
            "--template",
            TEMPLATE,
            "--uuid",
            UUID,
            "--cwd",
            "/fixture",
            "--model",
            "claude-fixture",
            "--version",
            "2.1.283",
            "--out",
            text(out)?,
        ],
    )
}

/// What a search of every object a ref reaches found: how many objects it
/// read and how many held the needle.
struct Search {
    read: usize,
    matches: usize,
}

/// Read every object `git rev-list --objects <ref>` lists through
/// `git cat-file`, and count those whose bytes hold `needle`.
fn search(git_dir: &Path, git_ref: &str, needle: &[u8]) -> Result<Search, Box<dyn Error>> {
    let dir = text(git_dir)?;
    let listed = git_text(&["--git-dir", dir, "rev-list", "--objects", git_ref], None)?;
    let mut read = 0;
    let mut matches = 0;
    for line in listed.lines() {
        let id = line.split(' ').next().ok_or("an object id")?;
        let kind = git_text(&["--git-dir", dir, "cat-file", "-t", id], None)?;
        let bytes = git_bytes(&["--git-dir", dir, "cat-file", &kind, id], None)?;
        read += 1;
        if bytes.windows(needle.len()).any(|w| w == needle) {
            matches += 1;
        }
    }
    Ok(Search { read, matches })
}

fn tracked_hashes(home: &Path) -> Result<BTreeMap<String, String>, Box<dyn Error>> {
    let mut out = BTreeMap::new();
    for path in tracked_set(&Home::read(home)?)? {
        let bytes = std::fs::read(home.join(&path))?;
        out.insert(path, Hash::of(&bytes).as_str().to_owned());
    }
    Ok(out)
}

/// A fixture home rendered, shipped and fetched.
struct Moved {
    /// The source home.
    source: PathBuf,
    /// The remote it was shipped to.
    remote: PathBuf,
    /// The home it was fetched into.
    target: PathBuf,
    /// The source's tracked hashes, taken before the ship.
    before: BTreeMap<String, String>,
}

fn moved(dir: &Path) -> Result<Moved, Box<dyn Error>> {
    let source = dir.join("source");
    let home = Home::open(&source)?;
    std::fs::copy(SESSION, home.session_path("fixture")?)?;
    render(dir, &source, &dir.join("first-out"))?;
    let before = tracked_hashes(&source)?;
    let remote = dir.join("remote.git");
    report(dir, &["ship", "--home", text(&source)?, "--remote", text(&remote)?])?;
    let target = dir.join("target");
    report(dir, &["fetch", "--remote", text(&remote)?, "--home", text(&target)?])?;
    Ok(Moved {
        source,
        remote,
        target,
        before,
    })
}

#[test]
fn every_object_the_shipped_ref_reaches_is_read_and_none_holds_the_secret() -> Gate {
    let dir = tempfile::tempdir()?;
    let Moved { source, remote, .. } = moved(dir.path())?;
    let found = search(&remote, "refs/lys/home", SECRET_VALUE.as_bytes())?;
    let tracked = tracked_set(&Home::read(&source)?)?;
    let mut directories = BTreeSet::new();
    for path in &tracked {
        let mut prefix = String::new();
        let parts: Vec<&str> = path.split('/').collect();
        for part in &parts[..parts.len() - 1] {
            if !prefix.is_empty() {
                prefix.push('/');
            }
            prefix.push_str(part);
            directories.insert(prefix.clone());
        }
    }
    let expected = tracked.len() + directories.len() + 1 + 1;
    assert_eq!(found.read, expected);
    assert_eq!(found.matches, 0);
    Ok(())
}

#[test]
fn the_same_search_finds_the_secret_once_where_it_was_planted() -> Gate {
    let dir = tempfile::tempdir()?;
    let scratch = dir.path().join("scratch.git");
    let scratch_arg = text(&scratch)?;
    git_bytes(&["init", "--quiet", "--bare", scratch_arg], None)?;
    let blob = git_text(
        &["--git-dir", scratch_arg, "hash-object", "-w", "--stdin"],
        Some(b"token=fixture-secret-value-0001"),
    )?;
    let entry = format!("100644 blob {blob}\tplanted.txt\n");
    let tree = git_text(&["--git-dir", scratch_arg, "mktree"], Some(entry.as_bytes()))?;
    let commit = git_text(
        &["--git-dir", scratch_arg, "commit-tree", &tree, "-m", "planted"],
        None,
    )?;
    git_bytes(
        &["--git-dir", scratch_arg, "update-ref", "refs/lys/home", &commit],
        None,
    )?;
    let found = search(&scratch, "refs/lys/home", SECRET_VALUE.as_bytes())?;
    assert_eq!(found.read, 3);
    assert_eq!(found.matches, 1);
    Ok(())
}

#[test]
fn a_render_of_the_fetched_session_records_the_source_session_head() -> Gate {
    let dir = tempfile::tempdir()?;
    let Moved {
        source,
        target,
        before,
        ..
    } = moved(dir.path())?;
    assert!(!before.is_empty());
    assert_eq!(tracked_hashes(&source)?, before);
    let target_out = dir.path().join("target-out");
    let from_target = render(dir.path(), &target, &target_out)?;
    let from_source = render(dir.path(), &source, &dir.path().join("source-out"))?;
    let head = from_target["session_head"].as_str().ok_or("a session head")?;
    assert_eq!(head.len(), 64);
    assert_eq!(from_source["session_head"], head);
    let launch = from_target["launch"].as_str().ok_or("a launch line")?;
    let resumed = format!("--resume {}/{UUID}.jsonl", target_out.display());
    assert!(launch.contains(&resumed), "{launch}");
    Ok(())
}
