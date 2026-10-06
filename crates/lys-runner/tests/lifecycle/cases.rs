#![cfg(test)]
//! Stream reads release the table, structured limits drive rotation, and ordinary stops flush spend.

use std::collections::BTreeMap;
use std::error::Error;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use super::Sessions;
use crate::Launch;
use crate::peer::Collected;
use crate::rotation::{Limit, Rotation, RotationState};
use crate::tracking::{CLAUDE_ADAPTER, Harness, Tracking};
use crate::tracking_store::{Body, Commit, SourceState};

type TestResult = Result<(), Box<dyn Error>>;

fn tracked(
    dir: &std::path::Path,
    script: &str,
    transcript: &std::path::Path,
) -> Result<Arc<Sessions>, Box<dyn Error>> {
    let sessions = Sessions::open(&dir.join("state"), 4096)?;
    sessions.start(Launch {
        session: "session".to_owned(),
        program: "/bin/sh".to_owned(),
        arguments: vec!["-c".to_owned(), script.to_owned()],
        directory: "/".to_owned(),
        environment: BTreeMap::from([("TRANSCRIPT".to_owned(), transcript.display().to_string())]),
        config: None,
        columns: 80,
        rows: 24,
        rotation: None,
        policy: None,
    })?;
    let mut table = sessions.lock()?;
    table
        .sessions
        .get_mut("session")
        .ok_or("session missing")?
        .guard
        .tracking = Some(Tracking {
        harness: Harness::ClaudeCode,
        adapter: CLAUDE_ADAPTER.to_owned(),
        version: "2.1.285".to_owned(),
        config_home: dir.display().to_string(),
        context_window: 200_000,
        profile_version: 1,
        account: Some("account".to_owned()),
        requires_pre_tool: false,
    });
    table.feed.append(
        "session",
        1,
        Vec::new(),
        Commit {
            source: Some(SourceState {
                path: transcript.display().to_string(),
                bound: "native".to_owned(),
                ..SourceState::default()
            }),
            attempt: None,
        },
    )?;
    drop(table);
    sessions.until("session", &AtomicBool::new(false), |session, _| {
        session
            .scrollback()
            .from(0)
            .is_ok_and(|bytes| bytes.windows(7).any(|word| word == b"ready\r\n"))
            .then_some(Ok(()))
    })?;
    Ok(sessions)
}

#[test]
fn a_stream_wait_holds_no_session_table_lock() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("stream");
    nix::unistd::mkfifo(
        &path,
        nix::sys::stat::Mode::S_IRUSR | nix::sys::stat::Mode::S_IWUSR,
    )?;
    let sessions = tracked(dir.path(), "printf 'ready\\n'; exec cat", &path)?;
    let reader = Arc::clone(&sessions);
    let joined = std::thread::spawn(move || reader.read_source("session", None));
    // Opening the writer signals that the reader reached the stream; it cannot finish until EOF.
    let writer = std::fs::OpenOptions::new().write(true).open(&path)?;
    let available = sessions.table.try_lock().is_ok();
    drop(writer);
    joined
        .join()
        .map_err(|panic| format!("stream reader panicked: {panic:?}"))?;
    std::fs::remove_file(&path)?;
    sessions.stop_all()?;
    assert!(available, "transcript I/O held the global session table");
    Ok(())
}

#[test]
fn quoted_limit_words_with_a_forty_percent_status_do_not_rotate_claude() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("stream.jsonl");
    std::fs::write(&path, b"")?;
    let sessions = tracked(dir.path(), "printf 'ready\\n'; exec cat", &path)?;
    sessions
        .lock()?
        .sessions
        .get_mut("session")
        .ok_or("session missing")?
        .rotation = Some(RotationState::new(Rotation {
        accounts: vec!["one".to_owned(), "two".to_owned()],
        variable: "ACCOUNT".to_owned(),
        limit: Limit::Words {
            words: vec!["usage limit reached".to_owned()],
        },
        resume_arguments: Vec::new(),
    })?);
    sessions.collect(
        "session",
        &Collected::StatusLine {
            input: serde_json::json!({
                "session_id": "native", "rate_limits": {"five_hour": {
                    "used_percentage": 40, "resets_at": crate::session::now_ms() / 1000 + 3600
                }}
            }),
        },
    )?;
    let tripped = {
        let mut table = sessions.lock()?;
        let session = table.sessions.get_mut("session").ok_or("session missing")?;
        session
            .output
            .begin(session.generation, session.rotation.as_ref(), true)?;
        assert!(
            !session
                .output
                .push(session.generation, b"review says: usage limit reached")?
        );
        session
            .rotation
            .as_ref()
            .ok_or("rotation missing")?
            .tripped()
    };
    sessions.stop_all()?;
    assert!(
        !tripped,
        "quoted terminal words rotated a structured Claude session"
    );
    Ok(())
}

