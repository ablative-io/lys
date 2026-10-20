#![cfg(test)]

//! A question that cannot be answered is named, never drawn as nobody: with
//! `SpiceDB` answering the service but refusing every relationship read, with
//! a grant recorded that it would not take, and with a revocation the grant
//! log could not record, `/grants/reach` and `/grants/who` answer the named
//! 503 rather than a page with no holders. An unreadable log tail keeps its
//! original reconciliation refusal.

use std::error::Error;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, GRANT_MODEL, Service};
use lys_identity::OperationId;
use lys_identity_server::dev_seed::seed_configured;
use lys_identity_server::spicedb::SpiceDbSettings;
use serde_json::json;

type TestResult = Result<(), Box<dyn Error>>;

#[path = "support/grant_resource_engine.rs"]
mod grant_resource_engine;

#[tokio::test]
async fn an_unreadable_engine_is_named_by_reach_and_who_after_a_live_grant() -> TestResult {
    let broken = Arc::new(AtomicBool::new(false));
    let mut engine = grant_resource_engine::Engine::start(
        Arc::clone(&broken),
        Arc::new(AtomicBool::new(false)),
    )?;
    let key = tempfile::NamedTempFile::new()?;
    std::fs::write(key.path(), "fixture-only")?;
    let settings = SpiceDbSettings {
        endpoint: engine.endpoint.clone(),
        key_file: key.path().to_owned(),
        mirror: "engine_down_fixture".to_owned(),
    };
    let (mut service, seeded) = Service::start_judging(GRANT_MODEL, Some(settings), |config| {
        Ok(seed_configured(config, [ADMINISTRATOR, "bea-subject"])?)
    })
    .await?;
    let ada = service
        .sign_in(Login {
            subject: ADMINISTRATOR.to_owned(),
            email: "ada@example.test".to_owned(),
        })
        .await?;
    let holder = seeded.people[1].id.to_string();
    let root = json!({
        "operation": OperationId::generate()?.to_string(), "route": "browser",
        "holder": holder, "resource": { "kind": "doc", "id": "1" },
        "relation": "alpha", "pass_on": { "kind": "use_only" },
        "window": { "starts_at": 0, "ends_at": null },
    });
    let (status, issued) = service.post("/grants/roots", Some(&ada), &root).await?;
    assert_eq!(status, 200, "live grant before the outage: {issued}");
    let reach = json!({
        "route": "browser",
        "resources": [{ "kind": "doc", "id": "1", "actions": ["read"] }],
    });
    let who = json!({
        "route": "browser", "resource": { "kind": "doc", "id": "1" }, "action": "read",
        "page_size": 10, "after": null,
    });
    for (path, body) in [("/grants/reach", &reach), ("/grants/who", &who)] {
        let (status, answer) = service.post(path, Some(&ada), body).await?;
        assert_eq!(status, 200, "{path} while the engine answers: {answer}");
        let holders = if path == "/grants/reach" {
            &answer["resources"][0]["holders"]
        } else {
            &answer["holders"]
        };
        assert_eq!(holders.as_array().ok_or("holders missing")?.len(), 1);
        assert_eq!(holders[0]["holder"], holder, "{path}: {answer}");
    }
    broken.store(true, Ordering::SeqCst);
    for (path, body) in [("/grants/reach", &reach), ("/grants/who", &who)] {
        let (status, answer) = service.post(path, Some(&ada), body).await?;
        assert_eq!(status, 503, "{path}: {answer}");
        assert_eq!(
            answer["refusal"], "PermissionEngineUnavailable",
            "{path}: {answer}"
        );
    }
    service.close()?;
    engine.stop()?;
    Ok(())
}

