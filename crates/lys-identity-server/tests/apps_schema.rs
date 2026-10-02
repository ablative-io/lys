#![cfg(test)]
//! An app's schema after registration: a change names the version it
//! replaces, its dry run and its refused write name the same stranded
//! relations and counts, it is taken once those grants are revoked through
//! the ordinary route, every version stays readable, and a change an app
//! makes waits for the administrator. Lys's own model is the app `lys`,
//! recorded once from the model file, which a later start does not read.
//! The apps store starts from its snapshot and reads only the leaves after it.

use std::error::Error;
use std::path::Path;
use std::sync::Arc;

#[path = "shared/bench.rs"]
mod bench;

use identity_contract::apps::{
    Auth, BEA, NOTES, TestResult, get, login, ok, op, post, put, refused, registered, registration,
    root, seeded, workspace_schema,
};
use identity_contract::harness::{ADMINISTRATOR, GRANT_MODEL, Service};
use lys_core::Ed25519Identity;
use lys_identity_server::Config;
use lys_identity_server::apps_state::{By, Line, Placed, Registered};
use lys_identity_server::apps_store::AppStore;
use lys_identity_server::dev_seed::seed_configured;
use lys_log_store::Start;
use serde_json::{Value, json};

/// The workspace schema without the workspace's `member` relation.
fn without_member() -> Value {
    let mut schema = workspace_schema(NOTES);
    schema["kinds"][format!("{NOTES}.workspace")]["relations"] =
        json!({"owner": ["read", "write"]});
    schema
}

#[tokio::test]
async fn a_registration_naming_a_service_account_without_their_store_is_refused_by_name()
-> TestResult {
    let (service, _) = Service::start_adjusted(
        GRANT_MODEL,
        None,
        None,
        None,
        |config| config.service_accounts_dir = None,
        |config| Ok(seed_configured(config, [ADMINISTRATOR, BEA])?),
    )
    .await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    let mut body = registration(NOTES, &workspace_schema(NOTES))?;
    body["service_account"] = json!("account-without-a-store");
    let answer = post(&service, "/apps", Auth::Cookie(&admin), &body).await?;
    refused(&answer, 503, "ServiceAccountsUnavailable")?;
    Ok(())
}

#[tokio::test]
async fn a_change_naming_a_stale_version_is_refused_schema_version_moved() -> TestResult {
    let (service, _) = seeded().await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    registered(&service, &admin, NOTES).await?;
    let path = format!("/apps/{NOTES}/schema");
    let change = json!({"operation": op()?, "replaces": 7, "schema": without_member()});
    let answer = put(&service, &path, Auth::Cookie(&admin), &change).await?;
    refused(&answer, 409, "schema_version_moved")?;
    assert_eq!(answer.1["fields"][0]["at"], "/replaces", "{}", answer.1);
    Ok(())
}

