#![cfg(test)]
//! Seat operations use the runner's PTY, boundaries and exit evidence.

#[path = "support/runner_start.rs"]
pub mod support;

use std::error::Error;
use std::os::unix::fs::PermissionsExt;
use std::sync::Arc;

use lys_runner::operations::OperationState;
use lys_runner::tracking::{CLAUDE_ADAPTER, Harness, Tracking};
use lys_runner::{Act, Answer, Client, Launch, Sessions};
use serde_json::json;
use sha2::{Digest, Sha256};
use support::{Table, operation};

type TestResult = Result<(), Box<dyn Error>>;

async fn seated(exits: bool) -> Result<Table, Box<dyn Error>> {
    let table = Table::set().await?;
    let program = table.dir.path().join("declared-harness");
    let exit = if exits {
        "exit 0"
    } else {
        "printf 'ignored-exit\\n'"
    };
    std::fs::write(
        &program,
        format!(
            "#!/bin/sh\nif [ \"$1\" = --version ]; then echo 2.1.285; exit 0; fi\nstty -echo\nprintf 'pty-ready\\n'\nesc=$(printf '\\033')\nwhile IFS= read -r line; do\nline=${{line#\"$esc[200~\"}}\nline=${{line%\"$esc[201~\"}}\ncase \"$line\" in /exit) {exit};; *) printf 'received:%s\\n' \"$line\";; esac\ndone\n"
        ),
    )?;
    std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o700))?;
    let machine = table.lys_machine("Seat box").await?;
    table
        .ok(
            "/seats",
            &json!({
                "operation": operation()?, "name": "waffles", "agent": table.agent(),
                "profile_version": 1, "machine": machine,
            }),
        )
        .await?;
    Ok(table)
}

fn sessions(table: &Table) -> Result<Arc<Sessions>, Box<dyn Error>> {
    Ok(Arc::clone(
        table.serving.as_ref().ok_or("no runner")?.sessions(),
    ))
}

fn client(table: &Table) -> Client {
    Client::new(
        table.dir.path().join("runner.sock"),
        Arc::clone(&table.server_key),
    )
}

fn hook(table: &Table, session: &str, event: &str) -> TestResult {
    sessions(table)?.collect(
        session,
        &lys_runner::peer::Collected::Hook {
            event: event.to_owned(),
            input: json!({}),
        },
    )?;
    Ok(())
}

fn wait(table: &Table, session: &str, pattern: &str) -> TestResult {
    let answer = client(table).ask(&Act::Wait {
        session: session.to_owned(),
        cursor: Some(0),
        pattern: pattern.to_owned(),
        regex: false,
    })?;
    assert!(matches!(answer, Answer::Matched { .. }), "{answer:?}");
    Ok(())
}

fn delivery_recorded(table: &Table, session: &str, operation: &str, text: &str) -> TestResult {
    let answer = client(table).ask(&Act::AsCaller {
        caller: table.person(0),
        done: Box::new(Act::Operate {
            operation: lys_runner::operations::Operation {
                operation: operation.to_owned(),
                session: session.to_owned(),
                request: lys_runner::operations::OperationRequest::Notice {
                    text: text.to_owned(),
                },
            },
        }),
    })?;
    let Answer::Operation { outcome } = answer else {
        return Err("no operation receipt".into());
    };
    assert_eq!(outcome.state, OperationState::Delivered, "{outcome:?}");
    Ok(())
}

async fn started(table: &Table, tracked: bool) -> Result<String, Box<dyn Error>> {
    let operation = operation()?;
    if tracked {
        let digest = Sha256::digest(format!("lys-identity/launch-session/v1\n{operation}"));
        let session = format!("op-{}", &lys_runner::protocol::hex(&digest)[..32]);
        let launch: Launch = serde_json::from_value(json!({
            "session": session, "program": table.dir.path().join("declared-harness"),
            "arguments": [], "directory": table.dir.path(), "environment": {},
            "columns": 80, "rows": 24,
        }))?;
        sessions(table)?.begin_for(
            launch,
            None,
            Some(Tracking {
                harness: Harness::ClaudeCode,
                adapter: CLAUDE_ADAPTER.to_owned(),
                version: "2.1.285".to_owned(),
                config_home: table.dir.path().display().to_string(),
                context_window: 200_000,
                profile_version: 1,
                account: None,
                requires_pre_tool: false,
            }),
            &table.person(0),
        )?;
    }
    let answer = table
        .ok("/seats/waffles/start", &json!({ "operation": operation }))
        .await?;
    let session = answer["session"].as_str().ok_or("no session")?.to_owned();
    wait(table, &session, "pty-ready")?;
    Ok(session)
}

