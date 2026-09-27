//! `lys-home ship` run as the binary (HOME-019 R5): the first ship makes the
//! home's repository and the remote and pushes exactly the tracked set; a
//! ship to a bare repository made beforehand; every refusal by name with
//! nothing pushed; an unchanged home pushed again with no new commit; a
//! changed home committed on its last commit; a second home refused as
//! diverged. Every ship runs through `ship_unchanged`, which proves the
//! source home's tracked files and its paths are the same after as before.
//! Every remote is a path in a temporary directory; no test reaches the
//! network. No test name carries a line of the fixture session.

use lys_home::EntryBody;
use lys_home::Hash;
use lys_home::Home;
use lys_home::record::tracked::tracked_set;
use serde_json::Value;
use serde_json::json;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
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

#[path = "home_ship/refusals.rs"]
mod refusals;

fn text(path: &Path) -> Result<&str, Box<dyn Error>> {
    Ok(path.to_str().ok_or("a UTF-8 path")?)
}

/// Run lys-home with HOME a fresh directory under `dir`, no config
/// directory of this machine, and the fixture secret in its environment.
fn lys_home(dir: &Path, cwd: &Path, args: &[&str]) -> Result<Output, Box<dyn Error>> {
    let user_home = dir.join("user-home");
    std::fs::create_dir_all(&user_home)?;
    Ok(Command::new(BIN)
        .current_dir(cwd)
        .env("HOME", &user_home)
        .env_remove("CLAUDE_CONFIG_DIR")
        .env("LYS_FIXTURE_TOKEN", SECRET_VALUE)
        .args(args)
        .output()?)
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

/// Plain git's trimmed stdout, refusing a non-zero exit.
fn git_text(args: &[&str]) -> Result<String, Box<dyn Error>> {
    let output = plain_git(args)?;
    if !output.status.success() {
        return Err(format!("git {} exited {:?}", args.join(" "), output.status.code()).into());
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}

/// A fixture home under `dir/name`: the launch session as `fixture`, then
/// one render-launch of the launch template.
fn fixture_home(dir: &Path, name: &str) -> Result<PathBuf, Box<dyn Error>> {
    let root = dir.join(name);
    let home = Home::open(&root)?;
    std::fs::copy(SESSION, home.session_path("fixture")?)?;
    let out = dir.join(format!("{name}-out"));
    std::fs::create_dir(&out)?;
    let rendered = lys_home(
        dir,
        dir,
        &[
            "render-launch",
            "--home",
            text(&root)?,
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
            text(&out)?,
        ],
    )?;
    assert_eq!(rendered.status.code(), Some(0));
    Ok(root)
}

fn sha256_hex(bytes: &[u8]) -> String {
    Hash::of(bytes).as_str().to_owned()
}

/// Every path under the home's `sessions/`, `blocks/` and `templates/`,
/// other than a lock file, relative to the home.
fn store_paths(home: &Path) -> Result<BTreeSet<String>, Box<dyn Error>> {
    let mut out = BTreeSet::new();
    for store in ["sessions", "blocks", "templates"] {
        walk(home, &home.join(store), &mut out)?;
    }
    Ok(out)
}

fn walk(home: &Path, dir: &Path, out: &mut BTreeSet<String>) -> Gate {
    if !dir.is_dir() {
        return Ok(());
    }
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        let relative = path.strip_prefix(home)?.to_string_lossy().into_owned();
        if relative.ends_with("..lock") {
            continue;
        }
        if path.is_dir() {
            walk(home, &path, out)?;
        }
        out.insert(relative);
    }
    Ok(())
}

/// The SHA-256 of every file of the home's tracked set.
fn tracked_hashes(home: &Path) -> Result<BTreeMap<String, String>, Box<dyn Error>> {
    let mut out = BTreeMap::new();
    for path in tracked_set(&Home::read(home)?)? {
        let bytes = std::fs::read(home.join(&path))?;
        out.insert(path, sha256_hex(&bytes));
    }
    Ok(out)
}

/// The one way this file ships: hashes every tracked file and lists the
/// home's paths immediately before the ship, runs it, asserts both are the
/// same immediately after, and returns the output and how many files were
/// compared.
fn ship_unchanged(
    dir: &Path,
    cwd: &Path,
    home: &Path,
    remote: &str,
) -> Result<(Output, usize), Box<dyn Error>> {
    let hashes = tracked_hashes(home)?;
    let paths = store_paths(home)?;
    let output = lys_home(
        dir,
        cwd,
        &["ship", "--home", text(home)?, "--remote", remote],
    )?;
    assert_eq!(tracked_hashes(home)?, hashes);
    assert_eq!(store_paths(home)?, paths);
    Ok((output, hashes.len()))
}

/// Ship from the test's temporary directory and return the one JSON
/// report, asserting at least one file was compared and the exit code.
fn ship_report(dir: &Path, home: &Path, remote: &Path, code: i32) -> Result<Value, Box<dyn Error>> {
    let (output, compared) = ship_unchanged(dir, dir, home, text(remote)?)?;
    assert!(compared >= 1);
    assert_eq!(output.status.code(), Some(code));
    Ok(serde_json::from_slice(&output.stdout)?)
}

/// Ship expecting a refusal on stderr and nothing on stdout; returns stderr.
fn ship_refused(
    dir: &Path,
    cwd: &Path,
    home: &Path,
    remote: &str,
) -> Result<String, Box<dyn Error>> {
    let (output, compared) = ship_unchanged(dir, cwd, home, remote)?;
    assert!(compared >= 1);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    Ok(String::from_utf8(output.stderr)?)
}

/// Every path under `dir`, lock files included, with each file's SHA-256
/// (the empty string for a directory).
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
                out.insert(relative, sha256_hex(&std::fs::read(&path)?));
            }
        }
    }
    Ok(out)
}

