//! What `lys-home ship` refuses (HOME-019 R5), each by name and each with
//! nothing pushed.

use lys_home::Home;
use lys_home::record::tracked::tracked_set;
use serde_json::json;

use super::{
    Gate, STAGE_THREE, fixture_home, git_text, listing, sha256_hex, ship_refused, ship_report, text,
};

#[test]
fn a_remote_that_is_not_a_bare_repository_is_refused_naming_what_was_found() -> Gate {
    let dir = tempfile::tempdir()?;
    let home = fixture_home(dir.path(), "home")?;
    let file = dir.path().join("remote-file");
    std::fs::write(&file, b"not a repository")?;
    let before = sha256_hex(&std::fs::read(&file)?);
    let stderr = ship_refused(dir.path(), dir.path(), &home, text(&file)?)?;
    assert!(stderr.contains("remote_not_bare"), "{stderr}");
    assert!(stderr.contains(text(&file)?), "{stderr}");
    assert!(stderr.contains("a file"), "{stderr}");
    assert_eq!(sha256_hex(&std::fs::read(&file)?), before);

    let worktree = dir.path().join("worktree");
    git_text(&["init", "--quiet", text(&worktree)?])?;
    let plain = dir.path().join("plain");
    std::fs::create_dir(&plain)?;
    std::fs::write(plain.join("x.txt"), b"x")?;
    let mut cases = 0;
    for (remote, found) in [
        (&worktree, "a non-bare repository"),
        (&plain, "a directory that is not a bare repository"),
    ] {
        let before = listing(remote)?;
        let stderr = ship_refused(dir.path(), dir.path(), &home, text(remote)?)?;
        assert!(stderr.contains("remote_not_bare"), "{stderr}");
        assert!(stderr.contains(found), "{stderr}");
        assert_eq!(listing(remote)?, before);
        cases += 1;
    }
    assert_eq!(cases, 2);
    assert!(!home.join(".git").exists());
    Ok(())
}

#[test]
fn a_remote_off_this_machine_is_refused_before_anything_is_written() -> Gate {
    let dir = tempfile::tempdir()?;
    let home = fixture_home(dir.path(), "home")?;
    let cwd = dir.path().join("cwd");
    std::fs::create_dir(&cwd)?;
    let mut cases = 0;
    for remote in [
        "host.example:r.git",
        "ssh://host.example/r.git",
        "https://github.com/o/r.git",
    ] {
        let home_before = listing(&home)?;
        let cwd_before = listing(&cwd)?;
        let stderr = ship_refused(dir.path(), &cwd, &home, remote)?;
        assert!(stderr.starts_with("remote_not_local"), "{stderr}");
        assert!(stderr.contains(STAGE_THREE), "{stderr}");
        assert_eq!(listing(&home)?, home_before);
        assert_eq!(listing(&cwd)?, cwd_before);
        cases += 1;
    }
    assert_eq!(cases, 3);
    Ok(())
}

#[test]
fn a_home_with_no_session_is_refused_as_empty() -> Gate {
    let dir = tempfile::tempdir()?;
    let root = dir.path().join("home");
    let home = Home::open(&root)?;
    home.blocks()?.put(b"fixture block")?;
    let tracked = tracked_set(&home)?;
    assert_eq!(tracked.len(), 1);
    assert!(tracked[0].starts_with("blocks/"));
    let remote = dir.path().join("remote.git");
    let stderr = ship_refused(dir.path(), dir.path(), &root, text(&remote)?)?;
    assert!(stderr.starts_with("empty_home"), "{stderr}");
    assert!(stderr.contains("holds no sessions"), "{stderr}");
    assert!(
        stderr.contains("ship after a session has been captured"),
        "{stderr}"
    );
    assert!(!remote.exists());
    assert!(!root.join(".git").exists());
    Ok(())
}

