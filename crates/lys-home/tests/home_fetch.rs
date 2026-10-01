//! `lys-home fetch` run as the binary (HOME-019 R6): the shipped ref
//! arrives in a new home byte for byte, each session gains one arrival
//! beside its head with an execution id of its own, committed as one
//! commit on the fetched one and carried onward by a later ship; every
//! refusal before a write leaves the target as it was, and every refusal
//! after one removes exactly what fetch created. Every remote is a path in
//! a temporary directory; no test reaches the network. No test name
//! carries a line of the fixture session.

use lys_home::Hash;
use lys_home::Home;
use lys_home::record::tracked::tracked_set;
use serde_json::Value;
use std::collections::BTreeMap;
use std::error::Error;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::process::Output;

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
const STAGE_THREE: &str =
    "shipping off this machine waits for stage 3's encryption from the secrets step";

type Gate = Result<(), Box<dyn Error>>;

#[path = "home_fetch/arrival.rs"]
mod arrival;
#[path = "home_fetch/fixture.rs"]
mod fixture;
#[path = "home_fetch/refusals.rs"]
mod refusals;

fn text(path: &Path) -> Result<&str, Box<dyn Error>> {
    Ok(path.to_str().ok_or("a UTF-8 path")?)
}

/// Run lys-home with HOME a fresh directory under `dir`, no config
/// directory of this machine, the fixture secret in its environment, and
/// `path` as its PATH when one is given.
fn lys_home_with(dir: &Path, path: Option<&str>, args: &[&str]) -> Result<Output, Box<dyn Error>> {
    let user_home = dir.join("user-home");
    std::fs::create_dir_all(&user_home)?;
    let mut command = Command::new(BIN);
    command
        .current_dir(dir)
        .env("HOME", &user_home)
        .env_remove("CLAUDE_CONFIG_DIR")
        .env("LYS_FIXTURE_TOKEN", SECRET_VALUE)
        .args(args);
    if let Some(path) = path {
        command.env("PATH", path);
    }
    Ok(command.output()?)
}

fn lys_home(dir: &Path, args: &[&str]) -> Result<Output, Box<dyn Error>> {
    lys_home_with(dir, None, args)
}

/// Plain git for the test's own checks and setup, reading no configuration
/// of this machine.
fn plain_git(args: &[&str]) -> Result<Output, Box<dyn Error>> {
    Ok(Command::new("git")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_AUTHOR_NAME", "fixture")
        .env("GIT_AUTHOR_EMAIL", "fixture@invalid")
        .env("GIT_COMMITTER_NAME", "fixture")
        .env("GIT_COMMITTER_EMAIL", "fixture@invalid")
        .args(args)
        .output()?)
}

/// Plain git's stdout, refusing a non-zero exit.
fn git_bytes(args: &[&str]) -> Result<Vec<u8>, Box<dyn Error>> {
    let output = plain_git(args)?;
    if !output.status.success() {
        return Err(format!("git {} exited {:?}", args.join(" "), output.status.code()).into());
    }
    Ok(output.stdout)
}

fn git_text(args: &[&str]) -> Result<String, Box<dyn Error>> {
    Ok(String::from_utf8(git_bytes(args)?)?.trim().to_owned())
}

/// A fixture home under `dir/name`: the launch session as `fixture`, then
/// one render-launch of the launch template.
fn fixture_home(dir: &Path, name: &str) -> Result<PathBuf, Box<dyn Error>> {
    let (root, rendered) = fixture::home(dir, name)?;
    assert_eq!(rendered, Some(0));
    Ok(root)
}

/// Ship `home` to `remote` and return the commit.
fn ship(dir: &Path, home: &Path, remote: &Path) -> Result<String, Box<dyn Error>> {
    let (commit, status) = fixture::ship(dir, home, remote)?;
    assert_eq!(status, Some(0));
    Ok(commit)
}

fn fetch(dir: &Path, remote: &str, target: &Path) -> Result<Output, Box<dyn Error>> {
    lys_home(dir, &["fetch", "--remote", remote, "--home", text(target)?])
}

/// A shipped fixture home and its fetch into `<dir>/target`: the source,
/// the remote, the ship's commit and the fetch report.
struct Moved {
    source: PathBuf,
    remote: PathBuf,
    target: PathBuf,
    commit: String,
    report: Value,
}

fn moved(dir: &Path) -> Result<Moved, Box<dyn Error>> {
    let source = fixture_home(dir, "source")?;
    let remote = dir.join("remote.git");
    let commit = ship(dir, &source, &remote)?;
    let target = dir.join("target");
    let output = fetch(dir, text(&remote)?, &target)?;
    assert_eq!(output.status.code(), Some(0));
    let value: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(value["command"], "fetch");
    Ok(Moved {
        source,
        remote,
        target,
        commit,
        report: value["report"].clone(),
    })
}

fn sha256_of(path: &Path) -> Result<String, Box<dyn Error>> {
    Ok(Hash::of(&std::fs::read(path)?).as_str().to_owned())
}

