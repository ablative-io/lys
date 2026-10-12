#![cfg(test)]
//! Notify placeholders bind literal runner paths before the spawn boundary.

use std::collections::BTreeMap;
use std::error::Error;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use super::{Sessions, notify_named};
use crate::Launch;

type TestResult = Result<(), Box<dyn Error>>;
const PLACEHOLDER: &str =
    r#"notify=["$LYS_PROGRAM", "runner", "notify", "--socket", "$LYS_RUNNER_SOCKET"]"#;

fn launch() -> Launch {
    Launch {
        session: "notify-session".to_owned(),
        program: "/bin/cat".to_owned(),
        arguments: vec![PLACEHOLDER.to_owned()],
        directory: "/".to_owned(),
        environment: BTreeMap::new(),
        config: None,
        columns: 80,
        rows: 24,
        rotation: None,
        policy: None,
    }
}

#[test]
fn notify_binding_quotes_both_paths_and_replaces_every_placeholder() -> TestResult {
    let mut launch = launch();
    launch.arguments.extend([
        "-c".to_owned(),
        PLACEHOLDER.to_owned(),
        "notify=[]".to_owned(),
    ]);
    let program = "/opt/runner \"quoted\"/lys";
    let socket = "/run/back\\slash/runner.sock";
    launch.environment = BTreeMap::from([
        ("LYS_PROGRAM".to_owned(), program.to_owned()),
        ("LYS_RUNNER_SOCKET".to_owned(), socket.to_owned()),
    ]);
    let environment = launch.environment.clone();
    notify_named(&mut launch)?;
    let notify: Vec<String> = serde_json::from_str(
        launch.arguments[0]
            .strip_prefix("notify=")
            .ok_or("notify missing")?,
    )?;
    assert_eq!(notify, [program, "runner", "notify", "--socket", socket]);
    assert_eq!(launch.arguments[2], launch.arguments[0]);
    assert_eq!(launch.arguments[1], "-c");
    assert_eq!(launch.arguments[3], "notify=[]");
    assert_eq!(launch.environment, environment);
    Ok(())
}

#[test]
fn notify_binding_refuses_missing_program_without_changing_launch() {
    let mut launch = launch();
    launch.environment.insert(
        "LYS_RUNNER_SOCKET".to_owned(),
        "/run/runner.sock".to_owned(),
    );
    let original = launch.clone();
    let error = notify_named(&mut launch).expect_err("missing program admitted");
    assert_eq!(error.name(), "notify_unnamed");
    assert!(error.to_string().contains("LYS_PROGRAM"));
    assert_eq!(launch, original);
}

#[test]
fn notify_binding_refuses_missing_socket_without_changing_launch() {
    let mut launch = launch();
    launch
        .environment
        .insert("LYS_PROGRAM".to_owned(), "/opt/lys".to_owned());
    let original = launch.clone();
    let error = notify_named(&mut launch).expect_err("missing socket admitted");
    assert_eq!(error.name(), "notify_unnamed");
    assert!(error.to_string().contains("LYS_RUNNER_SOCKET"));
    assert_eq!(launch, original);
}

#[test]
fn notify_binding_without_placeholder_keeps_every_input() -> TestResult {
    let mut launch = launch();
    launch.arguments = vec!["notify=[]".to_owned(), "$LYS_PROGRAM".to_owned()];
    let original = launch.clone();
    notify_named(&mut launch)?;
    assert_eq!(launch, original);
    Ok(())
}

#[test]
fn session_notify_binds_after_runner_paths_are_published_before_spawn() -> TestResult {
    let state = tempfile::tempdir()?;
    let sessions = Sessions::open(state.path(), 4096)?;
    let socket = state.path().join("runner.sock");
    sessions.serving_on(&socket);
    let mut launch = launch();
    launch.arguments = vec![
        "-c".to_owned(),
        "exec cat".to_owned(),
        PLACEHOLDER.to_owned(),
    ];
    launch.program = "/bin/sh".to_owned();
    launch.environment = BTreeMap::from([
        ("LYS_PROGRAM".to_owned(), "/foreign/program".to_owned()),
        ("LYS_RUNNER_SOCKET".to_owned(), "/foreign/socket".to_owned()),
    ]);
    sessions.start(launch)?;
    let observed = {
        let table = sessions.lock()?;
        table
            .sessions
            .get("notify-session")
            .and_then(|session| session.launch.clone())
            .ok_or("launch missing")?
    };
    sessions.stop_all()?;
    let notify: Vec<String> = serde_json::from_str(
        observed.arguments[2]
            .strip_prefix("notify=")
            .ok_or("notify missing")?,
    )?;
    assert_eq!(
        notify,
        [
            std::env::current_exe()?
                .to_str()
                .ok_or("program is not text")?,
            "runner",
            "notify",
            "--socket",
            socket.to_str().ok_or("socket is not text")?,
        ]
    );
    Ok(())
}

#[test]
fn session_notify_unnamed_refuses_before_spawn_and_releases_reservation() -> TestResult {
    let state = tempfile::tempdir()?;
    let sessions = Sessions::open(state.path(), 4096)?;
    let spawned = Arc::new(AtomicBool::new(false));
    let observed = Arc::clone(&spawned);
    *sessions
        .spawn_probe
        .lock()
        .map_err(|error| error.to_string())? =
        Some(Box::new(move || observed.store(true, Ordering::SeqCst)));
    let error = sessions
        .start(launch())
        .expect_err("unnamed notify spawned");
    assert_eq!(error.name(), "notify_unnamed");
    assert!(!spawned.load(Ordering::SeqCst));
    {
        let table = sessions.lock()?;
        assert!(table.sessions.is_empty());
        assert!(table.starting.is_empty());
    }
    sessions.stop_all()?;
    Ok(())
}
