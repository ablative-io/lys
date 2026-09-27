//! `lys-home fetch` run as the binary (HOME-019 R6): the shipped ref
//! arrives in a new home byte for byte, each session gains one arrival
//! beside its head with an execution id of its own, committed as one
//! commit on the fetched one and carried onward by a later ship; every
//! refusal before a write leaves the target as it was, and every refusal
//! after one removes exactly what fetch created. Every remote is a path in
//! a temporary directory; no test reaches the network. No test name
//! carries a line of the fixture session.

use std::collections::BTreeMap;
use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::{Value, json};

use lys_home::record::tracked::tracked_set;
use lys_home::record::verify::verify_sessions;
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
const STAGE_THREE: &str =
    "shipping off this machine waits for stage 3's encryption from the secrets step";

type Gate = Result<(), Box<dyn Error>>;

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
    let root = dir.join(name);
    let home = Home::open(&root)?;
    std::fs::copy(SESSION, home.session_path("fixture")?)?;
    let out = dir.join(format!("{name}-out"));
    std::fs::create_dir(&out)?;
    let rendered = lys_home(
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

/// Ship `home` to `remote` and return the commit.
fn ship(dir: &Path, home: &Path, remote: &Path) -> Result<String, Box<dyn Error>> {
    let output = lys_home(
        dir,
        &["ship", "--home", text(home)?, "--remote", text(remote)?],
    )?;
    assert_eq!(output.status.code(), Some(0));
    let value: Value = serde_json::from_slice(&output.stdout)?;
    Ok(value["report"]["commit"]
        .as_str()
        .ok_or("a commit")?
        .to_owned())
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
    for (relative, _) in files_under(from)? {
        if relative.starts_with(".git") {
            continue;
        }
        let dest = to.join(&relative);
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(from.join(&relative), dest)?;
    }
    Ok(())
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
fn each_session_ends_with_one_arrival_beside_its_head() -> Gate {
    let dir = tempfile::tempdir()?;
    let m = moved(dir.path())?;
    let file = std::fs::read(m.target.join("sessions").join("fixture.jsonl"))?;
    let last = last_line(&file)?;
    let head = std::fs::read_to_string(m.target.join("sessions").join("fixture.head"))?;
    assert_eq!(last["type"], "custom");
    assert_eq!(last["customType"], "lys.harness_event");
    assert_eq!(last["parentId"], head.trim());
    let data = &last["data"];
    assert_eq!(data["kind"], "arrival");
    assert_eq!(data["detail"]["source_commit"], m.commit.as_str());
    assert_eq!(data["detail"]["remote"], json!(m.remote));
    assert_eq!(data["detail"]["ref"], "refs/lys/home");
    let execution = data["detail"]["execution"].as_str().ok_or("an execution")?;
    assert_eq!(execution.len(), 32);
    assert!(
        execution
            .bytes()
            .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
    );
    assert_eq!(
        m.report["sessions"],
        json!([{"session": "fixture", "execution": execution}])
    );
    let mut searched = 0;
    for (relative, _) in files_under(&m.source)? {
        let bytes = std::fs::read(m.source.join(&relative))?;
        let found = bytes
            .windows(execution.len())
            .any(|w| w == execution.as_bytes());
        assert!(!found, "{relative}");
        searched += 1;
    }
    assert!(searched > 0);
    assert!(verify_sessions(&Home::read(&m.target)?)?.is_empty());
    Ok(())
}

#[test]
fn every_template_a_render_event_names_is_held_by_the_target() -> Gate {
    let dir = tempfile::tempdir()?;
    let m = moved(dir.path())?;
    let session = std::fs::read_to_string(m.target.join("sessions").join("fixture.jsonl"))?;
    let mut named = 0;
    for line in session.lines().skip(1) {
        let value: Value = serde_json::from_str(line)?;
        if value["data"]["kind"] == "template_render" {
            let hash = value["data"]["detail"]["template"]
                .as_str()
                .ok_or("a template hash")?;
            assert!(
                m.target
                    .join("templates")
                    .join(&hash[..2])
                    .join(hash)
                    .is_file()
            );
            named += 1;
        }
    }
    assert!(named >= 1);
    Ok(())
}

#[test]
fn the_arrival_is_one_commit_on_the_fetched_one_and_the_status_is_clean() -> Gate {
    let dir = tempfile::tempdir()?;
    let m = moved(dir.path())?;
    let target = text(&m.target)?;
    assert_eq!(git_text(&["-C", target, "status", "--porcelain"])?, "");
    let head = git_text(&["-C", target, "rev-parse", "HEAD"])?;
    let parents = git_text(&["-C", target, "rev-list", "--parents", "-n", "1", "HEAD"])?;
    assert_eq!(parents, format!("{head} {}", m.commit));
    assert_eq!(
        git_text(&["-C", target, "rev-parse", "refs/lys/home"])?,
        head
    );
    let changed = git_text(&["-C", target, "diff", "--name-only", &m.commit, "HEAD"])?;
    assert_eq!(
        changed.lines().collect::<Vec<_>>(),
        ["sessions/fixture.index.jsonl", "sessions/fixture.jsonl"]
    );
    let mut files = 0;
    for path in ["sessions/fixture.jsonl", "sessions/fixture.index.jsonl"] {
        let before = git_bytes(&[
            "-C",
            target,
            "cat-file",
            "blob",
            &format!("{}:{path}", m.commit),
        ])?;
        let after = git_bytes(&["-C", target, "cat-file", "blob", &format!("HEAD:{path}")])?;
        let added = after
            .strip_prefix(before.as_slice())
            .ok_or("the old bytes first")?;
        let (last, body) = added.split_last().ok_or("one added line")?;
        assert_eq!(*last, b'\n', "{path}");
        assert!(!body.contains(&b'\n'), "{path}");
        if path == "sessions/fixture.jsonl" {
            assert!(std::fs::read(m.target.join(path))?.ends_with(added));
        }
        files += 1;
    }
    assert_eq!(files, 2);
    let message = git_text(&["-C", target, "log", "-1", "--format=%B", "HEAD"])?;
    assert!(message.contains(&m.commit), "{message}");
    assert!(message.contains(text(&m.remote)?), "{message}");
    assert!(message.contains("refs/lys/home"), "{message}");
    Ok(())
}

#[test]
fn a_ship_from_the_target_carries_the_arrival_onward() -> Gate {
    let dir = tempfile::tempdir()?;
    let m = moved(dir.path())?;
    let onward = dir.path().join("onward.git");
    let output = lys_home(
        dir.path(),
        &[
            "ship",
            "--home",
            text(&m.target)?,
            "--remote",
            text(&onward)?,
        ],
    )?;
    assert_eq!(output.status.code(), Some(0));
    let value: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(value["report"]["unchanged"], true);
    let head = git_text(&["-C", text(&m.target)?, "rev-parse", "HEAD"])?;
    assert_eq!(value["report"]["commit"], head.as_str());
    let blob = git_bytes(&[
        "--git-dir",
        text(&onward)?,
        "cat-file",
        "blob",
        "refs/lys/home:sessions/fixture.jsonl",
    ])?;
    let last = last_line(&blob)?;
    assert_eq!(last["data"]["kind"], "arrival");
    assert_eq!(
        last["data"]["detail"]["execution"],
        m.report["sessions"][0]["execution"]
    );
    Ok(())
}

/// The real git's path, found on this process's PATH.
fn real_git() -> Result<PathBuf, Box<dyn Error>> {
    let path = std::env::var_os("PATH").ok_or("a PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join("git"))
        .find(|candidate| candidate.is_file())
        .ok_or_else(|| "git on PATH".into())
}

#[test]
fn a_failed_arrival_commit_removes_everything_fetch_created() -> Gate {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir()?;
    let source = fixture_home(dir.path(), "source")?;
    let remote = dir.path().join("remote.git");
    ship(dir.path(), &source, &remote)?;
    let fake = dir.path().join("fake-bin");
    std::fs::create_dir(&fake)?;
    let script = fake.join("git");
    std::fs::write(
        &script,
        format!(
            "#!/bin/sh\nfor arg in \"$@\"; do\n  if [ \"$arg\" = commit-tree ]; then exit 1; fi\ndone\nexec '{}' \"$@\"\n",
            real_git()?.display()
        ),
    )?;
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755))?;
    let path = format!("{}:{}", fake.display(), std::env::var("PATH")?);
    let was_empty = dir.path().join("was-empty-2");
    std::fs::create_dir(&was_empty)?;
    let mut cases = 0;
    for (target, existed) in [(dir.path().join("nocommit"), false), (was_empty, true)] {
        let output = lys_home_with(
            dir.path(),
            Some(&path),
            &[
                "fetch",
                "--remote",
                text(&remote)?,
                "--home",
                text(&target)?,
            ],
        )?;
        let stderr = String::from_utf8(output.stderr)?;
        assert_eq!(output.status.code(), Some(1));
        assert!(stderr.starts_with("arrival_failed"), "{stderr}");
        assert!(stderr.contains("commit-tree"), "{stderr}");
        assert!(output.stdout.is_empty());
        if existed {
            assert!(is_empty_dir(&target)?);
        } else {
            assert!(!target.exists());
        }
        cases += 1;
    }
    assert_eq!(cases, 2);
    Ok(())
}

#[test]
fn two_sessions_arrive_with_execution_ids_of_their_own() -> Gate {
    let dir = tempfile::tempdir()?;
    let source = fixture_home(dir.path(), "source")?;
    let home = Home::open(&source)?;
    std::fs::copy(SESSION, home.session_path("second")?)?;
    drop(home.open_session("second")?);
    let remote = dir.path().join("remote.git");
    ship(dir.path(), &source, &remote)?;
    let target = dir.path().join("target");
    let output = fetch(dir.path(), text(&remote)?, &target)?;
    assert_eq!(output.status.code(), Some(0));
    let value: Value = serde_json::from_slice(&output.stdout)?;
    let sessions = value["report"]["sessions"].as_array().ok_or("sessions")?;
    assert_eq!(sessions.len(), 2);
    assert_ne!(sessions[0]["execution"], sessions[1]["execution"]);
    let mut checked = 0;
    for arrived in sessions {
        let id = arrived["session"].as_str().ok_or("a session")?;
        let file = std::fs::read(target.join("sessions").join(format!("{id}.jsonl")))?;
        let last = last_line(&file)?;
        assert_eq!(last["data"]["kind"], "arrival");
        assert_eq!(last["data"]["detail"]["execution"], arrived["execution"]);
        checked += 1;
    }
    assert_eq!(checked, 2);
    Ok(())
}

#[test]
fn a_target_holding_a_home_or_anything_is_refused_and_left_as_it_was() -> Gate {
    let dir = tempfile::tempdir()?;
    let source = fixture_home(dir.path(), "source")?;
    let remote = dir.path().join("remote.git");
    ship(dir.path(), &source, &remote)?;
    let holds = dir.path().join("holds");
    std::fs::create_dir_all(holds.join("sessions"))?;
    let full = dir.path().join("full");
    std::fs::create_dir(&full)?;
    std::fs::write(full.join("x.txt"), b"x")?;
    let mut cases = 0;
    for (target, refusal) in [(holds, "target_holds_home"), (full, "target_not_empty")] {
        let before = listing(&target)?;
        let output = fetch(dir.path(), text(&remote)?, &target)?;
        let stderr = String::from_utf8(output.stderr)?;
        assert_eq!(output.status.code(), Some(1));
        assert!(stderr.starts_with(refusal), "{stderr}");
        assert_eq!(listing(&target)?, before);
        cases += 1;
    }
    assert_eq!(cases, 2);
    Ok(())
}

#[test]
fn a_remote_off_this_machine_is_refused_and_the_empty_target_stays_empty() -> Gate {
    let dir = tempfile::tempdir()?;
    let empty = dir.path().join("empty");
    std::fs::create_dir(&empty)?;
    let output = fetch(dir.path(), "https://github.com/o/r.git", &empty)?;
    let stderr = String::from_utf8(output.stderr)?;
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr.starts_with("remote_not_local"), "{stderr}");
    assert!(stderr.contains(STAGE_THREE), "{stderr}");
    assert!(is_empty_dir(&empty)?);
    Ok(())
}