#[tokio::test]
async fn a_stranding_change_is_named_alike_by_its_dry_run_and_its_write_and_taken_after_revoking()
-> TestResult {
    let (service, seeded) = seeded().await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    let bea = seeded.people[1].id.to_string();
    registered(&service, &admin, NOTES).await?;
    let workspace = format!("{NOTES}.workspace");
    let mut grants = Vec::new();
    for id in ["one", "two"] {
        let issued = ok(root(&service, &admin, &bea, (&workspace, id), "member").await?)?;
        grants.push(issued["grant"].as_str().ok_or("no grant")?.to_owned());
    }

    let path = format!("/apps/{NOTES}/schema");
    let dry = json!({"replaces": 1, "schema": without_member()});
    let checked = ok(post(
        &service,
        &format!("{path}/check"),
        Auth::Cookie(&admin),
        &dry,
    )
    .await?)?;
    assert_eq!(checked["applies"], false, "{checked}");
    assert_eq!(
        checked["stranded"],
        json!([{"kind": workspace, "relation": "member", "count": 2}])
    );
    assert_eq!(
        checked["diff"]["relations_removed"],
        json!([{"kind": workspace, "name": "member"}])
    );
    let current = ok(get(&service, &path, Auth::Cookie(&admin)).await?)?;
    assert_eq!(current["version"], 1, "the dry run wrote nothing");

    let change = json!({"operation": op()?, "replaces": 1, "schema": without_member()});
    let answer = put(&service, &path, Auth::Cookie(&admin), &change).await?;
    refused(&answer, 409, "schema_change_strands_grants")?;
    let fields = &answer.1["fields"];
    assert_eq!(
        fields,
        &json!([{"at": format!("/schema/kinds/{workspace}/relations/member"), "count": 2}])
    );
    assert_eq!(
        fields[0]["count"], checked["stranded"][0]["count"],
        "the two name the same count"
    );

    for grant in &grants {
        let revoke = json!({"operation": op()?, "route": "api", "reason": "the relation is going"});
        ok(post(
            &service,
            &format!("/grants/{grant}/revoke"),
            Auth::Cookie(&admin),
            &revoke,
        )
        .await?)?;
    }
    let changed = ok(put(&service, &path, Auth::Cookie(&admin), &change).await?)?;
    assert_eq!(changed["applied"], true, "{changed}");
    assert_eq!(changed["app"]["version"], 2, "the version moves by one");

    let first = ok(get(&service, &format!("{path}?version=1"), Auth::Cookie(&admin)).await?)?;
    assert_eq!(
        first["schema"],
        workspace_schema(NOTES),
        "version 1 stays readable after 2"
    );
    let second = ok(get(&service, &format!("{path}?version=2"), Auth::Cookie(&admin)).await?)?;
    assert_eq!(second["schema"], without_member());
    let missing = get(&service, &format!("{path}?version=9"), Auth::Cookie(&admin)).await?;
    refused(&missing, 404, "schema_version_unknown")?;
    let gone = root(&service, &admin, &bea, (&workspace, "three"), "member").await?;
    assert_eq!(
        gone.0, 403,
        "a removed relation grants nothing new: {}",
        gone.1
    );
    Ok(())
}

#[tokio::test]
async fn a_change_an_app_makes_waits_for_the_administrator() -> TestResult {
    let (service, _) = seeded().await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    let credential = registered(&service, &admin, NOTES).await?;
    let path = format!("/apps/{NOTES}/schema");
    let mut wider = workspace_schema(NOTES);
    wider["kinds"][format!("{NOTES}.folder")] =
        json!({"actions": ["read"], "relations": {"viewer": ["read"]}});
    let change = json!({"operation": op()?, "replaces": 1, "schema": wider});
    let proposed = ok(put(&service, &path, Auth::Bearer(&credential), &change).await?)?;
    assert_eq!(proposed["applied"], false, "{proposed}");
    assert_eq!(proposed["app"]["version"], 1);
    assert_eq!(proposed["app"]["pending"]["replaces"], 1);
    let again = json!({"operation": op()?, "replaces": 1, "schema": wider});
    refused(
        &put(&service, &path, Auth::Bearer(&credential), &again).await?,
        409,
        "schema_change_pending",
    )?;
    let approve = json!({"operation": op()?});
    refused(
        &post(
            &service,
            &format!("{path}/approve"),
            Auth::Bearer(&credential),
            &approve,
        )
        .await?,
        403,
        "NotAdmitted",
    )?;
    let applied = ok(post(
        &service,
        &format!("{path}/approve"),
        Auth::Cookie(&admin),
        &approve,
    )
    .await?)?;
    assert_eq!(applied["app"]["version"], 2, "{applied}");
    assert_eq!(
        applied["diff"]["kinds_added"],
        json!([format!("{NOTES}.folder")])
    );
    let decline = json!({"operation": op()?});
    refused(
        &post(
            &service,
            &format!("{path}/decline"),
            Auth::Cookie(&admin),
            &decline,
        )
        .await?,
        409,
        "app_decided",
    )?;
    let lys =
        json!({"operation": op()?, "replaces": 1, "schema": {"relations": {"alpha": ["read"]}}});
    refused(
        &put(
            &service,
            "/apps/lys/schema",
            Auth::Bearer(&credential),
            &lys,
        )
        .await?,
        403,
        "NotAdmitted",
    )?;
    Ok(())
}

