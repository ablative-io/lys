#![cfg(test)]
//! An import reads one seat's monitor records through its HTTP API
//! (AGENTS-003 R2) and maps them into Lys's owners: the budget into context
//! limits and window, prompt settings into words, variables at their exact
//! scope with author and expiry. An answer that is unavailable or says it is
//! incomplete refuses by name and never reads as empty; a budget's usage
//! does not move its revision. A local server answers recorded monitor
//! bodies; no live monitor is asked.

use std::collections::BTreeMap;
use std::error::Error;
use std::sync::Arc;

use axum::Router;
use axum::http::StatusCode;
use axum::routing::{get, post};
use lys_identity_server::seat_import_monitor::{
    SCOPE_AMBIGUOUS, SOURCE_INCOMPLETE, SOURCE_UNAVAILABLE, read_monitor,
};
use lys_identity_server::seat_import_plan::{Completeness, Fragment, MonitorSource};
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

/// 2030-03-17T17:46:40Z.
const CAPTURED_AT: u64 = 1_900_000_000;
const PANE: &str = "w9:p4";

/// What the recorded monitor answers, by path; a status stands for a refusal.
#[derive(Clone)]
struct Recorded {
    answers: BTreeMap<&'static str, (StatusCode, Value)>,
    variables: BTreeMap<String, Value>,
}

fn budget(percent: u64, inform_at: &Value) -> Value {
    json!({
        "id": "sess-w",
        "name": "waffles",
        "percent": percent,
        "target": {"transport": "herdr", "id": PANE},
        "compaction": {"state": "not_due", "since": null, "percent": null, "requested_by": null},
        "pending_informs": [],
        "last_event": null,
        "history_scope": "durable_warden_journal",
        "history_error": null,
        "context_policy": "enforced",
        "context_limit_percent": 85,
        "window_tokens": 400_000,
        "window_source": {"kind": "override", "by": "Tom", "at": "2030-03-01T00:00:00Z"},
        "inform_at": inform_at,
        "inform_source": {"kind": "fleet_default"},
        "compact_at": 85,
        "compact_source": {"kind": "fleet_default"},
    })
}

fn prompts() -> Value {
    json!({
        "records": [
            {"id": "template:night", "kind": "template", "scope": "night", "revision": 2,
             "texts": {"scheduled": "Night shift: {{goals}}", "wake": "Wake: {{message}}"},
             "template_id": null, "checkpoint": "", "author": "Tom", "deleted": false},
            {"id": "agent:waffles", "kind": "agent", "scope": "waffles", "revision": 4,
             "texts": {"inform": "Context at {{context_percent}}. Focus: {{vars.focus | none}}"},
             "template_id": "template:night", "checkpoint": "", "author": "Tom", "deleted": false},
            {"id": "agent:gaia", "kind": "agent", "scope": "gaia", "revision": 1,
             "texts": {"inform": "not this seat's"},
             "template_id": null, "checkpoint": "", "author": "Tom", "deleted": false},
        ],
        "defaults": {},
        "variables": ["agent", "session_id", "goals", "percent"],
        "slots": ["inform", "prepare", "compact", "wake", "scheduled"],
    })
}

fn variables(scope: &str, revision: u64, entries: &Value) -> Value {
    let (kind, key) = scope.split_once(':').unwrap_or_default();
    json!({
        "id": scope, "kind": kind, "scope": key, "revision": revision, "entries": entries,
        "updated_at": "2030-03-17T17:00:00Z", "author": "waffles",
        "values": {}, "expired_keys": [], "missing_keys": [],
    })
}

