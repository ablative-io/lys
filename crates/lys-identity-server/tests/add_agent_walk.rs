#![cfg(test)]
//! The add-agent requests must reach a live runner before a start is confirmed.

#[path = "support/runner_start.rs"]
mod support;

use std::error::Error;
use std::sync::Arc;

use lys_runner::protocol::StatusView;
use lys_runner::{Act, Answer, Client};
use serde_json::{Value, json};
use support::{Table, operation};

type TestResult = Result<(), Box<dyn Error>>;

struct Evidence {
    agent: String,
    machine: String,
    reviewed: Value,
    started: Value,
    session: String,
    status: StatusView,
    live: Value,
}

async fn profile(table: &Table, agent: &str) -> Result<Value, Box<dyn Error>> {
    let (code, fixture) = table
        .service
        .get(
            &format!("/agents/{}/provisioning", table.agent()),
            Some(&table.ada),
        )
        .await?;
    if code != 200 {
        return Err(format!("fixture profile answered {code}: {fixture}").into());
    }
    let harness = fixture["profile"]["harness"]
        .as_object()
        .ok_or("fixture profile has no declared harness")?;
    let path = format!("/agents/{agent}/provisioning");
    // The folder is named per agent and must exist: the runner starts the run
    // in it and refuses an empty name rather than falling back to its own.
    let working_folder = table.dir.path().join("work");
    std::fs::create_dir_all(&working_folder)?;
    let working_folder = working_folder
        .to_str()
        .ok_or("the working folder's path is not UTF-8")?
        .to_owned();
    table
        .ok(
            &path,
            &json!({
                "operation": operation()?, "from_version": 0,
                "model_access": ["claude-fable-5-1"], "tools": [], "skills": [], "mcp_servers": [],
                "instructions": "", "instructions_mode": "keep", "note": "Start this agent",
                "harness": harness, "permissions": { "default_mode": "plan" },
                "working_folder": working_folder,
            }),
        )
        .await?;
    table
        .ok(
            &format!("{path}/1/review"),
            &json!({ "operation": operation()? }),
        )
        .await
}

async fn walk(table: &Table) -> Result<Evidence, Box<dyn Error>> {
    let registered = table
        .ok(
            "/agents",
            &json!({ "operation": operation()?, "display_name": "Pancake" }),
        )
        .await?;
    let agent = registered["agent"]
        .as_str()
        .ok_or("agent registration has no id")?
        .to_owned();
    table
        .ok(
            &format!("/identities/{agent}/transitions"),
            &json!({ "operation": operation()?, "transition": "activate", "reason": "Start this agent" }),
        )
        .await?;

    let reviewed = profile(table, &agent).await?;

    let machine = table
        .machine(
            &json!({
                "operation": operation()?, "name": "This computer", "kind": "Computer", "runtime": "lys-runner",
                "slots": 0, "may_run": [agent], "may_run_roles": [], "may_reach": [],
            }),
            Some(json!({ "kind": "lys" })),
        )
        .await?;
    let (code, started) = table
        .start(
            &agent,
            &json!({ "machine": machine, "operation": operation()? }),
        )
        .await?;
    if code != 200 || started["runner"]["state"] != "running" {
        return Err(
            format!("start did not confirm a running runner: HTTP {code}: {started}").into(),
        );
    }
    let session = started["session"]
        .as_str()
        .ok_or("start has no session id")?
        .to_owned();
    let client = Client::new(
        table.dir.path().join("runner.sock"),
        Arc::clone(&table.server_key),
    );
    let Answer::Matched { .. } = client.ask(&Act::Wait {
        session: session.clone(),
        cursor: Some(0),
        pattern: "profile-read".to_owned(),
        regex: false,
    })?
    else {
        return Err("the runner program did not signal that it read its profile".into());
    };
    let Answer::Status { status } = client.ask(&Act::Status {
        session: Some(session.clone()),
    })?
    else {
        return Err("the runner answered no session status".into());
    };
    let (code, live) = table.service.get("/runtime/live", Some(&table.ada)).await?;
    if code != 200 {
        return Err(format!("live sessions answered {code}: {live}").into());
    }
    Ok(Evidence {
        agent,
        machine,
        reviewed,
        started,
        session,
        status,
        live,
    })
}

#[tokio::test(flavor = "multi_thread")]
async fn adding_an_agent_and_this_computer_starts_a_real_runner_session() -> TestResult {
    let table = Table::set().await?;
    let walked = walk(&table).await;
    let Evidence {
        agent,
        machine,
        reviewed,
        started,
        session,
        status,
        live,
    } = match (walked, table.close()) {
        (Ok(evidence), Ok(())) => evidence,
        (Err(error), Ok(())) | (Ok(_), Err(error)) => return Err(error),
        (Err(walk), Err(close)) => {
            return Err(
                format!("add-agent walk failed: {walk}; runner cleanup failed: {close}").into(),
            );
        }
    };

    assert_eq!(reviewed["profile"]["version"], 1, "{reviewed}");
    assert!(reviewed["profile"]["reviewed_by"].is_string(), "{reviewed}");
    assert_eq!(started["agent"], agent, "{started}");
    assert_eq!(started["machine"], machine, "{started}");
    assert_eq!(started["runtime"], "lys-runner", "{started}");
    assert_eq!(started["provisioning_version"], 1, "{started}");
    assert_eq!(started["runner"]["session"], session, "{started}");
    assert_eq!(started["runner"]["state"], "running", "{started}");
    assert!(
        started["runner"]["pid"].as_u64().is_some_and(|pid| pid > 0),
        "{started}"
    );
    let held = status
        .sessions
        .iter()
        .find(|held| held.session == session)
        .ok_or("runner did not hold the returned session")?;
    assert!(held.ended.is_none(), "{held:?}");
    assert!(held.pid.is_some_and(|pid| pid > 0), "{held:?}");
    let listed = live["sessions"]
        .as_array()
        .and_then(|all| all.iter().find(|entry| entry["session"] == session))
        .ok_or("the returned runner session is not listed live")?;
    assert_eq!(listed["agent"], agent, "{listed}");
    assert_eq!(listed["machine"], machine, "{listed}");
    assert_eq!(listed["shown"], "running", "{listed}");
    assert_eq!(live["unanswered"], json!([]), "{live}");
    println!("add-agent runner session: {session}");
    Ok(())
}
