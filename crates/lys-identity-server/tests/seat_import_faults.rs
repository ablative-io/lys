#![cfg(test)]
//! AGENTS-003 R9: every import fault is driven by an explicit fixture signal
//! or event and answered by name, never waited out with a sleep, a deadline
//! or a watchdog. The behaviour red matrix, `seat_import_red_matrix.json`,
//! is the record naming a runnable test for every acceptance behaviour of the
//! brief; its red and green are written from the qualifying battery.
//!
//! Every call goes through the public routes: `POST /seats/{name}/import/dry-run`,
//! `POST /seats/{name}/import/confirm` and `GET /seats/{name}/import`. The
//! Argus source is a fixture server this file owns; it answers, denies, or
//! holds one request until the test releases it, telling the test the moment
//! the importer has reached it. Holding the importer there is how a process
//! exit lands mid-import and how a reply is lost: the service is restarted
//! (the harness stops its runtime and opens every store again from disk)
//! while the importer waits, and the restart returns only when the new
//! service is ready.

#[path = "support/runner_start.rs"]
mod support;

use std::error::Error;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use axum::Router;
use axum::extract::{Request, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::{Value, json};
use support::{Table, operation};
use tokio::sync::{oneshot, watch};

type TestResult = Result<(), Box<dyn Error>>;

const SEAT: &str = "waffles";
/// A secret held by the credential fixture; no answer may carry it (R3).
const CANARY: &str = "canary-private-key-0f3c9a1e5b7d";
/// The refusal a confirmation answers when a source changed after the
/// preview (R4), as `SeatImportError::SourceChanged` names it.
const SOURCE_MOVED: &str = "import_source_changed";

/// What the Argus fixture does with the next request.
enum Mode {
    Answer,
    Deny,
    Hold(Option<Held>),
}

/// One held request: the test hears `reached` with the path, then releases.
struct Held {
    reached: oneshot::Sender<String>,
    release: oneshot::Receiver<()>,
}

struct Argus {
    mode: Mutex<Mode>,
}

impl Argus {
    fn set(&self, mode: Mode) -> TestResult {
        *self
            .mode
            .lock()
            .map_err(|error| format!("argus fixture lock poisoned: {error}"))? = mode;
        Ok(())
    }
}

/// The documented empty answer of each Argus read the importer makes (R2).
fn argus_body(path: &str) -> Option<Value> {
    Some(match path {
        "/api/budgets" => json!({ "budgets": [], "invalid": [] }),
        "/api/prompt-settings" => json!({ "records": [], "builtins": {}, "variables": [] }),
        "/api/agent-variables/get" => json!({
            "id": "agent:waffles", "revision": 0, "values": {}, "entries": {},
            "missing_keys": [], "expired_keys": [],
        }),
        "/api/scheduled-messages" => json!({ "schedules": [], "health": {
            "tick_ms": 1000, "last_tick_at": null, "timer_active": true, "busy_schedule": null,
        }}),
        "/api/rules" => json!({ "rules": [] }),
        _ => return None,
    })
}

async fn argus_answer(State(fixture): State<Arc<Argus>>, request: Request) -> Response {
    let path = request.uri().path().to_owned();
    let (deny, held) = match fixture.mode.lock() {
        Ok(mut mode) => match &mut *mode {
            Mode::Answer => (false, None),
            Mode::Deny => (true, None),
            Mode::Hold(held) => (false, held.take()),
        },
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };
    if deny {
        let body = json!({ "error": "collector authentication required" });
        return (StatusCode::FORBIDDEN, axum::Json(body)).into_response();
    }
    if let Some(held) = held
        && held.reached.send(path.clone()).is_ok()
    {
        // Released, or the test dropped the sender: either is the signal.
        held.release.await.ok();
    }
    match argus_body(&path) {
        Some(body) => axum::Json(body).into_response(),
        None => (StatusCode::NOT_FOUND, axum::Json(json!({ "error": path }))).into_response(),
    }
}

/// A service with one seat, its Claude resource folder and an Argus fixture.
struct Scene {
    table: Table,
    folder: PathBuf,
    argus: Arc<Argus>,
    argus_base: String,
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
    table
        .ok(
            "/seats",
            &json!({
                "operation": operation()?, "name": SEAT, "agent": table.agent(),
                "profile_version": 1, "machine": machine,
            }),
        )
        .await?;
    let folder = table.dir.path().join("seat-resources").join(SEAT);
    std::fs::create_dir_all(&folder)?;
    let settings = r#"{"model":"claude-fable-5-1","permissions":{"defaultMode":"plan"}}"#;
    std::fs::write(folder.join("settings.json"), settings)?;
    std::fs::write(folder.join("system-prompt.md"), "You are Waffles.\n")?;
    std::fs::write(folder.join("mcp.json"), r#"{"mcpServers":{}}"#)?;
    let argus = Arc::new(Argus {
        mode: Mutex::new(Mode::Answer),
    });
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let argus_base = format!("http://{}", listener.local_addr()?);
    let router = Router::new()
        .fallback(argus_answer)
        .with_state(Arc::clone(&argus));
    tokio::spawn(async move { axum::serve(listener, router).await });
    Ok(Scene {
        table,
        folder,
        argus,
        argus_base,
    })
}

impl Scene {
    /// The seat's manifest: its resource folder, and Argus when asked.
    fn manifest(&self, with_argus: bool, seat_identity_file: Option<&Path>) -> Value {
        let argus = with_argus.then(|| {
            json!({ "base": self.argus_base, "secret_env": null, "agent": SEAT, "session": null })
        });
        json!({
            "seat": SEAT, "harness": "claude", "codex_profile": null,
            "claude_folder": self.folder, "seat_identity_file": seat_identity_file, "monitor": argus,
            "secret_handles": [], "recipient_map": {},
        })
    }

    async fn dry_run(&self, manifest: &Value) -> Result<Value, Box<dyn Error>> {
        let path = format!("/seats/{SEAT}/import/dry-run");
        let body = json!({ "manifest": manifest });
        self.table.ok(&path, &body).await
    }

    async fn confirm(
        &self,
        plan: &Value,
        operation: &str,
    ) -> Result<(u16, Value), Box<dyn Error>> {
        let path = format!("/seats/{SEAT}/import/confirm");
        let body = confirm_body(plan, operation);
        self.table.service.post(&path, Some(&self.table.ada), &body).await
    }

    async fn status(&self) -> Result<Value, Box<dyn Error>> {
        let path = format!("/seats/{SEAT}/import");
        let answer = self.table.service.get(&path, Some(&self.table.ada)).await?;
        assert_eq!(answer.0, 200, "{path}: {}", answer.1);
        Ok(answer.1)
    }

    /// Every destination the import may write, as its owner answers it.
    async fn destinations(&self) -> Result<Vec<Value>, Box<dyn Error>> {
        let agent = self.table.agent();
        let mut read = Vec::new();
        for path in [
            format!("/agents/{agent}/provisioning"),
            format!("/agents/{agent}/variables"),
            "/words".to_owned(),
            "/schedules".to_owned(),
            format!("/seats/{SEAT}"),
        ] {
            read.push(self.table.service.get(&path, Some(&self.table.ada)).await?.1);
        }
        Ok(read)
    }
}

fn confirm_body(plan: &Value, operation: &str) -> Value {
    json!({
        "plan_id": plan["plan_id"], "plan_revision": plan["plan_revision"],
        "operation": operation,
    })
}

fn plan_refusals(plan: &Value) -> Result<&Vec<Value>, Box<dyn Error>> {
    Ok(plan["refusals"]
        .as_array()
        .ok_or_else(|| format!("the plan carries no refusals list: {plan}"))?)
}

/// A refused answer, by `name`.
fn refused(answer: &(u16, Value), name: &str) {
    assert_ne!(answer.0, 200, "a fault was accepted: {}", answer.1);
    assert_eq!(answer.1["refusal"], name, "{}", answer.1);
}

/// A refused confirmation of a plan the dry run already refused, changing
/// no destination.
async fn refused_and_unwritten(scene: &Scene, plan: &Value) -> TestResult {
    let before = scene.destinations().await?;
    let answer = scene.confirm(plan, &operation()?).await?;
    refused(&answer, "import_plan_refused");
    assert_eq!(scene.destinations().await?, before, "a refused import wrote");
    Ok(())
}

type InFlight = tokio::task::JoinHandle<Result<(u16, String), reqwest::Error>>;

/// Send `plan`'s confirmation with Argus set to hold the importer's first
/// read; answer once the importer has reached it, with the request in
/// flight, the release, and the Argus path it is held at.
async fn held_confirm(
    scene: &Scene,
    plan: &Value,
    confirming: &str,
) -> Result<(InFlight, oneshot::Sender<()>, String), Box<dyn Error>> {
    let (reached, at) = oneshot::channel();
    let (release, released) = oneshot::channel();
    scene.argus.set(Mode::Hold(Some(Held {
        reached,
        release: released,
    })))?;
    let url = format!("{}/seats/{SEAT}/import/confirm", scene.table.service.base);
    let body = confirm_body(plan, confirming).to_string();
    let cookie = scene.table.ada.clone();
    let in_flight = tokio::spawn(async move {
        let answer = reqwest::Client::new()
            .post(url)
            .header(reqwest::header::COOKIE, cookie)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(body)
            .send()
            .await?;
        let status = answer.status().as_u16();
        Ok((status, answer.text().await?))
    });
    let held_at = at.await?;
    Ok((in_flight, release, held_at))
}

/// Reads `/seats` until `stop` changes, answering how many reads answered
/// 200 and the first that did not.
async fn competing_load(
    base: String,
    cookie: String,
    mut stop: watch::Receiver<bool>,
) -> Result<u64, String> {
    let client = reqwest::Client::new();
    let cookie = reqwest::header::HeaderValue::try_from(cookie)
        .map_err(|error| format!("the session cookie is not a header: {error}"))?;
    let mut seats = base;
    let one = format!("{seats}/seats/{SEAT}");
    seats.push_str("/seats");
    let mut answered = 0_u64;
    loop {
        for url in [&seats, &one] {
            let response = client
                .get(url)
                .header(reqwest::header::COOKIE, cookie.clone())
                .send()
                .await
                .map_err(|error| format!("{url}: {error}"))?;
            if response.status().as_u16() != 200 {
                return Err(format!("{url} answered {}", response.status()));
            }
            answered += 1;
        }
        if *stop.borrow_and_update() {
            return Ok(answered);
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn seat_import_signal_faults() -> TestResult {
    let mut scene = scene().await?;
    let (stop, stopped) = watch::channel(false);
    let load = tokio::spawn(competing_load(
        scene.table.service.base.clone(),
        scene.table.ada.clone(),
        stopped,
    ));
    let manifest = scene.manifest(false, None);
    let mut answers = Vec::new();

    // Source mutation: the file changes between preview and confirmation.
    let prompt = scene.folder.join("system-prompt.md");
    let plan = scene.dry_run(&manifest).await?;
    assert_eq!(plan_refusals(&plan)?.len(), 0, "{plan}");
    let before = scene.destinations().await?;
    std::fs::write(&prompt, "You are Waffles, changed after the preview.\n")?;
    let answer = scene.confirm(&plan, &operation()?).await?;
    refused(&answer, SOURCE_MOVED);
    assert!(answer.1.to_string().contains("system-prompt.md"), "{}", answer.1);
    assert_eq!(scene.destinations().await?, before, "a moved source wrote");
    answers.push(answer.1);
    std::fs::write(&prompt, "You are Waffles.\n")?;

    // Denied read: the settings file cannot be read.
    let settings = scene.folder.join("settings.json");
    std::fs::set_permissions(&settings, std::fs::Permissions::from_mode(0o000))?;
    let plan = scene.dry_run(&manifest).await?;
    std::fs::set_permissions(&settings, std::fs::Permissions::from_mode(0o600))?;
    let refusals = plan_refusals(&plan)?;
    assert!(
        refusals.iter().any(|refusal| refusal["name"] == "import_source_unreadable"
            && refusal.to_string().contains("settings.json")),
        "{plan}"
    );
    refused_and_unwritten(&scene, &plan).await?;
    answers.push(plan);

    // Credential-reference failure: absent, then unreadable, each refused
    // before confirmation and neither read for its key.
    let absent = scene.table.dir.path().join("absent.seat.json");
    let unreadable = scene.table.dir.path().join("waffles.seat.json");
    std::fs::write(
        &unreadable,
        json!({ "seat": SEAT, "privateKeyPem": CANARY }).to_string(),
    )?;
    std::fs::set_permissions(&unreadable, std::fs::Permissions::from_mode(0o000))?;
    for reference in [&absent, &unreadable] {
        let plan = scene.dry_run(&scene.manifest(false, Some(reference))).await?;
        let refusals = plan_refusals(&plan)?;
        let locator = reference.to_str().ok_or("fixture path is not text")?;
        assert!(
            refusals.iter().any(|refusal| refusal.to_string().contains(locator)),
            "{plan}"
        );
        refused_and_unwritten(&scene, &plan).await?;
        answers.push(plan);
    }

    // An Argus source that refuses to be read is unavailable, never empty.
    scene.argus.set(Mode::Deny)?;
    let plan = scene.dry_run(&scene.manifest(true, None)).await?;
    scene.argus.set(Mode::Answer)?;
    assert!(
        plan_refusals(&plan)?.iter().any(|refusal| refusal["name"]
            == "import_source_unavailable"
            && refusal["member"].as_str().is_some_and(|member| member.contains("/api/"))),
        "{plan}"
    );
    refused_and_unwritten(&scene, &plan).await?;
    answers.push(plan);

    // The competing reads all answered while every fault was refused.
    stop.send(true)?;
    let read = load.await??;
    assert!(read >= 2, "the competing load made {read} reads");
    for answer in &answers {
        assert!(!answer.to_string().contains(CANARY), "{answer}");
    }

    // Process exit mid-import, losing the confirmation's reply: the
    // importer is held at Argus, the service restarts under it, and the
    // same confirmation sent again resolves the one operation.
    let with_argus = scene.manifest(true, None);
    let mut plan = scene.dry_run(&with_argus).await?;
    assert_eq!(plan_refusals(&plan)?.len(), 0, "{plan}");
    let confirming = operation()?;
    let (in_flight, release, held_at) = held_confirm(&scene, &plan, &confirming).await?;
    assert!(held_at.starts_with("/api/"), "{held_at}");
    scene.table.service.restart().await?;
    drop(release);
    match in_flight.await? {
        Err(lost) => eprintln!("the confirmation's reply was lost: {lost}"),
        Ok((status, text)) => assert_ne!(status, 200, "completed across the exit: {text}"),
    }
    let status_path = format!("/seats/{SEAT}/import");
    let (_, after_exit) = scene.table.service.get(&status_path, Some(&scene.table.ada)).await?;

    // Recovery is the same confirmation. A reserved import resumes as
    // itself; an exit before reservation leaves nothing reserved, and the
    // dry run (kept in memory only) is shown again and confirmed under the
    // same operation.
    let mut answer = scene.confirm(&plan, &confirming).await?;
    if answer.1["refusal"] == "import_plan_unknown" {
        assert!(after_exit["state"].is_null(), "a reserved import was forgotten: {after_exit}");
        plan = scene.dry_run(&with_argus).await?;
        answer = scene.confirm(&plan, &confirming).await?;
    }
    assert_eq!(answer.0, 200, "{}", answer.1);
    let completed = scene.status().await?;
    assert_eq!(completed["state"], "completed", "{completed}");
    if let Some(reserved) = after_exit["operation"].as_str() {
        assert_eq!(completed["operation"], reserved, "{after_exit} {completed}");
    }
    let settled = scene.destinations().await?;
    let again = scene.confirm(&plan, &confirming).await?;
    assert_eq!(again.0, 200, "{}", again.1);
    assert_eq!(again.1["receipt"], answer.1["receipt"], "a rerun made a second receipt");
    assert_eq!(scene.destinations().await?, settled, "a rerun wrote");
    let (_, live) = scene.table.service.get("/runtime/live", Some(&scene.table.ada)).await?;
    assert_eq!(live["sessions"].as_array().map(Vec::len), Some(0), "import started {live}");
    scene.table.close()
}

/// The four hot paths do no import work: with a confirmation held inside
/// the importer, registry reads and a delivery each answer at once with
/// their own answer, the same as before the import began. Proxy forwarding
/// and hook ingest live in lys-runner, whose dependencies do not include
/// this crate, so no import code is reachable from them at all.
#[tokio::test(flavor = "multi_thread")]
async fn seat_import_hot_paths_do_no_import_work() -> TestResult {
    let scene = scene().await?;
    let reads = [
        "/seats".to_owned(),
        format!("/seats/{SEAT}"),
        "/seats/owned".to_owned(),
        "/runtime/live".to_owned(),
    ];
    let (send, ada) = (format!("/seats/{SEAT}/send"), Some(scene.table.ada.as_str()));
    let mut before = Vec::new();
    for path in &reads {
        before.push(scene.table.service.get(path, ada).await?);
    }
    let plan = scene.dry_run(&scene.manifest(true, None)).await?;
    assert_eq!(plan_refusals(&plan)?.len(), 0, "{plan}");

    let (in_flight, release, held_at) = held_confirm(&scene, &plan, &operation()?).await?;
    assert!(held_at.starts_with("/api/"), "{held_at}");

    // The import is in flight and holding whatever it holds.
    for (path, earlier) in reads.iter().zip(&before) {
        let answer = scene.table.service.get(path, ada).await?;
        assert_eq!(&answer, earlier, "{path} changed while an import was held");
    }
    let body = json!({ "operation": operation()?, "text": "hello" });
    let delivered = scene.table.service.post(&send, ada, &body).await?;
    assert_eq!(delivered.1["refusal"], "seat_not_running", "{}", delivered.1);

    release.send(()).map_err(|()| "the held request was dropped")?;
    let (status, text) = in_flight.await??;
    assert_eq!(status, 200, "{text}");
    scene.table.close()
}
