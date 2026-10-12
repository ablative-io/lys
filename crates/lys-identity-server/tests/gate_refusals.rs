//! The runner refusals published by the service are exercised at their boundaries.

use lys_runner::peer::{Leader, StartIdentity};
use lys_runner::seat_owner::protocol::OwnerBinding;
use lys_runner::seat_owner::store::{
    Cursors, Custody, Holder, Intent, Lease, OwnerRecord, OwnerStore,
};
use serde_json::json;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn invalid_owner_bindings_are_refused_before_use() -> TestResult {
    let binding = OwnerBinding {
        seat: "../seat".to_owned(),
        session: "session".to_owned(),
        conversation: String::new(),
        generation: 1,
    };
    let error = binding.validate().err().ok_or("path binding admitted")?;
    assert_eq!(error.name(), "seat_owner_binding_invalid");
    Ok(())
}

#[test]
fn a_proved_conversation_cannot_replace_the_owned_conversation() -> TestResult {
    let directory = tempfile::tempdir()?;
    let mut store = OwnerStore::open(directory.path())?;
    let record = OwnerRecord {
        seat: "seat".to_owned(),
        session: "session".to_owned(),
        conversation: "first".to_owned(),
        harness: None,
        lease: Lease {
            generation: 1,
            holder: Holder::Owner {
                start: Leader {
                    pid: 42,
                    start: StartIdentity("fixture-start".to_owned()),
                },
            },
            taken_at: 1,
            build: "fixture".to_owned(),
        },
        custody: Custody::Owned,
        cursors: Cursors::default(),
        credential_references: Vec::new(),
        established_at: 1,
    };
    store.record(
        &format!("{:032x}", 1),
        &Intent::Establish {
            record: Box::new(record),
        },
    )?;
    let error = store
        .record(
            &format!("{:032x}", 2),
            &Intent::Conversation {
                session: "session".to_owned(),
                conversation: "replacement".to_owned(),
            },
        )
        .err()
        .ok_or("owned conversation replaced")?;
    assert_eq!(error.name(), "seat_owner_binding_mismatch");
    assert_eq!(
        store.owner("session").ok_or("owner missing")?.conversation,
        "first"
    );
    assert_eq!(
        OwnerStore::open(directory.path())?
            .owner("session")
            .ok_or("reopened owner missing")?
            .conversation,
        "first"
    );
    Ok(())
}

#[test]
fn an_enabled_uninstalled_plugin_refuses_before_publication_or_spawn() -> TestResult {
    use sha2::{Digest, Sha256};
    let directory = tempfile::tempdir()?;
    let config_home = tempfile::tempdir()?;
    let sessions = lys_runner::Sessions::open(directory.path(), 4096)?;
    let settings = json!({"enabledPlugins":{"uninstalled@fixture":true}}).to_string();
    let launch = serde_json::from_value(json!({
        "session":"plugin-session", "program":"/bin/sh", "arguments":["-c", "exit 99"],
        "directory":directory.path(), "columns":80, "rows":24,
        "environment":{"CLAUDE_CONFIG_DIR":config_home.path()},
        "config":{"files":[{"path":"settings.json", "text":settings,
            "sha256":format!("{:x}", Sha256::digest(settings.as_bytes()))}],
            "argument_files":{}, "environment_paths":{}, "working_directory":false, "harness":"claude_code"}
    }))?;
    let error = sessions
        .start(launch)
        .err()
        .ok_or("uninstalled plugin started")?;
    assert_eq!(error.name(), "plugin_not_installed");
    assert!(
        !directory
            .path()
            .join("sessions/plugin-session/config")
            .exists()
    );
    sessions.stop_all()?;
    Ok(())
}