fn recorded() -> Recorded {
    let ok = StatusCode::OK;
    let answers = BTreeMap::from([
        (
            "/api/budgets",
            (
                ok,
                json!({"budgets": [budget(41, &json!([50, 70]))], "invalid": []}),
            ),
        ),
        ("/api/prompt-settings", (ok, prompts())),
        (
            "/api/scheduled-messages",
            (
                ok,
                json!({"schedules": [], "error": null,
                "health": {"tick_ms": 1000, "last_tick_at": null, "busy_schedule": null,
                    "timer_active": true, "skips": [], "held": []}}),
            ),
        ),
        (
            "/api/rules",
            (ok, json!({"rules": [], "skipped_lines": 0, "skips": []})),
        ),
    ]);
    let agent = json!({
        "focus": {"value": "Review parser", "expires_at": null,
            "updated_at": "2030-03-17T17:00:00Z", "author": "waffles"},
    });
    let session = json!({
        "ticket": {"value": "A-17", "expires_at": "2030-03-18T00:00:00Z",
            "updated_at": "2030-03-17T17:00:00Z", "author": "Tom"},
        "stale": {"value": true, "expires_at": "2030-03-17T17:46:40Z",
            "updated_at": "2030-03-17T17:00:00Z", "author": "Tom"},
    });
    let variables = BTreeMap::from([
        (
            "agent:waffles".to_owned(),
            variables("agent:waffles", 3, &agent),
        ),
        (
            "session:sess-w".to_owned(),
            variables("session:sess-w", 9, &session),
        ),
    ]);
    Recorded { answers, variables }
}

/// Serve `recorded` on a local port; answer its base address.
async fn serve(recorded: Recorded) -> Result<String, Box<dyn Error>> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let base = format!("http://{}", listener.local_addr()?);
    let recorded = Arc::new(recorded);
    let mut routes = Router::new();
    for path in [
        "/api/budgets",
        "/api/prompt-settings",
        "/api/scheduled-messages",
        "/api/rules",
    ] {
        let recorded = Arc::clone(&recorded);
        routes = routes.route(
            path,
            get(move || {
                let (status, body) = recorded.answers[path].clone();
                async move { (status, axum::Json(body)) }
            }),
        );
    }
    let asked = Arc::clone(&recorded);
    routes = routes.route(
        "/api/agent-variables/get",
        post(move |axum::Json(body): axum::Json<Value>| {
            let scope = body["scope"].as_str().unwrap_or_default();
            let answer = match asked.variables.get(scope) {
                Some(answer) => (StatusCode::OK, axum::Json(answer.clone())),
                None => (
                    StatusCode::UNPROCESSABLE_ENTITY,
                    axum::Json(json!({"error": scope})),
                ),
            };
            async move { answer }
        }),
    );
    tokio::spawn(async move { axum::serve(listener, routes).await });
    Ok(base)
}

fn source(base: &str) -> MonitorSource {
    MonitorSource {
        base: base.to_owned(),
        secret_env: None,
        secret_header: None,
        agent: "waffles".to_owned(),
        session: Some("sess-w".to_owned()),
    }
}

async fn read(recorded: Recorded) -> Result<Fragment, Box<dyn Error>> {
    let base = serve(recorded).await?;
    Ok(read_monitor(&source(&base), CAPTURED_AT).await)
}

fn destination<'a>(fragment: &'a Fragment, kind: &str, id: &str) -> Option<&'a Value> {
    fragment
        .destinations
        .iter()
        .find(|entry| entry.record_kind == kind && entry.record_id == id)
        .map(|entry| &entry.change)
}

fn refusal_names(fragment: &Fragment) -> Vec<&str> {
    fragment
        .refusals
        .iter()
        .map(|refused| refused.name.as_str())
        .collect()
}

