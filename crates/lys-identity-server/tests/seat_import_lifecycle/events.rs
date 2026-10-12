//! Import state and session declarations survive lifecycle events.

use serde_json::json;

use super::{
    SEAT, Scene, TestResult, Value, operation, qualified_is_empty, scene, start_refused, support,
    surviving,
};

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
    assert_eq!(
        surviving(scene.status().await?),
        surviving(completed.clone())
    );
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
    assert_eq!(
        surviving(scene.status().await?),
        surviving(completed.clone())
    );
    scene.table.close()
}

/// DIRECTORY-064's pin, held: lys-runner's `QUALIFIED` list is empty and the
/// imported seat's start is refused by name, nothing starting. When the
/// pin lands and the start answers, this is the survival test: the seat's
/// session survives an attachment exit, a service exit and an upgrade
/// handoff ([`survives`]).
#[tokio::test(flavor = "multi_thread")]
async fn seat_import_survives_lifecycle_once_qualified() -> TestResult {
    // A seat's start runs its owner as the workspace's `lys`; without it the
    // owner is the test binary and the start is refused for that alone.
    support::owner_program()?.ok_or(
        "owner_program_missing: build the workspace's lys binary beside the tests (cargo nextest run --workspace)",
    )?;
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
    assert_eq!(
        surviving(scene.status().await?),
        surviving(completed.clone())
    );
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
    assert_eq!(
        surviving(scene.status().await?),
        surviving(completed.clone())
    );
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

#[tokio::test(flavor = "multi_thread")]
async fn seat_import_preserves_session_controls_and_message_prefix() -> TestResult {
    let scene = scene().await?;
    let path = format!("/agents/{}/provisioning", scene.table.agent());
    let before = scene.get(&path).await?;
    let session = &before["versions"]
        .as_array()
        .ok_or("versions missing")?
        .last()
        .ok_or("version missing")?["settings"]["session"];
    assert_eq!(session["requires_controls"], true);
    assert_eq!(session["message_prefix"], "[Seat] ");
    let plan = scene.dry_run().await?;
    assert_eq!(
        plan["destinations"][0]["change"]["settings"]["session"],
        *session
    );
    let answer = scene.confirm(&plan, &operation()?).await?;
    assert_eq!(answer.0, 200, "{}", answer.1);
    let after = scene.get(&path).await?;
    let versions = after["versions"].as_array().ok_or("versions missing")?;
    assert_eq!(versions.len(), 3);
    assert_eq!(
        versions.last().ok_or("imported version missing")?["settings"]["session"],
        *session
    );
    scene.table.close()
}

#[tokio::test(flavor = "multi_thread")]
async fn seat_import_manifest_without_sources_refuses_confirm() -> TestResult {
    let scene = scene().await?;
    let mut manifest = scene.manifest();
    manifest["claude_folder"] = Value::Null;
    let plan = scene
        .table
        .ok(
            "/seats/waffles/import/dry-run",
            &serde_json::json!({"manifest":manifest}),
        )
        .await?;
    assert!(plan["sources"].as_array().is_some_and(Vec::is_empty));
    assert!(plan["destinations"].as_array().is_some_and(Vec::is_empty));
    assert!(plan["refusals"].as_array().is_some_and(|refusals| {
        refusals
            .iter()
            .any(|refusal| refusal["name"] == "import_source_missing")
    }));
    let answer = scene.confirm(&plan, &operation()?).await?;
    assert_eq!(answer.0, 409, "{}", answer.1);
    assert_eq!(answer.1["refusal"], "import_plan_refused");
    scene.table.close()
}