/// Every file under `dir`, recursively, relative to `dir`, with its SHA-256.
fn files_under(dir: &Path) -> Result<BTreeMap<String, String>, Box<dyn Error>> {
    let mut out = BTreeMap::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(next) = pending.pop() {
        if !next.is_dir() {
            continue;
        }
        for entry in std::fs::read_dir(&next)? {
            let path = entry?.path();
            if path.is_dir() {
                pending.push(path);
            } else {
                let relative = path.strip_prefix(dir)?.to_string_lossy().into_owned();
                out.insert(relative, sha256_of(&path)?);
            }
        }
    }
    Ok(out)
}

/// Every path under `dir`, directories included, with each file's SHA-256.
fn listing(dir: &Path) -> Result<BTreeMap<String, String>, Box<dyn Error>> {
    let mut out = BTreeMap::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(next) = pending.pop() {
        for entry in std::fs::read_dir(&next)? {
            let path = entry?.path();
            let relative = path.strip_prefix(dir)?.to_string_lossy().into_owned();
            if path.is_dir() {
                pending.push(path);
                out.insert(relative, String::new());
            } else {
                out.insert(relative, sha256_of(&path)?);
            }
        }
    }
    Ok(out)
}

/// The last line of a file, parsed.
fn last_line(bytes: &[u8]) -> Result<Value, Box<dyn Error>> {
    let text = std::str::from_utf8(bytes)?;
    let line = text.lines().last().ok_or("a last line")?;
    Ok(serde_json::from_str(line)?)
}

/// Copy a home's stores and sessions, never its repository, into `to`.
fn copy_home(from: &Path, to: &Path) -> Gate {
    Ok(fixture::copy_tree(from, to, false)?)
}

/// Whether `dir` is a directory that holds nothing.
fn is_empty_dir(dir: &Path) -> Result<bool, Box<dyn Error>> {
    Ok(dir.is_dir() && std::fs::read_dir(dir)?.next().is_none())
}

#[test]
fn a_fetch_into_an_absent_target_arrives_on_the_shipped_commit() -> Gate {
    let dir = tempfile::tempdir()?;
    let m = moved(dir.path())?;
    assert_eq!(m.report["commit"], m.commit.as_str());
    assert_eq!(m.report["ref"], "refs/lys/home");
    let parent = git_text(&["-C", text(&m.target)?, "rev-parse", "HEAD^1"])?;
    assert_eq!(parent, m.commit);
    Ok(())
}

#[test]
fn the_fetched_commit_holds_every_tracked_file_of_the_source_byte_for_byte() -> Gate {
    let dir = tempfile::tempdir()?;
    let m = moved(dir.path())?;
    let target = text(&m.target)?;
    let tracked = tracked_set(&Home::read(&m.source)?)?;
    let mut compared = 0;
    for path in &tracked {
        let blob = git_bytes(&[
            "-C",
            target,
            "cat-file",
            "blob",
            &format!("{}:{path}", m.commit),
        ])?;
        assert_eq!(
            Hash::of(&blob).as_str(),
            sha256_of(&m.source.join(path))?,
            "{path}"
        );
        compared += 1;
    }
    assert_eq!(compared, tracked.len());
    let listed = git_text(&["-C", target, "ls-tree", "-r", "--name-only", &m.commit])?;
    let listed: Vec<String> = listed.lines().map(str::to_owned).collect();
    assert_eq!(listed, tracked);
    assert!(listed.iter().any(|p| p.starts_with("templates/")));
    Ok(())
}

#[test]
fn the_head_blocks_and_templates_arrive_byte_identical() -> Gate {
    let dir = tempfile::tempdir()?;
    let m = moved(dir.path())?;
    let head = Path::new("sessions").join("fixture.head");
    assert_eq!(
        std::fs::read(m.target.join(&head))?,
        std::fs::read(m.source.join(&head))?
    );
    let mut stores = 0;
    for store in ["blocks", "templates"] {
        let source = files_under(&m.source.join(store))?;
        let target = files_under(&m.target.join(store))?;
        assert!(!source.is_empty(), "{store}");
        assert_eq!(target.len(), source.len(), "{store}");
        assert_eq!(target, source, "{store}");
        stores += 1;
    }
    assert_eq!(stores, 2);
    Ok(())
}

#[test]
fn a_fetch_writes_nothing_to_the_source_or_the_remote() -> Gate {
    let dir = tempfile::tempdir()?;
    let source = fixture_home(dir.path(), "source")?;
    let remote = dir.path().join("remote.git");
    let commit = ship(dir.path(), &source, &remote)?;
    let snapshot = |home: &Path| -> Result<BTreeMap<String, String>, Box<dyn Error>> {
        let mut all = BTreeMap::new();
        for store in ["sessions", "blocks", "templates"] {
            for (relative, hash) in files_under(&home.join(store))? {
                all.insert(format!("{store}/{relative}"), hash);
            }
        }
        Ok(all)
    };
    let before = snapshot(&source)?;
    assert!(!before.is_empty());
    let output = fetch(dir.path(), text(&remote)?, &dir.path().join("target"))?;
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(snapshot(&source)?, before);
    assert_eq!(
        git_text(&["--git-dir", text(&remote)?, "rev-parse", "refs/lys/home"])?,
        commit
    );
    Ok(())
}
