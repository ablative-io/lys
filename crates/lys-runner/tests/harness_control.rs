//! Managed launch boundaries preserve manual input and name refused controls.

#![cfg(test)]

use std::collections::BTreeMap;
use std::error::Error;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use lys_core::Ed25519Identity;
use lys_runner::protocol::{Greeting, Request, hex, sign_request, signed_bytes};
use lys_runner::{Act, Answer, Launch, PROTOCOL_VERSION, Sessions};

type Result<T = ()> = std::result::Result<T, Box<dyn Error>>;

fn launch() -> Launch {
    Launch {
        session: "manual".to_owned(),
        program: "/bin/sh".to_owned(),
        arguments: vec!["-c".to_owned(), "stty -echo; echo ready; cat".to_owned()],
        directory: "/".to_owned(),
        environment: BTreeMap::new(),
        config: None,
        columns: 80,
        rows: 24,
        rotation: None,
        policy: None,
    }
}

fn with_sessions(test: impl FnOnce(&Arc<Sessions>, &std::path::Path) -> Result) -> Result {
    let directory = tempfile::tempdir()?;
    let sessions = Sessions::open(directory.path(), 4096)?;
    let result = test(&sessions, directory.path());
    sessions.stop_all()?;
    result
}

#[test]
fn a_pty_only_session_requiring_automatic_controls_refuses_control_transport_unsupported() -> Result
{
    with_sessions(|sessions, directory| {
        let key = Ed25519Identity::load_or_generate(&directory.join("server.key"))?;
        let greeting = Greeting {
            version: PROTOCOL_VERSION,
            runner: sessions.runner().to_owned(),
            challenge: "01".repeat(32),
        };
        let act = serde_json::json!({
            "act": "start_managed",
            "managed": {
                "launch": launch(),
                "transport": {"mode": "pty"},
                "conversation": "conversation",
                "requires_controls": true
            },
            "lys_mcp": null,
            "proxy": null
        })
        .to_string();
        let request = Request {
            version: PROTOCOL_VERSION,
            runner: greeting.runner.clone(),
            challenge: greeting.challenge.clone(),
            signature: hex(&key.sign(&signed_bytes(
                PROTOCOL_VERSION,
                &greeting.runner,
                &greeting.challenge,
                &act,
            ))),
            act,
        };
        let answer = lys_runner::socket::dispatch(
            sessions,
            &key.public_key_bytes(),
            &greeting,
            &serde_json::to_string(&request)?,
            &AtomicBool::new(false),
        );
        match answer {
            Answer::Refused { refusal, .. } if refusal == "control_transport_unsupported" => Ok(()),
            other => Err(format!("expected control_transport_unsupported, got {other:?}").into()),
        }
    })
}

#[test]
fn the_existing_manual_terminal_launch_still_accepts_manual_input() -> Result {
    with_sessions(|sessions, directory| {
        let mut manual = launch();
        manual.directory = directory.display().to_string();
        sessions.start(manual)?;
        let key = Ed25519Identity::load_or_generate(&directory.join("server.key"))?;
        let greeting = Greeting {
            version: PROTOCOL_VERSION,
            runner: sessions.runner().to_owned(),
            challenge: "01".repeat(32),
        };
        let ready = Act::Wait {
            session: "manual".to_owned(),
            cursor: Some(0),
            pattern: "ready".to_owned(),
            regex: false,
        };
        match lys_runner::socket::dispatch(
            sessions,
            &key.public_key_bytes(),
            &greeting,
            &sign_request(&key, &greeting, &ready)?,
            &AtomicBool::new(false),
        ) {
            Answer::Matched { .. } => {}
            other => return Err(format!("terminal did not become ready: {other:?}").into()),
        }
        sessions.input("manual", "manual input", true)?;
        let act = Act::Wait {
            session: "manual".to_owned(),
            cursor: Some(0),
            pattern: "manual input".to_owned(),
            regex: false,
        };
        let answer = lys_runner::socket::dispatch(
            sessions,
            &key.public_key_bytes(),
            &greeting,
            &sign_request(&key, &greeting, &act)?,
            &AtomicBool::new(false),
        );
        match answer {
            Answer::Matched { matched, .. } if matched.contains("manual input") => Ok(()),
            other => Err(format!("manual input was not returned: {other:?}").into()),
        }
    })
}

