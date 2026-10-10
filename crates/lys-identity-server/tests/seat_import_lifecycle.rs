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
const UNQUALIFIED: &str = "seat_owner_start_failed";
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
        "session": {"requires_controls": true},
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

/// The start refused by its owner start's name.
fn start_refused(answer: &(u16, Value)) {
    assert_ne!(answer.0, 200, "a managed start answered: {}", answer.1);
    assert_eq!(answer.1["refusal"], UNQUALIFIED, "{}", answer.1);
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
            let mut answer = self.get(&path).await?;
            // The latest dry run is kept in memory only (R4: a restart
            // forgets it and the person runs a new one), so it is no part
            // of the import that must survive.
            if let Some(status) = answer.as_object_mut() {
                status.remove("previewed");
            }
            held.push(answer);
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

/// Without a launched seat: import status and the completed manifest
/// survive a service restart and an upgrade handoff, and confirming again
/// after each writes nothing and starts nothing.
#[tokio::test(flavor = "multi_thread")]
async fn seat_import_status_survives_restart_and_upgrade() -> TestResult {
    let mut scene = scene().await?;
    let (plan, confirming, completed) = scene.imported().await?;
    let held = scene.held().await?;

    scene.table.service.restart().await?;
    assert_eq!(scene.held().await?, held, "a restart changed the import");
    scene.confirmed_again(&plan, &confirming).await?;

    let intent = scene.intent()?;
    scene
        .table
        .service
        .restart_adjusted(|config| config.operator_upgrade_file = Some(intent))
        .await?;
    assert_eq!(
        scene.held().await?,
        held,
        "an upgrade handoff changed the import"
    );
    assert_eq!(scene.status().await?, completed);
    scene.confirmed_again(&plan, &confirming).await?;
    assert_eq!(
        scene.seat_sessions().await?,
        Vec::<Value>::new(),
        "import started a seat"
    );
    scene.table.close()
}

/// Today an imported seat is not started: its start is refused
/// `seat_owner_start_failed` by name, nothing starts, and the import stays
/// the one completed.
#[tokio::test(flavor = "multi_thread")]
async fn seat_import_survives_lifecycle() -> TestResult {
    let scene = scene().await?;
    let (_, _, completed) = scene.imported().await?;
    let answer = scene.start().await?;
    start_refused(&answer);
    assert_eq!(
        scene.seat_sessions().await?,
        Vec::<Value>::new(),
        "a refused start started"
    );
    assert_eq!(scene.status().await?, completed);
    scene.table.close()
}

/// DIRECTORY-064's pin, held: lys-runner's `QUALIFIED` list is empty and the
/// imported seat's start is refused by name, nothing starting. When the
/// pin lands and the start answers, this is the survival test: the seat's
/// session survives an attachment exit, a service exit and an upgrade
/// handoff ([`survives`]).
#[tokio::test(flavor = "multi_thread")]
async fn seat_import_survives_lifecycle_once_qualified() -> TestResult {
    let scene = scene().await?;
    let (plan, confirming, completed) = scene.imported().await?;
    let answer = scene.start().await?;
    if answer.0 == 200 {
        return survives(scene, &plan, &confirming, &completed).await;
    }
    assert!(
        qualified_is_empty()?,
        "lys-runner names a qualified binary, yet the start was refused: {}",
        answer.1
    );
    start_refused(&answer);
    assert_eq!(scene.status().await?, completed);
    scene.table.close()
}

/// With the moved seat running: its session survives an attachment exit, a
/// service exit and an upgrade handoff, and the registry reads the same
/// seat and the one completed import receipt after each.
async fn survives(
    mut scene: Scene,
    plan: &Value,
    confirming: &str,
    completed: &Value,
) -> TestResult {
    let sessions = scene.seat_sessions().await?;
    assert_eq!(sessions.len(), 1, "{sessions:?}");
    let seat = scene.get(&format!("/seats/{SEAT}")).await?;

    // A correlated operation in flight across every event: the same send
    // answered the same way, never delivered twice.
    let sending = operation()?;
    let send = json!({ "operation": sending, "text": "still here" });
    let send_path = format!("/seats/{SEAT}/send");
    let delivery = scene
        .table
        .service
        .post(&send_path, Some(&scene.table.ada), &send)
        .await?;
    assert_eq!(delivery.0, 200, "{}", delivery.1);

    scene.attach_and_exit().await?;
    assert_eq!(
        scene.seat_sessions().await?,
        sessions,
        "attach exit moved the seat"
    );

    scene.table.service.restart().await?;
    assert_eq!(
        scene.seat_sessions().await?,
        sessions,
        "service exit moved the seat"
    );

    let intent = scene.intent()?;
    scene
        .table
        .service
        .restart_adjusted(|config| config.operator_upgrade_file = Some(intent))
        .await?;
    assert_eq!(
        scene.seat_sessions().await?,
        sessions,
        "upgrade moved the seat"
    );

    let again = scene
        .table
        .service
        .post(&send_path, Some(&scene.table.ada), &send)
        .await?;
    assert_eq!(
        again, delivery,
        "the in-flight send was not resolved as itself"
    );
    assert_eq!(scene.get(&format!("/seats/{SEAT}")).await?, seat);
    assert_eq!(&scene.status().await?, completed);
    // A confirmation while the seat runs is refused by name (the seat runs,
    // or the restarts forgot the dry run) and touches it not.
    let answer = scene.confirm(plan, confirming).await?;
    let named = answer.1["refusal"].as_str().unwrap_or_default();
    assert!(
        ["import_seat_running", "import_plan_unknown"].contains(&named),
        "{}",
        answer.1
    );
    assert_eq!(
        scene.seat_sessions().await?,
        sessions,
        "the importer restarted the seat"
    );
    scene.table.close()
}

/// The upgrade owner refuses the confirmation by name and the intent is
/// kept; once the upgrade clears, the reply of the retried confirmation is
/// lost, and the same operation read back and sent again resolves into one
/// completed import without starting anything.
#[tokio::test(flavor = "multi_thread")]
async fn seat_import_upgrade_failure_named() -> TestResult {
    let mut scene = scene().await?;
    let intent = scene.intent()?;
    std::fs::write(&intent, b"pending")?;
    let watched = intent.clone();
    scene
        .table
        .service
        .restart_adjusted(|config| config.operator_upgrade_file = Some(watched))
        .await?;
    let sessions = scene.seat_sessions().await?;
    // Previews live in memory only, so the plan is shown after the restart.
    let plan = scene.dry_run().await?;
    let revision = plan["plan_revision"]
        .as_str()
        .ok_or_else(|| format!("the plan has no revision: {plan}"))?
        .to_owned();

    let confirming = operation()?;
    let answer = scene.confirm(&plan, &confirming).await?;
    assert_ne!(
        answer.0, 200,
        "confirmed across a pending upgrade: {}",
        answer.1
    );
    assert_eq!(
        answer.1["refusal"], "import_upgrade_pending",
        "{}",
        answer.1
    );
    // The upgrade owner's intent is retained as written, and the import is
    // not complete: nothing of it is selected.
    assert_eq!(
        std::fs::read(&intent)?,
        b"pending",
        "the import touched the upgrade intent"
    );
    let path = format!("/seats/{SEAT}/import");
    let (code, kept) = scene
        .table
        .service
        .get(&path, Some(&scene.table.ada))
        .await?;
    assert_ne!(kept["state"], "completed", "{kept}");
    if code != 200 {
        assert!(
            kept["refusal"].as_str().is_some(),
            "an unnamed status refusal: {kept}"
        );
    }
    assert_eq!(
        scene.seat_sessions().await?,
        sessions,
        "cleanup touched a seat"
    );

    // The upgrade clears; the confirmation is sent and its reply discarded.
    std::fs::remove_file(&intent)?;
    let url = format!("{}/seats/{SEAT}/import/confirm", scene.table.service.base);
    let body = json!({
        "plan_id": plan["plan_id"], "plan_revision": revision, "operation": confirming,
    });
    let lost = reqwest::Client::new()
        .post(url)
        .header(reqwest::header::COOKIE, &scene.table.ada)
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(body.to_string())
        .send()
        .await?;
    drop(lost);

    let recovered = scene.status().await?;
    assert_eq!(recovered["state"], "completed", "{recovered}");
    let held = scene.held().await?;
    let answer = scene.confirm(&plan, &confirming).await?;
    assert_eq!(answer.0, 200, "{}", answer.1);
    assert_eq!(scene.held().await?, held, "recovery imported twice");
    assert_eq!(
        scene.seat_sessions().await?,
        sessions,
        "recovery started a seat"
    );
    scene.table.close()
}