#[test]
fn a_corrupted_block_fails_verification_by_hash_and_nothing_is_left() -> Gate {
    let dir = tempfile::tempdir()?;
    let source = fixture_home(dir.path(), "source")?;
    let copy = dir.path().join("copy");
    copy_home(&source, &copy)?;
    let tracked = tracked_set(&Home::read(&copy)?)?;
    let block = tracked
        .iter()
        .find(|p| p.starts_with("blocks/"))
        .ok_or("a block")?
        .clone();
    let hash = block.rsplit('/').next().ok_or("a hash")?.to_owned();
    std::fs::write(copy.join(&block), b"corrupted-block-bytes")?;
    let remote = dir.path().join("bad-remote.git");
    ship(dir.path(), &copy, &remote)?;
    let was_empty = dir.path().join("was-empty");
    std::fs::create_dir(&was_empty)?;
    let mut cases = 0;
    for (target, existed) in [(dir.path().join("bad"), false), (was_empty, true)] {
        let output = fetch(dir.path(), text(&remote)?, &target)?;
        assert_eq!(output.status.code(), Some(1));
        let stdout = String::from_utf8(output.stdout)?;
        assert!(!stdout.contains("corrupted-block-bytes"));
        let value: Value = serde_json::from_str(&stdout)?;
        assert_eq!(value["command"], "fetch");
        assert_eq!(value["refused"], "verification_failed");
        assert_eq!(value["bad_blocks"], json!([hash]));
        if existed {
            assert!(is_empty_dir(&target)?);
        } else {
            assert!(!target.exists());
        }
        cases += 1;
    }
    assert_eq!(cases, 2);
    Ok(())
}