#[test]
fn an_expired_managed_event_cursor_names_the_coverage_gap() -> Result {
    let first = tempfile::tempdir()?;
    let second = tempfile::tempdir()?;
    let first_feed = lys_runner::tracking_store::Feed::open(first.path())?;
    let second_feed = lys_runner::tracking_store::Feed::open(second.path())?;
    let cursor = first_feed.page(None)?.cursor;
    let error = second_feed
        .page(Some(&cursor))
        .err()
        .ok_or("expired event cursor silently began another feed")?;
    assert_eq!(error.name(), "cursor_expired");
    assert!(error.to_string().contains("feed"));
    Ok(())
}
#[test]
fn an_unqualified_selected_binary_is_named_with_its_actual_reported_version() -> Result {
    use lys_runner::harness_control::{ManagedLaunch, Transport};
    use std::os::unix::fs::PermissionsExt;
    with_sessions(|sessions, directory| {
        let executable = directory.join("fixture-harness");
        let marker = directory.join("executed");
        std::fs::write(
            &executable,
            "#!/bin/sh\nif [ \"$1\" = --version ]; then echo 'fixture-harness 9.8.7'; exit 0; fi\nprintf executed > executed\n",
        )?;
        std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o700))?;
        let mut managed = launch();
        managed.directory = directory.display().to_string();
        managed.program = executable.display().to_string();
        managed.arguments.clear();
        let error = sessions
            .start_managed(
                ManagedLaunch {
                    launch: managed,
                    transport: Transport::Claude,
                    conversation: "aaaaaaaa-bbbb-4ccc-addd-eeeeeeeeeeee".to_owned(),
                    requires_controls: true,
                },
                None,
                None,
            )
            .err()
            .ok_or("an unqualified harness was launched")?;
        assert_eq!(error.name(), "control_adapter_unqualified");
        let reason = error.to_string();
        assert!(reason.contains("Claude"), "{reason}");
        assert!(
            reason.contains(executable.to_str().ok_or("fixture path was not text")?),
            "{reason}"
        );
        assert!(reason.contains("9.8.7"), "{reason}");
        assert!(!marker.exists());
        Ok(())
    })
}

#[test]
fn a_whole_version_report_is_read_before_the_unqualified_refusal() -> Result {
    use lys_runner::harness_control::{ManagedLaunch, Transport};
    use std::os::unix::fs::PermissionsExt;
    with_sessions(|sessions, directory| {
        let executable = directory.join("fixture-harness");
        std::fs::write(
            &executable,
            "#!/bin/sh\nprintf '%8192s' ''\necho 'fixture-harness 9.8.7'\n",
        )?;
        std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o700))?;
        let mut managed = launch();
        managed.directory = directory.display().to_string();
        managed.program = executable.display().to_string();
        managed.arguments.clear();
        let error = sessions
            .start_managed(
                ManagedLaunch {
                    launch: managed,
                    transport: Transport::Claude,
                    conversation: "aaaaaaaa-bbbb-4ccc-addd-eeeeeeeeeeee".to_owned(),
                    requires_controls: true,
                },
                None,
                None,
            )
            .err()
            .ok_or("an oversized version probe was accepted")?;
        assert_eq!(error.name(), "control_adapter_unqualified");
        assert!(error.to_string().contains("9.8.7"));
        Ok(())
    })
}

#[test]
fn a_large_harness_output_frame_is_read_whole() -> Result {
    let text = "image-data".repeat(200_000);
    let frame = serde_json::json!({"type":"user","message":{"content":text}});
    let mut bytes = serde_json::to_vec(&frame)?;
    bytes.push(b'\n');
    let mut reader = std::io::Cursor::new(bytes);
    assert_eq!(
        lys_runner::harness_control::process::frame(&mut reader)?,
        frame
    );
    Ok(())
}