#[test]
fn an_ordinary_stop_counts_the_usage_written_on_hang_up() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("stream.jsonl");
    std::fs::write(&path, b"")?;
    let record = serde_json::json!({"type":"assistant", "timestamp":"2026-10-01T01:00:00Z", "message": {
        "id":"final", "model":"model", "usage":{"input_tokens":10,"output_tokens":20,
            "cache_creation_input_tokens":0,"cache_read_input_tokens":0}
    }});
    let script = format!(
        "trap 'printf '\\''%s\\n'\\'' '\\''{record}'\\'' >> \"$TRANSCRIPT\"; exit 0' HUP; printf 'ready\\n'; while IFS= read -r line; do :; done"
    );
    let sessions = tracked(dir.path(), &script, &path)?;
    let ended = sessions.end("session", &AtomicBool::new(false))?;
    let entries = sessions.lock()?.feed.page(None)?.entries;
    sessions.stop_all()?;
    assert_eq!(ended.status, Some(0), "{ended:?}");
    assert!(
        entries
            .iter()
            .any(|entry| matches!(&entry.body, Body::Usage(usage)
        if usage.figures.output_tokens == Some(20))),
        "{entries:?}"
    );
    Ok(())
}

#[test]
fn a_full_live_status_window_trips_the_declared_rotation() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("stream.jsonl");
    std::fs::write(&path, b"")?;
    let sessions = tracked(dir.path(), "printf 'ready\\n'; exec cat", &path)?;
    sessions
        .lock()?
        .sessions
        .get_mut("session")
        .ok_or("session missing")?
        .rotation = Some(RotationState::new(Rotation {
        accounts: vec!["one".to_owned(), "two".to_owned()],
        variable: "ACCOUNT".to_owned(),
        limit: Limit::PlanWindow,
        resume_arguments: Vec::new(),
    })?);
    sessions.collect(
        "session",
        &Collected::StatusLine {
            input: serde_json::json!({
                "session_id": "native", "rate_limits": {"five_hour": {
                    "used_percentage": 100, "resets_at": crate::session::now_ms() / 1000 + 3600
                }}
            }),
        },
    )?;
    let rotation = sessions
        .lock()?
        .sessions
        .get("session")
        .ok_or("session missing")?
        .rotation
        .as_ref()
        .ok_or("rotation missing")?
        .clone();
    let tripped = rotation.tripped() || !rotation.moves().is_empty();
    sessions.stop_all()?;
    assert!(
        tripped,
        "the collector did not apply the declared account-window limit"
    );
    Ok(())
}

#[test]
fn a_claude_code_launch_records_its_folder_as_trusted_before_the_spawn_and_only_once() -> TestResult
{
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir()?;
    let home = dir.path().join("home");
    let work = dir.path().join("work");
    std::fs::create_dir_all(&work)?;
    // A harness that answers its version as the adapter measured it, and at
    // its start prints the trust file it is handed, so the output proves the
    // row was there when the process began.
    let harness = dir.path().join("claude");
    std::fs::write(
        &harness,
        "#!/bin/sh\nif [ \"$1\" = --version ]; then echo '2.1.285 (Claude Code)'; exit 0; fi\ncat \"$TRUST\"\nprintf 'ready\\n'\nexec cat\n",
    )?;
    std::fs::set_permissions(&harness, std::fs::Permissions::from_mode(0o755))?;
    let trust_file = home.join(crate::trust::FILE);
    // As the server starts a run: no tracking, the harness named on the
    // launch's config, and the configuration directory the profile chose.
    let config = crate::launch_config::Config {
        requires_controls: false,
        files: Vec::new(),
        argument_files: BTreeMap::new(),
        environment_paths: BTreeMap::new(),
        working_directory: false,
        harness: Some(Harness::ClaudeCode),
    };
    let launch = |session: &str| Launch {
        session: session.to_owned(),
        program: harness.display().to_string(),
        arguments: Vec::new(),
        directory: work.display().to_string(),
        environment: BTreeMap::from([
            ("TRUST".to_owned(), trust_file.display().to_string()),
            ("CLAUDE_CONFIG_DIR".to_owned(), home.display().to_string()),
        ]),
        config: Some(config.clone()),
        columns: 80,
        rows: 24,
        rotation: None,
        policy: None,
    };
    let ready = |session: &mut crate::session::output::OutputState, _: &str| {
        let bytes = session.scrollback().from(0).ok()?;
        bytes
            .windows(7)
            .any(|word| word == b"ready\r\n")
            .then_some(Ok(bytes))
    };
    let sessions = Sessions::open(&dir.path().join("state"), 4096)?;
    assert!(
        !trust_file.exists(),
        "nothing is trusted before the first launch"
    );
    sessions.begin(launch("first"), None, None)?;
    let output = sessions.until("first", &AtomicBool::new(false), ready)?;
    let bytes = std::fs::read(&trust_file)?;
    let root: serde_json::Value = serde_json::from_slice(&bytes)?;
    let canonical = std::fs::canonicalize(&work)?.display().to_string();
    assert_eq!(
        root["projects"][canonical.as_str()][crate::trust::ROW],
        serde_json::Value::Bool(true),
        "the run's canonical folder has its trust row"
    );
    let printed = String::from_utf8_lossy(&output).replace("\r\n", "\n");
    assert!(
        printed.contains(crate::trust::ROW) && printed.contains(&canonical),
        "the harness read the row at its start: {printed}"
    );
    sessions.begin(launch("second"), None, None)?;
    sessions.until("second", &AtomicBool::new(false), ready)?;
    assert_eq!(
        std::fs::read(&trust_file)?,
        bytes,
        "a second launch into a trusted folder leaves the file byte-identical"
    );
    sessions.stop_all()?;
    Ok(())
}