#[test]
fn a_line_appended_past_the_index_is_reported_as_a_stale_index() -> Gate {
    let dir = tempfile::tempdir()?;
    let home = fixture_home(dir.path(), "home")?;
    let session = home.join("sessions").join("fixture.jsonl");
    let head = std::fs::read_to_string(home.join("sessions").join("fixture.head"))?;
    let line = json!({
        "type": "custom", "customType": "lys.fixture", "id": "appended-directly",
        "parentId": head.trim(), "timestamp": "2026-01-01T00:00:09.000Z",
    });
    let mut bytes = std::fs::read(&session)?;
    bytes.extend_from_slice(format!("{line}\n").as_bytes());
    std::fs::write(&session, bytes)?;
    let index = home.join("sessions").join("fixture.index.jsonl");
    let before = sha256_hex(&std::fs::read(&index)?);
    let remote = dir.path().join("remote.git");
    let value = ship_report(dir.path(), &home, &remote, 1)?;
    assert_eq!(
        value,
        json!({"command": "ship", "refused": "stale_index", "sessions": [{"session": "fixture", "reason": "index_not_this_file"}]})
    );
    assert!(!remote.exists());
    assert!(!home.join(".git").exists());
    assert_eq!(sha256_hex(&std::fs::read(&index)?), before);
    Ok(())
}

#[test]
fn a_missing_index_or_head_is_refused_naming_the_act_that_writes_it() -> Gate {
    let mut cases = 0;
    for (missing, refusal) in [
        ("fixture.index.jsonl", "index_missing"),
        ("fixture.head", "head_missing"),
    ] {
        let dir = tempfile::tempdir()?;
        let home = fixture_home(dir.path(), "home")?;
        let file = home.join("sessions").join(missing);
        std::fs::remove_file(&file)?;
        let remote = dir.path().join("remote.git");
        let stderr = ship_refused(dir.path(), dir.path(), &home, text(&remote)?)?;
        assert!(stderr.starts_with(refusal), "{stderr}");
        assert!(stderr.contains("fixture"), "{stderr}");
        assert!(stderr.contains(&format!("sessions/{missing}")), "{stderr}");
        assert!(stderr.contains("lys-home given --home"), "{stderr}");
        assert!(!remote.exists());
        assert!(!file.exists());
        cases += 1;
    }
    assert_eq!(cases, 2);
    Ok(())
}

#[test]
fn a_session_a_live_owner_holds_is_refused_without_waiting() -> Gate {
    let dir = tempfile::tempdir()?;
    let home = fixture_home(dir.path(), "home")?;
    let owner = Home::open(&home)?.open_session("fixture")?;
    let remote = dir.path().join("remote.git");
    let stderr = ship_refused(dir.path(), dir.path(), &home, text(&remote)?)?;
    drop(owner);
    assert!(stderr.starts_with("session_held"), "{stderr}");
    assert!(stderr.contains("fixture"), "{stderr}");
    assert!(stderr.contains("ship after that seat stops"), "{stderr}");
    assert!(!stderr.contains("ship without that session"), "{stderr}");
    assert!(!remote.exists());
    assert!(!home.join(".git").exists());
    Ok(())
}

#[test]
fn a_home_repository_tracking_a_foreign_file_is_refused() -> Gate {
    let dir = tempfile::tempdir()?;
    let home = fixture_home(dir.path(), "home")?;
    std::fs::write(home.join("notes.txt"), b"notes")?;
    let git_dir = home.join(".git");
    let (git_arg, home_arg) = (text(&git_dir)?, text(&home)?);
    git_text(&["init", "--quiet", home_arg])?;
    git_text(&[
        "--git-dir",
        git_arg,
        "--work-tree",
        home_arg,
        "add",
        "notes.txt",
    ])?;
    git_text(&[
        "--git-dir",
        git_arg,
        "--work-tree",
        home_arg,
        "commit",
        "--quiet",
        "-m",
        "notes",
    ])?;
    let remote = dir.path().join("remote.git");
    let stderr = ship_refused(dir.path(), dir.path(), &home, text(&remote)?)?;
    assert!(stderr.starts_with("foreign_tracked"), "{stderr}");
    assert!(stderr.contains("notes.txt"), "{stderr}");
    assert!(!remote.exists());
    Ok(())
}