#[tokio::test(flavor = "multi_thread")]
async fn seat_pty_starts_from_a_reviewed_profile_and_records_its_transport() -> TestResult {
    let table = seated(true).await?;
    let session = started(&table, false).await?;
    let live = sessions(&table)?.liveness(Some(&session))?;
    assert!(live[0].alive);
    assert!(!live[0].harness.managed);
    let (status, seat) = table
        .service
        .get("/seats/waffles", Some(&table.ada))
        .await?;
    assert_eq!(status, 200, "{seat}");
    assert_eq!(seat["transport"], "pty", "{seat}");
    table.close()
}

#[tokio::test(flavor = "multi_thread")]
async fn seat_pty_idle_stop_delivers_exit_and_observes_process_exit() -> TestResult {
    let table = seated(true).await?;
    let session = started(&table, false).await?;
    hook(&table, &session, "Stop")?;
    let operation = operation()?;
    let stopped = table
        .ok("/seats/waffles/stop", &json!({ "operation": operation }))
        .await?;
    assert_eq!(stopped["ended"], true, "{stopped}");
    let receipt = sessions(&table)?.outcome(&operation)?;
    assert_eq!(receipt.request, "notice");
    assert_eq!(
        receipt.text,
        Some(lys_runner::operations::TextDigest::of("/exit"))
    );
    assert!(matches!(
        receipt.state,
        OperationState::Delivered | OperationState::Uncertain
    ));
    assert!(!sessions(&table)?.liveness(Some(&session))?[0].alive);
    assert!(
        stopped["notes"]
            .as_array()
            .ok_or("no notes")?
            .iter()
            .any(|note| note
                .as_str()
                .is_some_and(|note| note.contains("reaped the process after the stop"))),
        "{stopped}"
    );
    table.close()
}

#[tokio::test(flavor = "multi_thread")]
async fn seat_pty_ignoring_exit_keeps_the_stop_until_end_now() -> TestResult {
    let table = seated(false).await?;
    let session = started(&table, false).await?;
    hook(&table, &session, "Stop")?;
    // The harness prints ignored-exit when it reads the stop's /exit: the
    // event that the stop was delivered and is waiting on the process.
    let watcher = client(&table);
    let ignoring = session.clone();
    let ignored = tokio::task::spawn_blocking(move || {
        watcher.ask(&Act::Wait {
            session: ignoring,
            cursor: Some(0),
            pattern: "ignored-exit".to_owned(),
            regex: false,
        })
    });
    let stop = json!({ "operation": operation()? });
    let stopping = table.ok("/seats/waffles/stop", &stop);
    let ending = async {
        let seen = ignored.await??;
        assert!(matches!(seen, Answer::Matched { .. }), "{seen:?}");
        table
            .ok(&format!("/runtime/sessions/{session}/end"), &json!({}))
            .await
    };
    let (stopped, ended) = tokio::join!(stopping, ending);
    let (stopped, ended) = (stopped?, ended?);
    assert_eq!(ended["answer"]["kind"], "ended", "{ended}");
    assert_eq!(stopped["ended"], true, "{stopped}");
    let notes = stopped["notes"].as_array().ok_or("no notes")?;
    assert!(
        notes.iter().all(|note| note
            .as_str()
            .is_some_and(|note| !note.contains("stop_wait"))),
        "{stopped}"
    );
    assert!(!sessions(&table)?.liveness(Some(&session))?[0].alive);
    let answer = client(&table).ask(&Act::Read {
        session,
        cursor: Some(0),
        lines: None,
        bytes: None,
        follow: false,
    })?;
    let Answer::Output { output } = answer else {
        return Err("no ended terminal output".into());
    };
    assert!(output.text.contains("ignored-exit"), "{output:?}");
    table.close()
}

#[tokio::test(flavor = "multi_thread")]
async fn seat_pty_restart_observes_old_exit_and_keeps_the_profile_version() -> TestResult {
    let table = seated(true).await?;
    let old = started(&table, false).await?;
    hook(&table, &old, "Stop")?;
    let path = format!("/agents/{}/provisioning", table.agent());
    table
        .ok(
            &path,
            &json!({
                "operation": operation()?, "from_version": 1,
                "model_access": ["claude-fable-5-1"], "tools": [], "skills": [],
                "mcp_servers": [], "instructions": "", "note": "",
                "harness": table.harness(), "permissions": {"default_mode": "plan"},
                "working_folder": table.dir.path(),
            }),
        )
        .await?;
    table
        .ok(
            &format!("{path}/2/review"),
            &json!({"operation": operation()?}),
        )
        .await?;
    let restart_operation = operation()?;
    let restarted = table
        .ok(
            "/seats/waffles/restart",
            &json!({ "operation": restart_operation }),
        )
        .await?;
    let new = restarted["session"]
        .as_str()
        .ok_or("no restarted session")?;
    assert_ne!(old, new);
    assert_eq!(restarted["stopped"], old);
    let digest = Sha256::digest(format!(
        "lys-identity/seat-act/v1\n{restart_operation}\nstop"
    ));
    let stop_operation = format!("op-{}", &lys_runner::protocol::hex(&digest)[..32]);
    let stop = sessions(&table)?.outcome(&stop_operation)?;
    assert_eq!(stop.request, "notice");
    assert_eq!(
        stop.text,
        Some(lys_runner::operations::TextDigest::of("/exit"))
    );
    assert_eq!(restarted["seat"]["profile_version"], 1);
    assert!(!sessions(&table)?.liveness(Some(&old))?[0].alive);
    assert!(sessions(&table)?.liveness(Some(new))?[0].alive);
    wait(&table, new, "pty-ready")?;
    table.close()
}

