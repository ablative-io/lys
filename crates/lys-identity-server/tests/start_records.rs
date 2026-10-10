#![cfg(test)]

//! The start route reads the server's own records: a profile version named
//! by the operation it was set with is checked against the provisioning
//! store's review, the network store's machines and egress lists, the
//! directory's lifecycle and the handles the secrets broker lists to the
//! caller, so it gives the start the start-command route admits for the
//! same agent, machine and profile; a version with no review is refused by
//! that name, and another agent's version is refused before any check.

#[path = "support/runner_start.rs"]
pub mod support;
use support::{PLANTED, Table, operation};

use std::error::Error;

use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

const ACTIVE: &str = "the agent is active";
const REVIEWED: &str = "its profile version is reviewed";
const ALLOWED: &str = "the machine is allowed for the role";
const CREDENTIALS: &str = "its virtual credentials are valid";
const REACHES: &str = "the machine may reach what the profile needs";

impl Table {
    /// Record a profile version of the seeded agent under `version`,
    /// reviewing it when `reviewed`.
    async fn version(&self, version: &str, reviewed: bool) -> TestResult {
        let path = format!("/agents/{}/provisioning", self.agent());
        let body = json!({
            "operation": version, "from_version": 0,
            "model_access": ["claude-fable-5-1"], "tools": [], "skills": [],
            "mcp_servers": [{ "name": "cambium", "url": "https://cambium.example.test/mcp" }],
            "instructions": "", "note": "",
            "harness": self.harness(),
            "permissions": {"default_mode": "plan"},
            "working_folder": self.dir.path(),
        });
        self.ok(&path, &body).await?;
        if reviewed {
            self.ok(
                &format!("{path}/1/review"),
                &json!({ "operation": operation()? }),
            )
            .await?;
        }
        Ok(())
    }

    /// A machine in use that lists the seeded agent and may reach the
    /// profile's server, with no runner recorded.
    async fn listing_machine(&self) -> Result<String, Box<dyn Error>> {
        let body = json!({
            "operation": operation()?, "name": "Build box", "kind": "server",
            "runtime": "sh", "slots": 1, "may_run": [self.agent()],
            "may_reach": ["cambium.example.test"],
        });
        self.machine(&body, None).await
    }

    /// Ask the start route for the seeded agent.
    async fn given(&self, version: &str, machine: &str) -> Result<(u16, Value), Box<dyn Error>> {
        let path = format!("/agents/{}/start", self.agent());
        let body = json!({ "profile_version": version, "machine": machine });
        self.service.post(&path, Some(&self.ada), &body).await
    }
}

/// Each check's result in `answer`, by the check's name.
fn result<'a>(answer: &'a Value, check: &str) -> &'a str {
    answer["checks"]
        .as_array()
        .and_then(|checks| checks.iter().find(|report| report["check"] == check))
        .and_then(|report| report["result"].as_str())
        .unwrap_or("absent")
}

/// The name of each refusal in `answer`, in order.
fn refusals(answer: &Value) -> Vec<&str> {
    answer["refused"]
        .as_array()
        .map(|refused| {
            refused
                .iter()
                .filter_map(|refusal| refusal["refusal"].as_str())
                .collect()
        })
        .unwrap_or_default()
}