#[test]
fn the_first_ship_makes_the_repository_and_the_remote_and_pushes_the_tracked_set() -> Gate {
    let dir = tempfile::tempdir()?;
    let home = fixture_home(dir.path(), "home")?;
    let remote = dir.path().join("remote.git");
    let value = ship_report(dir.path(), &home, &remote, 0)?;
    let report = &value["report"];
    assert_eq!(value["command"], "ship");
    assert_eq!(report["initialised"], true);
    assert_eq!(report["remote_created"], true);
    assert_eq!(report["unchanged"], false);
    assert_eq!(report["note"], "");
    assert_eq!(report["ref"], "refs/lys/home");
    assert_eq!(report["remote"], json!(remote));
    let commit = report["commit"].as_str().ok_or("a commit")?;
    assert_eq!(commit.len(), 40);
    assert!(
        commit
            .bytes()
            .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
    );
    let at = git_text(&["--git-dir", text(&remote)?, "rev-parse", "refs/lys/home"])?;
    assert_eq!(at, commit);
    let listed = git_text(&[
        "--git-dir",
        text(&remote)?,
        "ls-tree",
        "-r",
        "--name-only",
        "refs/lys/home",
    ])?;
    let listed: Vec<&str> = listed.lines().collect();
    let tracked = tracked_set(&Home::read(&home)?)?;
    assert_eq!(
        listed,
        tracked.iter().map(String::as_str).collect::<Vec<_>>()
    );
    assert_eq!(
        listed
            .iter()
            .filter(|p| p.starts_with("templates/"))
            .count(),
        1
    );
    assert!(listed.iter().any(|p| p.starts_with("blocks/")));
    Ok(())
}

#[test]
fn a_ship_to_a_bare_repository_made_beforehand_does_not_create_it() -> Gate {
    let dir = tempfile::tempdir()?;
    let home = fixture_home(dir.path(), "home")?;
    let bare = dir.path().join("bare.git");
    git_text(&["init", "--quiet", "--bare", text(&bare)?])?;
    let value = ship_report(dir.path(), &home, &bare, 0)?;
    assert_eq!(value["report"]["remote_created"], false);
    let at = git_text(&["--git-dir", text(&bare)?, "rev-parse", "refs/lys/home"])?;
    assert_eq!(value["report"]["commit"], at.as_str());
    Ok(())
}

#[test]
fn an_unchanged_home_ships_the_same_commit_again_and_a_changed_one_builds_on_it() -> Gate {
    let dir = tempfile::tempdir()?;
    let home = fixture_home(dir.path(), "home")?;
    let remote = dir.path().join("remote.git");
    let first = ship_report(dir.path(), &home, &remote, 0)?;
    let commit = first["report"]["commit"]
        .as_str()
        .ok_or("a commit")?
        .to_owned();
    let second = ship_report(dir.path(), &home, &remote, 0)?;
    assert_eq!(second["report"]["unchanged"], true);
    assert_eq!(second["report"]["initialised"], false);
    assert_eq!(second["report"]["commit"], commit.as_str());
    assert_eq!(
        second["report"]["note"],
        format!("the home is unchanged since {commit}; no new commit was made").as_str()
    );
    let remote_arg = text(&remote)?;
    assert_eq!(
        git_text(&[
            "--git-dir",
            remote_arg,
            "rev-list",
            "--count",
            "refs/lys/home"
        ])?,
        "1"
    );
    assert_eq!(
        git_text(&["--git-dir", remote_arg, "rev-parse", "refs/lys/home"])?,
        commit
    );

    let mut session = Home::open(&home)?.open_session("fixture")?;
    session.append(EntryBody::Custom {
        custom_type: "lys.fixture".to_owned(),
        data: None,
    })?;
    drop(session);
    let third = ship_report(dir.path(), &home, &remote, 0)?;
    assert_eq!(third["report"]["unchanged"], false);
    let next = third["report"]["commit"].as_str().ok_or("a commit")?;
    assert_ne!(next, commit);
    let parents = git_text(&[
        "--git-dir",
        remote_arg,
        "rev-list",
        "--parents",
        "-n",
        "1",
        next,
    ])?;
    assert_eq!(parents, format!("{next} {commit}"));
    assert_eq!(
        git_text(&["--git-dir", remote_arg, "rev-parse", "refs/lys/home"])?,
        next
    );
    Ok(())
}

#[test]
fn a_second_home_shipped_to_the_same_remote_is_refused_as_diverged() -> Gate {
    let dir = tempfile::tempdir()?;
    let first_home = fixture_home(dir.path(), "first")?;
    let second_home = fixture_home(dir.path(), "second")?;
    let remote = dir.path().join("remote.git");
    let first = ship_report(dir.path(), &first_home, &remote, 0)?;
    let commit = first["report"]["commit"]
        .as_str()
        .ok_or("a commit")?
        .to_owned();
    let stderr = ship_refused(dir.path(), dir.path(), &second_home, text(&remote)?)?;
    let second_git = second_home.join(".git");
    let second_commit = git_text(&[
        "--git-dir",
        text(&second_git)?,
        "rev-parse",
        "refs/lys/home",
    ])?;
    assert!(stderr.starts_with("ref_diverged"), "{stderr}");
    assert!(stderr.contains("refs/lys/home"), "{stderr}");
    assert!(stderr.contains(&commit), "{stderr}");
    assert!(stderr.contains(&second_commit), "{stderr}");
    assert_eq!(
        git_text(&["--git-dir", text(&remote)?, "rev-parse", "refs/lys/home"])?,
        commit
    );
    Ok(())
}