#[test]
fn a_trust_dialog_the_harness_still_shows_is_answered_for_the_named_folder_only() -> TestResult {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir()?;
    let home = dir.path().join("home");
    let work = dir.path().join("work");
    let other = dir.path().join("other");
    std::fs::create_dir_all(&work)?;
    std::fs::create_dir_all(&other)?;
    let shown_work = std::fs::canonicalize(&work)?.display().to_string();
    let shown_other = std::fs::canonicalize(&other)?.display().to_string();
    // A harness that lost the row to another harness's rewrite: it paints
    // the trust dialog for the folder named in DIALOG_FOLDER, waits for the
    // answer, and only then is ready.
    let harness = dir.path().join("claude");
    std::fs::write(
        &harness,
        "#!/bin/sh\nif [ \"$1\" = --version ]; then echo '2.1.285 (Claude Code)'; exit 0; fi\nprintf 'Do you trust the files in this folder?\\n\\n%s\\n\\n1. Yes, proceed\\n2. No, exit\\n' \"$DIALOG_FOLDER\"\nread answer\nprintf 'answered\\nready\\n'\nexec cat\n",
    )?;
    std::fs::set_permissions(&harness, std::fs::Permissions::from_mode(0o755))?;
    let config = crate::launch_config::Config {
        requires_controls: false,
        files: Vec::new(),
        argument_files: BTreeMap::new(),
        environment_paths: BTreeMap::new(),
        working_directory: false,
        harness: Some(Harness::ClaudeCode),
    };
    let launch = |session: &str, shown: &str| Launch {
        session: session.to_owned(),
        program: harness.display().to_string(),
        arguments: Vec::new(),
        directory: work.display().to_string(),
        environment: BTreeMap::from([
            ("DIALOG_FOLDER".to_owned(), shown.to_owned()),
            ("CLAUDE_CONFIG_DIR".to_owned(), home.display().to_string()),
        ]),
        config: Some(config.clone()),
        columns: 80,
        rows: 24,
        rotation: None,
        policy: None,
    };
    let never = AtomicBool::new(false);
    let printed = |session: &mut crate::session::output::OutputState, _: &str| {
        session.scrollback().from(0).ok().map(Ok)
    };
    let ready = |session: &mut crate::session::output::OutputState, _: &str| {
        let bytes = session.scrollback().from(0).ok()?;
        bytes
            .windows(7)
            .any(|word| word == b"ready\r\n")
            .then_some(Ok(bytes))
    };
    let state_of = |session: &str, state: &str| {
        let session = session.to_owned();
        let state = state.to_owned();
        move |table: &mut crate::session::Table| {
            table
                .feed
                .page(None)
                .ok()?
                .entries
                .iter()
                .any(|entry| {
                    entry.session == session
                        && matches!(&entry.body, Body::Coverage(coverage) if coverage.state == state)
                })
                .then_some(())
        }
    };
    let sessions = Sessions::open(&dir.path().join("state"), 4096)?;

    // The dialog names the run's folder: answered, and the run goes on.
    sessions.begin(launch("named", &shown_work), None, None)?;
    let output = sessions.until("named", &never, ready)?;
    let text = String::from_utf8_lossy(&output);
    assert!(
        text.contains("answered"),
        "the harness read the answer: {text}"
    );
    sessions.until_any(&never, state_of("named", "trust_answered"))?;

    // The dialog names another folder: left alone, said in the feed, and
    // the harness is still waiting at it.
    sessions.begin(launch("other", &shown_other), None, None)?;
    sessions.until_any(&never, state_of("other", "trust_dialog_unanswered"))?;
    let output = sessions.until("other", &never, printed)?;
    let text = String::from_utf8_lossy(&output);
    assert!(
        text.contains("Do you trust") && !text.contains("answered"),
        "nothing was typed into a dialog for another folder: {text}"
    );
    sessions.stop_all()?;
    Ok(())
}