#[tokio::test]
async fn lys_is_an_app_whose_schema_is_the_model_and_changes_only_by_the_administrator()
-> TestResult {
    let (service, seeded) = seeded().await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    let bea = seeded.people[1].id.to_string();
    let lys = ok(get(&service, "/apps/lys", Auth::Cookie(&admin)).await?)?;
    assert_eq!(lys["state"], "approved", "{lys}");
    assert_eq!(
        lys["schema"],
        json!({"relations": {"alpha": ["read", "write"], "beta": ["read"]}})
    );
    ok(root(&service, &admin, &bea, ("doc", "1"), "alpha").await?)?;
    let dry = json!({"replaces": 1, "schema": {"relations": {"beta": ["read"]}}});
    let checked = ok(post(
        &service,
        "/apps/lys/schema/check",
        Auth::Cookie(&admin),
        &dry,
    )
    .await?)?;
    assert_eq!(
        checked["stranded"],
        json!([{"kind": "doc", "relation": "alpha", "count": 1}]),
        "{checked}"
    );
    let wider = json!({"operation": op()?, "replaces": 1, "schema": {"relations": {
        "alpha": ["read", "write"], "beta": ["read"], "gamma": ["read"],
    }}});
    let changed = ok(put(&service, "/apps/lys/schema", Auth::Cookie(&admin), &wider).await?)?;
    assert_eq!(changed["app"]["version"], 2, "{changed}");
    let model = ok(get(&service, "/grants/model", Auth::Cookie(&admin)).await?)?;
    assert_eq!(model["version"], 2, "{model}");
    ok(root(&service, &admin, &bea, ("doc", "2"), "gamma").await?)?;
    Ok(())
}

fn config(dir: &Path, model: &str) -> Result<Config, Box<dyn Error>> {
    std::fs::write(dir.join("grant-model.json"), model)?;
    let config: Config = serde_json::from_value(json!({
        "listen": "127.0.0.1:0",
        "log_dir": dir.join("log"),
        "log_origin": "example.test/lys/directory",
        "event_key_file": dir.join("service.key"),
        "issuer": "https://issuer.example.test",
        "client_id": "lys",
        "client_secret_file": dir.join("client.secret"),
        "redirect_url": "https://lys.example.test/callback",
        "administrator": {"issuer": "https://issuer.example.test", "subject": "ada"},
        "link_audit_source": {"issuer": "https://issuer.example.test", "subject": "audit"},
        "session_seconds": 600,
        "secure_cookie": false,
        "grant_log_dir": dir.join("grant-log"),
        "grant_log_origin": "example.test/lys/grants",
        "grant_model_file": dir.join("grant-model.json"),
    }))?;
    Ok(config)
}

fn key(dir: &Path) -> Result<Arc<Ed25519Identity>, Box<dyn Error>> {
    Ok(Arc::new(Ed25519Identity::load_or_generate(
        &dir.join("apps.key"),
    )?))
}

/// Open the apps beside `model` in `dir`, keeping what the start said.
fn start(
    dir: &Path,
    model: &str,
    said: &std::sync::Mutex<Vec<String>>,
) -> Result<lys_identity_server::apps_store::AppStore, Box<dyn Error>> {
    let config = config(dir, model)?;
    let say = |line: &str| {
        if let Ok(mut lines) = said.lock() {
            lines.push(line.to_owned());
        }
    };
    Ok(lys_identity_server::apps_api::opened(
        &config,
        key(dir)?,
        &say,
    )?)
}

fn count(said: &std::sync::Mutex<Vec<String>>, prefix: &str) -> usize {
    said.lock()
        .map(|lines| lines.iter().filter(|line| line.starts_with(prefix)).count())
        .unwrap_or_default()
}

#[test]
fn a_later_model_that_only_adds_is_applied_once_at_start() -> TestResult {
    let dir = tempfile::tempdir()?;
    let said = std::sync::Mutex::new(Vec::<String>::new());
    let first = r#"{"version":1,"relations":{"alpha":["read"]}}"#;
    let later = r#"{"version":2,"relations":{"alpha":["read","agent.stop"],"only.agent.stop":["agent.stop"]}}"#;
    let model = start(dir.path(), first, &said)?.model()?;
    assert_eq!(model.version(), 1);
    let model = start(dir.path(), later, &said)?.model()?;
    assert_eq!(model.version(), 2, "the later model is the next version");
    assert_eq!(model.relations().count(), 2);
    let again = start(dir.path(), later, &said)?.model()?;
    assert_eq!(
        again, model,
        "a second start with the same file adds nothing"
    );
    let earlier = start(dir.path(), first, &said)?.model()?;
    assert_eq!(earlier, model, "an earlier file is never applied");
    assert_eq!(count(&said, "lys_model_recorded"), 1);
    assert_eq!(count(&said, "lys_model_applied"), 1);
    assert_eq!(count(&said, "grant_model_file_ignored"), 2);
    Ok(())
}

