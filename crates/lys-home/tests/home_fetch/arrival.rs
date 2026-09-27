//! The arrival `lys-home fetch` hangs beside each session's head (HOME-019
//! R6): one per session with an execution id of its own, every template a
//! render names held by the target, one commit on the fetched one, and
//! carried onward by a later ship.

use lys_home::Home;
use lys_home::record::verify::verify_sessions;
use serde_json::Value;
use serde_json::json;

use super::{
    Gate, SESSION, fetch, files_under, fixture_home, git_bytes, git_text, last_line, lys_home,
    moved, ship, text,
};

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
