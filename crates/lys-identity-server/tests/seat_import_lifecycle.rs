#![cfg(test)]
//! AGENTS-003 R7: an imported seat and its import survive the surfaces and
//! services around them. Import status and the one completed manifest
//! survive a service restart and an upgrade handoff; confirming again after
//! either changes nothing and starts nothing; an upgrade owner's refusal and
//! a lost reply are named and recovered into the same operation.
//!
//! Seat survival through a real launched binary waits on DIRECTORY-064's
//! qualification pin: lys-runner's `QUALIFIED` list is empty, so no managed
//! harness can start. A seat starts through its own owner (AGENTS-004 R1),
//! and the owner asks the adapter's qualification once it runs; this
//! service's fixture runner has no owner program to start, so here the
//! imported seat's start is refused by name, `seat_owner_start_failed`,
//! before any owner or harness exists, and that is what is asserted (the
//! adapter's own `control_adapter_unqualified` is lys-runner's to prove). The survival
//! legs are written whole in `survives`, called only from
//! `seat_import_survives_lifecycle_once_qualified`, which asserts the list is
//! empty and the refusal holds, and becomes the survival test itself the day
//! the pin lands and the start answers.
//!
//! Every call goes through the public routes; every event is explicit (a
//! restart that returns when the new service is ready, an attach the test
//! closes, an intent file the test writes and removes). Nothing sleeps.

#[path = "support/runner_start.rs"]
pub mod support;

use std::error::Error;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use support::{Table, operation};

type TestResult = Result<(), Box<dyn Error>>;

const SEAT: &str = "waffles";
/// The refusal a seat's start answers under this fixture: its owner cannot
/// be started, so nothing starts.
const START_REFUSED: &str = "seat_owner_start_failed";
/// lys-runner's list of qualified launched binaries, empty until the pin.
const QUALIFIED_EMPTY: &str = "const QUALIFIED: &[Qualification] = &[];";

/// A service with one seat and its Claude resource folder.
struct Scene {
    table: Table,
    folder: PathBuf,
}

async fn scene() -> Result<Scene, Box<dyn Error>> {
    let table = Table::set().await?;
    let machine = operation()?;
    table
        .machine(
            &json!({
                "operation": machine, "name": "Box", "kind": "laptop", "runtime": "sh",
                "slots": 1, "may_run": [table.agent()], "may_reach": [],
            }),
            Some(json!({ "kind": "lys" })),
        )
        .await?;
    managed_version(&table).await?;
    table
        .ok(
            "/seats",
            &json!({
                "operation": operation()?, "name": SEAT, "agent": table.agent(),
                "profile_version": 2, "machine": machine,
            }),
        )
        .await?;
    let folder = table.dir.path().join("seat-resources").join(SEAT);
    std::fs::create_dir_all(&folder)?;
    std::fs::write(
        folder.join("settings.json"),
        json!({ "model": "claude-fable-5-1", "permissions": { "defaultMode": "plan" } })
            .to_string(),
    )?;
    std::fs::write(folder.join("system-prompt.md"), "You are Waffles.\n")?;
    std::fs::write(
        folder.join("mcp.json"),
        json!({ "mcpServers": {} }).to_string(),
    )?;
    Ok(Scene { table, folder })
}

/// Record and review the agent's profile version 2: version 1 again, its
/// sessions requiring controls, so a seat on it starts only managed.
async fn managed_version(table: &Table) -> TestResult {
    let path = format!("/agents/{}/provisioning", table.agent());
    let body = json!({
        "operation": operation()?, "from_version": 1,
        "model_access": ["claude-fable-5-1"], "tools": [], "skills": [],
        "mcp_servers": [], "instructions": "", "note": "",
        "harness": table.harness(),
        "permissions": {"default_mode": "plan"},
        "working_folder": table.dir.path(),
        "session": {"stall_after":300,"requires_controls": true,"message_prefix":"[Seat] "},
    });
    table.ok(&path, &body).await?;
    table
        .ok(
            &format!("{path}/2/review"),
            &json!({ "operation": operation()? }),
        )
        .await?;
    Ok(())
}

/// Whether lys-runner's `QUALIFIED` list is still empty, read from its
/// source as DIRECTORY-064's pin leaves it.
fn qualified_is_empty() -> Result<bool, Box<dyn Error>> {
    let process =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../lys-runner/src/harness_control/process.rs");
    Ok(std::fs::read_to_string(process)?.contains(QUALIFIED_EMPTY))
}

/// What of an answer an event must leave unchanged: everything but the
/// latest dry run, which the service keeps in memory only (R4: a restart
/// forgets it, and a person runs a new one), so it is no part of the import.
fn surviving(mut answer: Value) -> Value {
    if let Some(status) = answer.as_object_mut() {
        status.remove("previewed");
    }
    answer
}

/// The start refused by its owner start's name.
fn start_refused(answer: &(u16, Value)) {
    assert_ne!(answer.0, 200, "a managed start answered: {}", answer.1);
    assert_eq!(answer.1["refusal"], START_REFUSED, "{}", answer.1);
}