#[test]
fn a_stale_index_pushed_with_plain_git_fails_verification_and_nothing_is_left() -> Gate {
    let dir = tempfile::tempdir()?;
    let source = fixture_home(dir.path(), "source")?;
    let copy = dir.path().join("copy");
    copy_home(&source, &copy)?;
    let session = copy.join("sessions").join("fixture.jsonl");
    let head = std::fs::read_to_string(copy.join("sessions").join("fixture.head"))?;
    let line = json!({
        "type": "custom", "customType": "lys.fixture", "id": "appended-directly",
        "parentId": head.trim(), "timestamp": "2026-01-01T00:00:09.000Z",
    });
    let mut bytes = std::fs::read(&session)?;
    bytes.extend_from_slice(format!("{line}\n").as_bytes());
    std::fs::write(&session, bytes)?;
    let remote = dir.path().join("stale-remote.git");
    git_bytes(&["init", "--quiet", "--bare", text(&remote)?])?;
    let copy_arg = text(&copy)?;
    git_bytes(&["init", "--quiet", copy_arg])?;
    let tracked = tracked_set(&Home::read(&copy)?)?;
    let mut add = vec!["-C", copy_arg, "add", "--"];
    add.extend(tracked.iter().map(String::as_str));
    git_bytes(&add)?;
    git_bytes(&["-C", copy_arg, "commit", "--quiet", "-m", "stale"])?;
    git_bytes(&[
        "-C",
        copy_arg,
        "push",
        "--quiet",
        text(&remote)?,
        "HEAD:refs/lys/home",
    ])?;
    let target = dir.path().join("stale");
    let output = fetch(dir.path(), text(&remote)?, &target)?;
    assert_eq!(output.status.code(), Some(1));
    let value: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(
        value["sessions"],
        json!([{"session": "fixture", "reason": "index_not_this_file"}])
    );
    assert!(!target.exists());
    Ok(())
}

#[test]
fn a_remote_whose_path_would_carry_the_arrival_over_the_cap_is_refused_first() -> Gate {
    let dir = tempfile::tempdir()?;
    let mut remote = dir.path().to_path_buf();
    while 400 - remote.as_os_str().len() > 202 {
        remote.push("r".repeat(200));
    }
    let last = 400 - remote.as_os_str().len() - 1;
    remote.push("r".repeat(last));
    assert_eq!(remote.as_os_str().len(), 400);
    assert!(remote.is_absolute());
    std::fs::create_dir_all(&remote)?;
    git_bytes(&["init", "--quiet", "--bare", text(&remote)?])?;
    let target = dir.path().join("target");
    let output = fetch(dir.path(), text(&remote)?, &target)?;
    let stderr = String::from_utf8(output.stderr)?;
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr.contains("over the limit"), "{stderr}");
    assert!(!target.exists());
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