#[tokio::test]
async fn an_unrecorded_revocation_names_the_unresolved_operation_then_the_unreadable_tail()
-> TestResult {
    let mut engine = grant_resource_engine::Engine::start(
        Arc::new(AtomicBool::new(false)),
        Arc::new(AtomicBool::new(false)),
    )?;
    let key = tempfile::NamedTempFile::new()?;
    std::fs::write(key.path(), "fixture-only")?;
    let settings = SpiceDbSettings {
        endpoint: engine.endpoint.clone(),
        key_file: key.path().to_owned(),
        mirror: "unresolved_fixture".to_owned(),
    };
    let (mut service, (seeded, grant_log)) =
        Service::start_judging(GRANT_MODEL, Some(settings), |config| {
            let seeded = seed_configured(config, [ADMINISTRATOR, "bea-subject"])?;
            Ok((seeded, config.grant_log_dir.clone()))
        })
        .await?;
    let ada = service
        .sign_in(Login {
            subject: ADMINISTRATOR.to_owned(),
            email: "ada@example.test".to_owned(),
        })
        .await?;
    let doc = json!({ "kind": "doc", "id": "1" });
    let root = json!({
        "operation": OperationId::generate()?.to_string(), "route": "api",
        "holder": seeded.people[1].id.to_string(), "resource": doc, "relation": "alpha",
        "pass_on": { "kind": "use_only" }, "window": { "starts_at": 0, "ends_at": null },
    });
    let (status, issued) = service.post("/grants/roots", Some(&ada), &root).await?;
    assert_eq!(status, 200, "{issued}");
    let grant = issued["grant"].as_str().ok_or("no grant")?;

    let aside = grant_log.with_extension("aside");
    std::fs::rename(&grant_log, &aside)?;
    let revoke = json!({
        "operation": OperationId::generate()?.to_string(), "route": "api",
        "reason": "the log cannot take it",
    });
    let (status, answer) = service
        .post(&format!("/grants/{grant}/revoke"), Some(&ada), &revoke)
        .await?;
    assert_eq!(status, 503, "the revocation is not recorded: {answer}");
    assert_eq!(answer["refusal"], "OperationUnresolved", "{answer}");
    let reach = json!({
        "route": "browser",
        "resources": [{ "kind": "doc", "id": "1", "actions": ["read"] }],
    });
    let who = json!({
        "route": "browser", "resource": doc, "action": "read", "page_size": 10, "after": null,
    });
    for (path, body) in [("/grants/reach", &reach), ("/grants/who", &who)] {
        let (status, answer) = service.post(path, Some(&ada), body).await?;
        assert_eq!(status, 503, "{path}: {answer}");
        assert_eq!(answer["refusal"], "LogUnavailable", "{path}: {answer}");
    }
    std::fs::rename(&aside, &grant_log)?;
    service.close()?;
    engine.stop()?;
    Ok(())
}

#[tokio::test]
async fn a_grant_the_engine_would_not_take_is_named_pending_by_reach_and_who() -> TestResult {
    let refusing = Arc::new(AtomicBool::new(false));
    let mut engine = grant_resource_engine::Engine::start(
        Arc::new(AtomicBool::new(false)),
        Arc::clone(&refusing),
    )?;
    let key = tempfile::NamedTempFile::new()?;
    std::fs::write(key.path(), "fixture-only")?;
    let settings = SpiceDbSettings {
        endpoint: engine.endpoint.clone(),
        key_file: key.path().to_owned(),
        mirror: "pending_fixture".to_owned(),
    };
    let (mut service, seeded) = Service::start_judging(GRANT_MODEL, Some(settings), |config| {
        Ok(seed_configured(config, [ADMINISTRATOR, "bea-subject"])?)
    })
    .await?;
    let ada = service
        .sign_in(Login {
            subject: ADMINISTRATOR.to_owned(),
            email: "ada@example.test".to_owned(),
        })
        .await?;
    let doc = json!({ "kind": "doc", "id": "1" });
    // The served grants open at the first question, folding the founding
    // grant into the engine (DIRECTORY-109 R3); the refusal starts after.
    let (status, opened) = service.get("/grants", Some(&ada)).await?;
    assert_eq!(status, 200, "{opened}");
    refusing.store(true, Ordering::SeqCst);
    let root = json!({
        "operation": OperationId::generate()?.to_string(), "route": "api",
        "holder": seeded.people[1].id.to_string(), "resource": doc, "relation": "alpha",
        "pass_on": { "kind": "use_only" }, "window": { "starts_at": 0, "ends_at": null },
    });
    let (status, answer) = service.post("/grants/roots", Some(&ada), &root).await?;
    assert_eq!(status, 503, "recorded, not yet taken: {answer}");
    assert_eq!(answer["refusal"], "ProjectionPending", "{answer}");
    let reach = json!({
        "route": "browser",
        "resources": [{ "kind": "doc", "id": "1", "actions": ["read"] }],
    });
    let who = json!({
        "route": "browser", "resource": doc, "action": "read", "page_size": 10, "after": null,
    });
    for (path, body) in [("/grants/reach", &reach), ("/grants/who", &who)] {
        let (status, answer) = service.post(path, Some(&ada), body).await?;
        assert_eq!(status, 503, "{path}: {answer}");
        assert_eq!(answer["refusal"], "ProjectionPending", "{path}: {answer}");
    }
    service.close()?;
    engine.stop()?;
    Ok(())
}