impl Scene {
    fn manifest(&self) -> Value {
        json!({
            "seat": SEAT, "harness": "claude", "codex_profile": null,
            "claude_folder": self.folder, "seat_identity_file": null, "monitor": null,
            "secret_handles": [], "recipient_map": {},
        })
    }

    async fn dry_run(&self) -> Result<Value, Box<dyn Error>> {
        let path = format!("/seats/{SEAT}/import/dry-run");
        let plan = self
            .table
            .ok(&path, &json!({ "manifest": self.manifest() }))
            .await?;
        assert_eq!(plan["refusals"], json!([]), "{plan}");
        Ok(plan)
    }

    async fn confirm(&self, plan: &Value, operation: &str) -> Result<(u16, Value), Box<dyn Error>> {
        let body = json!({
            "plan_id": plan["plan_id"], "plan_revision": plan["plan_revision"],
            "operation": operation,
        });
        let path = format!("/seats/{SEAT}/import/confirm");
        self.table
            .service
            .post(&path, Some(&self.table.ada), &body)
            .await
    }

    async fn get(&self, path: &str) -> Result<Value, Box<dyn Error>> {
        let (status, answer) = self.table.service.get(path, Some(&self.table.ada)).await?;
        if status != 200 {
            return Err(format!("{path} answered {status}: {answer}").into());
        }
        Ok(answer)
    }

    async fn status(&self) -> Result<Value, Box<dyn Error>> {
        self.get(&format!("/seats/{SEAT}/import")).await
    }

    /// What survives an event: the import status, the seat record, the
    /// running sessions and every destination the import wrote.
    async fn held(&self) -> Result<Vec<Value>, Box<dyn Error>> {
        let agent = self.table.agent();
        let mut held = Vec::new();
        for path in [
            format!("/seats/{SEAT}/import"),
            format!("/seats/{SEAT}"),
            "/runtime/live".to_owned(),
            format!("/agents/{agent}/provisioning"),
            format!("/agents/{agent}/variables"),
            "/words".to_owned(),
            "/schedules".to_owned(),
        ] {
            held.push(surviving(self.get(&path).await?));
        }
        Ok(held)
    }

    /// Plan and confirm the seat once, answering the confirmation's operation
    /// and the completed status.
    async fn imported(&self) -> Result<(Value, String, Value), Box<dyn Error>> {
        let plan = self.dry_run().await?;
        let confirming = operation()?;
        let answer = self.confirm(&plan, &confirming).await?;
        assert_eq!(answer.0, 200, "{}", answer.1);
        let status = self.status().await?;
        assert_eq!(status["state"], "completed", "{status}");
        Ok((plan, confirming, status))
    }

    /// The same confirmation again writes nothing: it answers the import it
    /// already made, or, once a restart has forgotten the dry run (the
    /// importer keeps previews in memory only), refuses it by name.
    async fn confirmed_again(&self, plan: &Value, confirming: &str) -> TestResult {
        let before = self.held().await?;
        let answer = self.confirm(plan, confirming).await?;
        if answer.0 != 200 {
            assert_eq!(answer.1["refusal"], "import_plan_unknown", "{}", answer.1);
        }
        assert_eq!(
            self.held().await?,
            before,
            "a repeated confirmation changed state"
        );
        Ok(())
    }

    /// The seat's start, as the service answers it.
    async fn start(&self) -> Result<(u16, Value), Box<dyn Error>> {
        self.table
            .service
            .post(
                &format!("/seats/{SEAT}/start"),
                Some(&self.table.ada),
                &json!({ "operation": operation()? }),
            )
            .await
    }

    /// The live sessions the seat's agent holds, as the runner reports them.
    async fn seat_sessions(&self) -> Result<Vec<Value>, Box<dyn Error>> {
        let agent = self.table.agent();
        let live = self.get("/runtime/live").await?;
        Ok(live["sessions"]
            .as_array()
            .ok_or_else(|| format!("no sessions list: {live}"))?
            .iter()
            .filter(|session| session["agent"] == agent.as_str())
            .cloned()
            .collect())
    }

    /// Open an attach, read its first bytes, and close it: the attachment
    /// exits by the test's own hand.
    async fn attach_and_exit(&self) -> TestResult {
        let mut answer = reqwest::Client::new()
            .post(format!("{}/seats/{SEAT}/attach", self.table.service.base))
            .header(reqwest::header::COOKIE, &self.table.ada)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body("{}")
            .send()
            .await?;
        assert_eq!(answer.status().as_u16(), 200);
        let first = answer.chunk().await?;
        assert!(first.is_some(), "the attach answered nothing");
        drop(answer);
        Ok(())
    }

    /// The upgrade intent file, its folder made.
    fn intent(&self) -> Result<PathBuf, Box<dyn Error>> {
        let folder = self.table.dir.path().join("upgrade");
        std::fs::create_dir_all(&folder)?;
        Ok(folder.join("intent.json"))
    }
}

#[path = "seat_import_lifecycle/events.rs"]
mod events;