#[tokio::test]
async fn seat_import_monitor_maps_records() -> TestResult {
    let fragment = read(recorded()).await?;
    assert!(fragment.refusals.is_empty(), "{:?}", fragment.refusals);
    let limits = destination(&fragment, "budget_limits", "agent.waffles").ok_or("no limits")?;
    assert_eq!(limits["holder"], json!({"kind": "agent", "id": "waffles"}));
    assert_eq!(
        limits["limits"],
        json!([
            {"unit": "context_percent", "amount": 50, "period": null, "act": "notice"},
            {"unit": "context_percent", "amount": 70, "period": null, "act": "notice"},
            {"unit": "context_percent", "amount": 85, "period": null, "act": "compact"},
        ])
    );
    assert!(destination(&fragment, "context_window", "agent.waffles").is_none());
    let window = fragment
        .excluded
        .iter()
        .find(|excluded| excluded.source_id == "monitor:budget:sess-w")
        .ok_or("the window override is not kept as not imported")?;
    assert_eq!(
        window.reason,
        "no Lys owner for a context window override (window_tokens)"
    );
    let budget = fragment
        .sources
        .iter()
        .find(|entry| entry.id == "monitor:budget:sess-w")
        .ok_or("no budget source")?;
    assert_eq!(window.revision, budget.source_revision);

    let inform = destination(&fragment, "words_slot", "agent:waffles/context_warning");
    let inform = inform.ok_or("no context warning")?;
    assert_eq!(inform["setting"]["kind"], "text");
    let scheduled = destination(&fragment, "words_slot", "agent:waffles/scheduled_reminder");
    let scheduled = scheduled.ok_or("no linked reminder")?;
    assert_eq!(
        scheduled["setting"],
        json!({"kind": "template", "name": "night-scheduled_reminder"})
    );
    let template = destination(&fragment, "words_template", "night-scheduled_reminder");
    assert_eq!(
        template.ok_or("no template")?["text"],
        "Night shift: {{goals}}"
    );
    assert!(
        !fragment
            .destinations
            .iter()
            .any(|entry| entry.change.to_string().contains("not this seat's")),
        "another agent's words are not imported"
    );

    let focus = destination(&fragment, "variable", "agent:waffles/focus").ok_or("no focus")?;
    assert_eq!(focus["value"], "Review parser");
    assert_eq!(focus["author"], "waffles");
    let ticket = destination(&fragment, "variable", "session:sess-w/ticket").ok_or("no ticket")?;
    assert_eq!(ticket["scope"], json!({"kind": "session", "id": "sess-w"}));
    assert_eq!(ticket["expires_at"], 1_900_022_400);
    assert_eq!(ticket["author"], "Tom");
    assert!(destination(&fragment, "variable", "session:sess-w/stale").is_none());
    assert!(
        fragment
            .excluded
            .iter()
            .any(|excluded| excluded.source_id == "monitor:variables:session:sess-w/stale")
    );
    assert!(
        fragment
            .prerequisites
            .iter()
            .any(|line| line.contains("sess-w"))
    );

    let target = fragment
        .sources
        .iter()
        .find(|entry| entry.kind == "monitor_transport_target")
        .ok_or("the target is not kept as provenance")?;
    assert!(target.locator.contains(PANE));
    assert!(
        fragment
            .destinations
            .iter()
            .all(|entry| !entry.change.to_string().contains(PANE)),
        "the transport target is never a destination"
    );
    let variables = fragment
        .sources
        .iter()
        .find(|entry| entry.id == "monitor:variables:session:sess-w")
        .ok_or("no session variables source")?;
    assert_eq!(
        (
            variables.revision_kind.as_str(),
            variables.source_revision.as_str()
        ),
        ("native", "9")
    );
    assert_eq!(fragment.schedule_counts.map(|counts| counts.total), Some(0));
    Ok(())
}