#[tokio::test]
async fn a_start_passes_the_checks_the_start_command_passes() -> TestResult {
    let table = Table::unprofiled().await?;
    let version = operation()?;
    table.version(&version, true).await?;
    let machine = table.listing_machine().await?;

    // The start command passes its agent, review, placement and egress
    // checks and is refused only after them, for want of a runner.
    let (status, commanded) = table
        .start(
            &table.agent(),
            &json!({ "machine": machine, "operation": operation()? }),
        )
        .await?;
    assert_ne!(status, 200, "{commanded}");
    assert_eq!(commanded["refusal"], "MachineWithoutRunner", "{commanded}");

    // The same agent, machine and profile are given their start here, with
    // the handle the broker lists to the caller; the machine names no
    // runner, so the command is answered and nothing is run.
    let (status, given) = table.given(&version, &machine).await?;
    assert_eq!(status, 200, "{given}");
    for check in [ACTIVE, REVIEWED, ALLOWED, CREDENTIALS, REACHES] {
        assert_eq!(result(&given, check), "passed", "{check}: {given}");
    }
    let record = &given["launch_record"];
    assert_eq!(record["agent"], table.agent(), "{given}");
    assert_eq!(record["machine"], machine.as_str(), "{given}");
    assert_eq!(record["profile_version"], version.as_str(), "{given}");
    assert_eq!(record["credential_ids"], json!(["h-live"]), "{given}");
    assert_eq!(record["executable"], table.harness()["program"], "{given}");
    assert_eq!(
        given["working_directory"],
        json!(table.dir.path()),
        "{given}"
    );
    assert_eq!(given["state"], "unconfirmed", "{given}");
    assert!(given.get("runner").is_none(), "{given}");
    assert!(!given.to_string().contains(PLANTED), "{given}");
    table.close()
}

#[tokio::test]
async fn a_version_with_no_review_is_refused_by_that_name() -> TestResult {
    let table = Table::unprofiled().await?;
    let version = operation()?;
    table.version(&version, false).await?;
    let machine = table.listing_machine().await?;

    let (status, given) = table.given(&version, &machine).await?;
    assert_eq!(status, 409, "{given}");
    assert_eq!(
        result(&given, REVIEWED),
        "profile_version_not_reviewed",
        "{given}"
    );
    assert!(
        refusals(&given).contains(&"profile_version_not_reviewed"),
        "{given}"
    );
    for check in [ACTIVE, ALLOWED, REACHES] {
        assert_eq!(result(&given, check), "passed", "{check}: {given}");
    }
    assert!(given.to_string().contains(&version), "{given}");

    // A version the provisioning store does not keep has no review either.
    let absent = operation()?;
    let (status, given) = table.given(&absent, &machine).await?;
    assert_eq!(status, 409, "{given}");
    assert_eq!(
        result(&given, REVIEWED),
        "profile_version_not_reviewed",
        "{given}"
    );
    table.close()
}

#[tokio::test]
async fn a_machine_that_does_not_take_the_agent_is_refused_by_name() -> TestResult {
    let table = Table::unprofiled().await?;
    let version = operation()?;
    table.version(&version, true).await?;
    let body = json!({
        "operation": operation()?, "name": "Other box", "kind": "server",
        "runtime": "sh", "slots": 1, "may_run": [], "may_reach": [],
    });
    let elsewhere = table.machine(&body, None).await?;

    let (status, given) = table.given(&version, &elsewhere).await?;
    assert_eq!(status, 409, "{given}");
    assert_eq!(
        result(&given, ALLOWED),
        "machine_not_allowed_for_role",
        "{given}"
    );
    assert_eq!(result(&given, REACHES), "egress_not_reachable", "{given}");
    assert_eq!(result(&given, REVIEWED), "passed", "{given}");
    table.close()
}

#[tokio::test]
async fn another_agents_version_is_refused_before_any_check() -> TestResult {
    let table = Table::unprofiled().await?;
    let version = operation()?;
    table.version(&version, true).await?;
    let other = table.agent_of(1).ok_or("no second seeded agent")?;
    let body = json!({
        "operation": operation()?, "name": "Shared box", "kind": "server",
        "runtime": "sh", "slots": 1, "may_run": [table.agent(), other],
        "may_reach": ["cambium.example.test"],
    });
    let machine = table.machine(&body, None).await?;

    let path = format!("/agents/{other}/start");
    let body = json!({ "profile_version": version, "machine": machine });
    let (status, refused) = table.service.post(&path, Some(&table.ada), &body).await?;
    assert_ne!(status, 200, "{refused}");
    assert_eq!(refused["refusal"], "LaunchUnrenderable", "{refused}");
    assert!(refused.get("checks").is_none(), "{refused}");
    table.close()
}