#[test]
fn a_later_model_that_takes_an_action_away_refuses_the_start_by_name() -> TestResult {
    let dir = tempfile::tempdir()?;
    let said = std::sync::Mutex::new(Vec::<String>::new());
    let model = start(
        dir.path(),
        r#"{"version":1,"relations":{"alpha":["read"]}}"#,
        &said,
    )?
    .model()?;
    let refused = start(
        dir.path(),
        r#"{"version":5,"relations":{"omega":["write"]}}"#,
        &said,
    )
    .err()
    .ok_or("a model taking alpha away was applied")?;
    assert!(
        refused
            .to_string()
            .contains("takes read from the relation `alpha`"),
        "{refused}"
    );
    let kept = start(
        dir.path(),
        r#"{"version":1,"relations":{"alpha":["read"]}}"#,
        &said,
    )?;
    assert_eq!(kept.model()?, model, "the refused start changed nothing");
    Ok(())
}

fn placed(operation: &str, id: &str) -> Line {
    Line::Placed(Placed {
        operation: operation.to_owned(),
        app: NOTES.to_owned(),
        child_kind: format!("{NOTES}.channel"),
        child_id: id.to_owned(),
        parent_kind: format!("{NOTES}.workspace"),
        parent_id: "team".to_owned(),
        by: By::Start,
        at: 5,
    })
}

#[test]
fn the_apps_store_starts_from_its_snapshot_and_reads_only_the_leaves_after_it() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("apps");
    let store = AppStore::open(&path, key(dir.path())?)?;
    assert!(
        matches!(store.start(), Start::Rebuilt { replayed: 0, .. }),
        "{}",
        store.start()
    );
    drop(store);

    let mut store = AppStore::open(&path, key(dir.path())?)?;
    assert_eq!(
        store.start(),
        &Start::Resumed {
            size: 0,
            replayed: 0
        }
    );
    store.keep(Line::Registered(Registered {
        operation: "op-1".to_owned(),
        app: NOTES.to_owned(),
        name: "notes fixture".to_owned(),
        redirects: Vec::new(),
        schema: workspace_schema(NOTES),
        service_account: None,
        by: By::ServiceAccount {
            id: "fixture_registrar".to_owned(),
        },
        at: 4,
    }))?;
    let refused_placement = store.keep(placed("op-2", "general"));
    assert_eq!(
        refused_placement.err().map(|error| error.name()).as_deref(),
        Some("app_not_approved"),
        "a pending app's resources are placed nowhere"
    );
    let before = store.held().clone();
    assert_eq!(store.len(), 1);
    drop(store);

    let store = AppStore::open(&path, key(dir.path())?)?;
    assert_eq!(
        store.start(),
        &Start::Resumed {
            size: 0,
            replayed: 1
        },
        "a start reads the snapshot and only the leaves after it"
    );
    assert_eq!(store.held(), &before);
    assert_eq!(store.snapshot_failure(), None);
    Ok(())
}

#[tokio::test]
async fn the_bench_answers_the_draft_as_the_real_check_answers_it_once_saved() -> TestResult {
    let (service, seeded) = seeded().await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    let bea = seeded.people[1].id.to_string();
    bench::answers_as_saved(&service, &admin, &bea).await
}

#[tokio::test]
async fn a_bench_whose_namespace_is_gone_is_refused_by_name_on_close_and_is_closed() -> TestResult {
    let (service, _) = seeded().await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    let open = json!({"app": NOTES, "schema": workspace_schema(NOTES)});
    let opened = ok(post(&service, "/apps/bench", Auth::Cookie(&admin), &open).await?)?;
    let bench = opened["bench"].as_str().ok_or("no bench")?.to_owned();
    let namespace = service.dir.path().join("benches").join(&bench);
    assert!(namespace.is_dir(), "the bench's namespace is made on open");
    std::fs::remove_dir_all(&namespace)?;
    let close = format!("/apps/bench/{bench}/close");
    refused(
        &post(&service, &close, Auth::Cookie(&admin), &json!({})).await?,
        503,
        "apps_unavailable",
    )?;
    refused(
        &post(&service, &close, Auth::Cookie(&admin), &json!({})).await?,
        404,
        "bench_unknown",
    )?;
    Ok(())
}
