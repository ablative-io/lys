#![cfg(test)]
use super::{act, record_after_end};
use crate::agent_pass_store::Passes;
use crate::error::ServerError;
use lys_identity::AgentId;
use lys_runner::{Act, Launch};
use std::collections::BTreeMap;
use std::error::Error;
use std::sync::{Arc, Barrier, Mutex};

type TestResult = Result<(), Box<dyn Error>>;
fn launch(configured: bool) -> Launch {
    Launch {
        session: "session-fixture".to_owned(),
        program: "/bin/sh".to_owned(),
        arguments: Vec::new(),
        directory: "/".to_owned(),
        environment: BTreeMap::new(),
        config: configured.then(|| lys_runner::launch_config::Config {
            files: Vec::new(),
            argument_files: BTreeMap::new(),
            environment_paths: BTreeMap::new(),
            working_directory: false,
        }),
        columns: 120,
        rows: 30,
        rotation: None,
        policy: None,
    }
}
#[test]
fn a_launch_without_native_config_issues_no_agent_pass() -> TestResult {
    let dir = tempfile::tempdir()?;
    let file = dir.path().join("passes.json");
    let mut passes = Passes::open(file.clone())?;
    let selected = act(
        &mut passes,
        AgentId::from_bytes([1; 16]),
        "http://fixture.test",
        launch(false),
        None,
    )?;
    assert!(matches!(selected, Act::Start { lys_mcp: None, .. }));
    assert!(!passes.has_session("session-fixture")?);
    assert!(!file.exists());
    Ok(())
}
#[test]
fn concurrent_start_selection_issues_one_pass() -> TestResult {
    let dir = tempfile::tempdir()?;
    let passes = Arc::new(Mutex::new(Passes::open(dir.path().join("passes.json"))?));
    let barrier = Arc::new(Barrier::new(2));
    let mut threads = Vec::new();
    for _ in 0..2 {
        let passes = Arc::clone(&passes);
        let barrier = Arc::clone(&barrier);
        threads.push(std::thread::spawn(move || {
            barrier.wait();
            let mut passes = passes.lock().map_err(|error| error.to_string())?;
            act(
                &mut passes,
                AgentId::from_bytes([1; 16]),
                "http://fixture.test",
                launch(true),
                None,
            )
            .map_err(|error| error.to_string())
        }));
    }
    let mut starts = 0;
    let mut statuses = 0;
    for thread in threads {
        let selected = thread
            .join()
            .map_err(|error| format!("start selection panicked: {error:?}"))??;
        match selected {
            Act::Start {
                lys_mcp: Some(_), ..
            } => starts += 1,
            Act::Status { .. } => statuses += 1,
            _ => return Err("unexpected start selection".into()),
        }
    }
    assert_eq!(starts, 1);
    assert_eq!(statuses, 1);
    Ok(())
}
#[test]
fn a_failed_pass_end_still_attempts_the_start_receipt() {
    let mut recorded = false;
    let ended = Err(ServerError::AgentPassRefused {
        reason: "durable ending failed".to_owned(),
    });
    let answer = record_after_end(ended, || {
        recorded = true;
        Ok(())
    });
    assert!(recorded);
    assert!(matches!(answer, Err(ServerError::AgentPassRefused { .. })));
}
#[test]
fn failed_pass_end_and_receipt_both_name_their_failure() {
    let answer: Result<(), ServerError> = record_after_end(
        Err(ServerError::AgentPassRefused {
            reason: "durable ending failed".to_owned(),
        }),
        || {
            Err(ServerError::RuntimeUnavailable {
                reason: "receipt append failed".to_owned(),
            })
        },
    );
    let error = answer
        .err()
        .map(|error| error.to_string())
        .unwrap_or_default();
    assert!(error.contains("durable ending failed"));
    assert!(error.contains("receipt append failed"));
}
