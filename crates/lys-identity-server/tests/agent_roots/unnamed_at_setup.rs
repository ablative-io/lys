//! AGENTS-017 R4: when setup cannot name the computer Lys runs on, setup still
//! finishes and its answer says `own_computer_unnamed` by name; the
//! administrator's one standing choice then posts agent-roots, which names the
//! computer before it gives any root, the four grants are given from those
//! roots, and a start on that computer is admitted. A name that failed at
//! setup is never a dead end.
//!
//! The failing write is the network store's own: the network file's path is
//! held by a directory while setup runs, so the store's write and its re-read
//! both fail, and the store reads the file again on its next use once the
//! directory is gone. No clocks: every step ends on the service's answer.

use std::error::Error;

use identity_contract::apps::{Auth, get, op, post};
use lys_identity::grants::shipped_model;
use serde_json::{Value, json};

use super::support::Table;
use super::{agent_roots, give_five, ok, own_computers};

type TestResult = Result<(), Box<dyn Error>>;

#[tokio::test(flavor = "multi_thread")]
async fn a_computer_setup_could_not_name_is_named_by_agent_roots_and_a_start_on_it_works()
-> TestResult {
    let table = Table::unseeded(&shipped_model()).await?;
    let walked = walk(&table).await;
    table.close()?;
    walked
}

async fn walk(table: &Table) -> TestResult {
    let service = &table.service;
    let auth = Auth::Cookie(&table.ada);

    // The network file's path is a directory while setup runs: its write fails.
    let network = service.dir.path().join("network.json");
    let kept = std::fs::read(&network).ok();
    if kept.is_some() {
        std::fs::remove_file(&network)?;
    }
    std::fs::create_dir(&network)?;
    std::fs::write(network.join("held"), b"the network file's path is taken")?;
    let set_up = ok(post(
        service,
        "/setup",
        auth,
        &json!({"operation": op()?, "display_name": "Administrator"}),
    )
    .await?)?;
    let person = set_up["person"].as_str().ok_or("no person")?.to_owned();
    let unnamed = set_up["own_computer_unnamed"]
        .as_str()
        .ok_or_else(|| format!("setup did not say the computer went unnamed: {set_up}"))?;
    assert!(unnamed.starts_with("own_computer_unnamed: "), "{unnamed}");
    // Setup still recorded the directory roots, and no root on a computer.
    let roots = agent_roots(service, &table.ada).await?;
    assert!(!roots.is_empty(), "setup recorded no roots");
    assert!(
        roots
            .iter()
            .all(|root| root["resource"]["kind"] != "machine"),
        "{roots:?}"
    );

    // The path is the store's again.
    std::fs::remove_dir_all(&network)?;
    if let Some(bytes) = kept {
        std::fs::write(&network, bytes)?;
    }

    // The one standing choice: agent-roots first, which names the computer.
    let agent = ok(post(
        service,
        "/agents",
        auth,
        &json!({"operation": op()?, "display_name": "Builder"}),
    )
    .await?)?["agent"]
        .as_str()
        .ok_or("no agent")?
        .to_owned();
    ok(post(
        service,
        &format!("/identities/{agent}/transitions"),
        auth,
        &json!({"operation": op()?, "transition": "activate", "reason": ""}),
    )
    .await?)?;
    ok(post(service, "/grants/agent-roots", auth, &json!({})).await?)?;
    let computers = own_computers(service, &table.ada).await?;
    assert_eq!(computers.len(), 1, "{computers:?}");
    let machine = computers[0]["id"]
        .as_str()
        .ok_or("no machine id")?
        .to_owned();
    assert_eq!(computers[0]["named_by"], person.as_str(), "{computers:?}");
    let runner = ok(get(
        service,
        &format!("/network/machines/{machine}/runner"),
        auth,
    )
    .await?)?;
    assert_eq!(runner["runner"]["kind"], "lys", "{runner}");

    let roots = agent_roots(service, &table.ada).await?;
    let on = |resource: Value| -> Result<Value, Box<dyn Error>> {
        roots
            .iter()
            .find(|root| root["resource"] == resource)
            .cloned()
            .ok_or_else(|| format!("no root on {resource}").into())
    };
    let own = on(json!({"kind": "person", "id": person}))?;
    let computer = on(json!({"kind": "machine", "id": machine}))?;
    let held = give_five(service, auth, (&own, &computer), (&person, &agent)).await?;
    assert_eq!(held.len(), 5, "{held:?}");

    // A reviewed profile, the agent allowed on the computer, and a start on it.
    let path = format!("/agents/{agent}/provisioning");
    table
        .ok(
            &path,
            &json!({
                "operation": op()?, "from_version": 0,
                "model_access": ["claude-fable-5-1"], "tools": [], "skills": [],
                "mcp_servers": [], "instructions": "", "note": "",
                "harness": table.harness(),
                "permissions": {"default_mode": "plan"},
                "working_folder": table.dir.path(),
            }),
        )
        .await?;
    table
        .ok(&format!("{path}/1/review"), &json!({"operation": op()?}))
        .await?;
    table
        .ok(
            &format!("/network/machines/{machine}/agents"),
            &json!({"operation": op()?, "agent": agent, "allow": true}),
        )
        .await?;
    let (status, started) = table
        .start(&agent, &json!({"machine": machine, "operation": op()?}))
        .await?;
    assert_eq!(status, 200, "{started}");
    assert_eq!(started["machine"], machine.as_str(), "{started}");
    assert_eq!(started["provisioning_version"], 1, "{started}");
    Ok(())
}
