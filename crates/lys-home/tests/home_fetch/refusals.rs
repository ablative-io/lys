//! What `lys-home fetch` refuses (HOME-019 R6): a refusal before a write
//! leaves the target as it was, and a refusal after one removes exactly
//! what fetch created.

use lys_home::Home;
use lys_home::record::tracked::tracked_set;
use serde_json::Value;
use serde_json::json;
use std::error::Error;
use std::path::PathBuf;

use super::{
    Gate, STAGE_THREE, copy_home, fetch, fixture_home, git_bytes, is_empty_dir, last_line, listing,
    lys_home_with, ship, text,
};

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
fn a_remote_whose_long_path_once_crossed_the_event_cap_arrives_whole() -> Gate {
    let dir = tempfile::tempdir()?;
    let source = fixture_home(dir.path(), "source")?;
    let mut remote = dir.path().to_path_buf();
    while 800 - remote.as_os_str().len() > 202 {
        remote.push("r".repeat(200));
    }
    let last = 800 - remote.as_os_str().len() - 1;
    remote.push("r".repeat(last));
    assert_eq!(remote.as_os_str().len(), 800);
    assert!(remote.is_absolute());
    let commit = ship(dir.path(), &source, &remote)?;
    let target = dir.path().join("target");
    let output = fetch(dir.path(), text(&remote)?, &target)?;
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let file = std::fs::read(target.join("sessions").join("fixture.jsonl"))?;
    let arrived = last_line(&file)?;
    assert_eq!(arrived["data"]["kind"], "arrival");
    assert_eq!(arrived["data"]["detail"]["remote"], json!(text(&remote)?));
    assert_eq!(arrived["data"]["detail"]["source_commit"], commit.as_str());
    let len = serde_json::to_vec(&arrived["data"])?.len();
    assert!(len > 800, "{len}");
    Ok(())
}