#[tokio::test(flavor = "multi_thread")]
async fn seat_pty_send_waits_for_the_signalled_boundary_and_returns_its_operation() -> TestResult {
    let table = seated(true).await?;
    let session = started(&table, true).await?;
    hook(&table, &session, "UserPromptSubmit")?;
    let operation = operation()?;
    let body = json!({ "operation": operation, "text": "boundary-message" });
    let sent = table.ok("/seats/waffles/send", &body).await?;
    assert_eq!(sent["operation"]["operation"], operation, "{sent}");
    assert_eq!(sent["operation"]["state"], "accepted", "{sent}");
    assert_eq!(sent["delivered"], false, "{sent}");
    assert_eq!(
        sessions(&table)?.outcome(&operation)?.state,
        OperationState::Accepted
    );
    let read = client(&table).ask(&Act::Read {
        session: session.clone(),
        cursor: Some(0),
        lines: None,
        bytes: None,
        follow: false,
    })?;
    let Answer::Output { output } = read else {
        return Err("no output".into());
    };
    assert!(!output.text.contains("received:boundary-message"));
    hook(&table, &session, "Stop")?;
    wait(&table, &session, "received:boundary-message")?;
    delivery_recorded(&table, &session, &operation, "boundary-message")?;
    assert_eq!(
        sessions(&table)?.outcome(&operation)?.state,
        OperationState::Delivered
    );
    let repeated = table.ok("/seats/waffles/send", &body).await?;
    assert_eq!(repeated["operation"]["state"], "delivered", "{repeated}");
    assert_eq!(repeated["delivered"], true);
    let read = client(&table).ask(&Act::Read {
        session: session.clone(),
        cursor: Some(0),
        lines: None,
        bytes: None,
        follow: false,
    })?;
    let Answer::Output { output } = read else {
        return Err("no output".into());
    };
    assert_eq!(output.text.matches("received:boundary-message").count(), 1);
    table.close()
}

#[test]
fn seat_config_refuses_the_retired_argus_key_and_names_the_line_to_delete() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("identity.json");
    std::fs::write(
        &path,
        "{\n  \"LYS_MONITOR_URL\": \"http://127.0.0.1:9\"\n}\n",
    )?;
    let error = lys_identity_server::Config::load(&path).expect_err("retired key accepted");
    let words = error.to_string();
    assert!(words.contains("LYS_MONITOR_URL"), "{words}");
    assert!(
        words.contains("delete") && words.contains("line 2"),
        "{words}"
    );
    std::fs::write(
        &path,
        "{\n  \"unknown_setting\": true,\n  \"LYS_MONITOR_URL\": \"http://127.0.0.1:9\"\n}\n",
    )?;
    let error = lys_identity_server::Config::load(&path).expect_err("retired key accepted");
    let words = error.to_string();
    assert!(words.contains("LYS_MONITOR_URL"), "{words}");
    assert!(words.contains("delete line 3"), "{words}");
    assert!(!words.contains("unknown field"), "{words}");
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn seat_pty_busy_stop_is_refused_and_force_bypasses_exit_delivery() -> TestResult {
    let table = seated(false).await?;
    let session = started(&table, false).await?;
    hook(&table, &session, "UserPromptSubmit")?;
    let operation = operation()?;
    let (status, refused) = table
        .service
        .post(
            "/seats/waffles/stop",
            Some(&table.ada),
            &json!({"operation": operation}),
        )
        .await?;
    assert_eq!(status, 409, "{refused}");
    assert_eq!(refused["refusal"], "seat_turn_in_progress");
    assert!(sessions(&table)?.liveness(Some(&session))?[0].alive);
    let stopped = table
        .ok(
            "/seats/waffles/stop",
            &json!({"operation": operation, "force": true}),
        )
        .await?;
    assert_eq!(stopped["ended"], true, "{stopped}");
    let error = sessions(&table)?
        .outcome(&operation)
        .expect_err("force submitted an operation");
    assert_eq!(error.name(), "operation_unknown");
    assert!(!sessions(&table)?.liveness(Some(&session))?[0].alive);
    table.close()
}