#[tokio::test]
async fn seat_import_monitor_refuses_incomplete() -> TestResult {
    let cases: [(&str, Value, &str); 5] = [
        (
            "/api/budgets",
            json!({"budgets": [], "invalid": [{"key": "sess-x", "reason": "two transports"}]}),
            SOURCE_INCOMPLETE,
        ),
        (
            "/api/budgets",
            json!({"budgets": [{"id": "sess-w", "name": "waffles", "budget_error": "inform level 90 is above compact_at 85"}], "invalid": []}),
            SOURCE_INCOMPLETE,
        ),
        (
            "/api/rules",
            json!({"rules": [], "skipped_lines": 2, "skips": []}),
            SOURCE_INCOMPLETE,
        ),
        (
            "/api/scheduled-messages",
            json!({"schedules": [], "error": null, "health": {"skips": [], "held": ["night"]}}),
            SOURCE_INCOMPLETE,
        ),
        (
            "/api/prompt-settings",
            json!({"defaults": {}}),
            SOURCE_INCOMPLETE,
        ),
    ];
    for (path, body, name) in cases {
        let mut answers = recorded();
        answers.answers.insert(path, (StatusCode::OK, body));
        let fragment = read(answers).await?;
        assert_eq!(
            refusal_names(&fragment),
            [name],
            "{path}: {:?}",
            fragment.refusals
        );
        assert!(
            fragment.refusals[0].member.contains(path),
            "{:?}",
            fragment.refusals
        );
        if !fragment.refusals[0].member.contains('#') {
            let entry = fragment
                .sources
                .iter()
                .find(|entry| entry.locator.ends_with(path))
                .ok_or("no source entry for the API")?;
            assert!(
                matches!(entry.completeness, Completeness::Incomplete { .. }),
                "{entry:?}"
            );
        }
    }

    let mut down = recorded();
    let error = json!({"error": "unavailable"});
    down.answers
        .insert("/api/budgets", (StatusCode::SERVICE_UNAVAILABLE, error));
    let fragment = read(down).await?;
    assert_eq!(refusal_names(&fragment), [SOURCE_UNAVAILABLE]);
    assert!(
        fragment
            .destinations
            .iter()
            .all(|entry| entry.record_kind != "budget_limits")
    );

    let mut empty = recorded();
    empty.answers.insert(
        "/api/budgets",
        (StatusCode::OK, json!({"budgets": [], "invalid": []})),
    );
    let fragment = read(empty).await?;
    assert!(
        fragment.refusals.is_empty(),
        "zero rows with no error is complete: {:?}",
        fragment.refusals
    );
    let budgets = fragment
        .sources
        .iter()
        .find(|entry| entry.id == "monitor:/api/budgets")
        .ok_or("no budgets source")?;
    assert_eq!(budgets.completeness, Completeness::Complete);

    let mut twice = recorded();
    let rows =
        json!({"budgets": [budget(1, &json!([50])), budget(2, &json!([60]))], "invalid": []});
    twice.answers.insert("/api/budgets", (StatusCode::OK, rows));
    let fragment = read(twice).await?;
    assert_eq!(refusal_names(&fragment), [SCOPE_AMBIGUOUS]);

    let closed = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let gone = format!("http://{}", closed.local_addr()?);
    drop(closed);
    let fragment = read_monitor(&source(&gone), CAPTURED_AT).await;
    assert!(
        refusal_names(&fragment)
            .iter()
            .all(|name| *name == SOURCE_UNAVAILABLE)
    );
    assert_eq!(
        fragment.refusals.len(),
        6,
        "every API is named: {:?}",
        fragment.refusals
    );
    assert!(fragment.destinations.is_empty());
    assert_eq!(
        fragment.schedule_counts, None,
        "an unread listing is not zero schedules"
    );
    Ok(())
}

#[tokio::test]
async fn an_unset_secret_variable_refuses_without_asking() -> TestResult {
    let base = serve(recorded()).await?;
    let mut source = source(&base);
    source.secret_env = Some("LYS_SEAT_IMPORT_TEST_SECRET_NEVER_SET".to_owned());
    source.secret_header = Some("x-collector-secret".to_owned());
    let fragment = read_monitor(&source, CAPTURED_AT).await;
    assert_eq!(refusal_names(&fragment), [SOURCE_UNAVAILABLE]);
    assert!(
        fragment.refusals[0]
            .detail
            .contains("LYS_SEAT_IMPORT_TEST_SECRET_NEVER_SET")
    );
    assert!(
        fragment.refusals[0].detail.contains("is not set"),
        "{:?}",
        fragment.refusals
    );
    assert!(fragment.destinations.is_empty());
    Ok(())
}

#[tokio::test]
async fn a_secret_named_with_no_header_refuses_without_asking() -> TestResult {
    let base = serve(recorded()).await?;
    let mut source = source(&base);
    source.secret_env = Some("LYS_SEAT_IMPORT_TEST_SECRET_NEVER_SET".to_owned());
    let fragment = read_monitor(&source, CAPTURED_AT).await;
    assert_eq!(refusal_names(&fragment), [SOURCE_UNAVAILABLE]);
    assert_eq!(fragment.refusals[0].member, "monitor.secret_header");
    assert!(
        fragment.refusals[0]
            .detail
            .contains("no header name is assumed")
    );
    assert!(
        fragment.sources.is_empty(),
        "nothing was asked: {:?}",
        fragment.sources
    );
    assert!(fragment.destinations.is_empty());

    source.secret_header = Some("not a header".to_owned());
    let fragment = read_monitor(&source, CAPTURED_AT).await;
    assert_eq!(refusal_names(&fragment), [SOURCE_UNAVAILABLE]);
    assert_eq!(fragment.refusals[0].member, "monitor.secret_header");
    assert!(
        fragment.sources.is_empty(),
        "nothing was asked: {:?}",
        fragment.sources
    );
    Ok(())
}

#[tokio::test]
async fn seat_import_monitor_revision_projection() -> TestResult {
    let revision = |fragment: &Fragment| {
        fragment
            .sources
            .iter()
            .find(|entry| entry.id == "monitor:budget:sess-w")
            .map(|entry| (entry.revision_kind.clone(), entry.source_revision.clone()))
    };
    let mut first = recorded();
    first.answers.insert(
        "/api/budgets",
        (
            StatusCode::OK,
            json!({"budgets": [budget(10, &json!([50, 70]))], "invalid": []}),
        ),
    );
    let mut used = recorded();
    used.answers.insert(
        "/api/budgets",
        (
            StatusCode::OK,
            json!({"budgets": [budget(64, &json!([50, 70]))], "invalid": []}),
        ),
    );
    let mut changed = recorded();
    changed.answers.insert(
        "/api/budgets",
        (
            StatusCode::OK,
            json!({"budgets": [budget(10, &json!([40, 70]))], "invalid": []}),
        ),
    );
    let first = revision(&read(first).await?).ok_or("no budget source")?;
    let used = revision(&read(used).await?).ok_or("no budget source")?;
    let changed = revision(&read(changed).await?).ok_or("no budget source")?;
    assert_eq!(first.0, "content_sha256");
    assert_eq!(
        first, used,
        "a usage observation does not move the definition's revision"
    );
    assert_ne!(first, changed, "changed inform levels move it");

    let mut renumbered = recorded();
    let mut body = prompts();
    body["records"][1]["revision"] = json!(5);
    renumbered
        .answers
        .insert("/api/prompt-settings", (StatusCode::OK, body));
    let prompt = |fragment: &Fragment| {
        fragment
            .sources
            .iter()
            .find(|entry| entry.id == "monitor:prompt:agent:waffles")
            .map(|entry| entry.source_revision.clone())
    };
    let before = read(recorded()).await?;
    let after = read(renumbered).await?;
    assert_eq!(prompt(&before).as_deref(), Some("4"));
    assert_eq!(prompt(&after).as_deref(), Some("5"));
    let words = |fragment: &Fragment| {
        destination(fragment, "words_slot", "agent:waffles/context_warning").cloned()
    };
    assert_eq!(
        words(&before),
        words(&after),
        "the wording is identical while the native revision moved"
    );
    Ok(())
}
