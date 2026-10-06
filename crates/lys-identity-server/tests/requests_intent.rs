//! An approval the service was stopped in the middle of: the intent is kept
//! and the decision is not. Whatever the grants hold for the intent's
//! operation settles it at the next decision, so a grant is never active
//! beside a request that waits or is declined.

use std::error::Error;
use std::sync::Arc;

use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_identity::OperationId;
use lys_identity::signer::load_service_key;
use lys_identity_server::dev_seed::{Seeded, seed_configured};
use lys_identity_server::requests_store::{Asked, Intended, RequestStore};
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

const BEA: &str = "bea-subject";
const FAR: u64 = 4_102_444_800;

fn login(subject: &str) -> Login {
    Login {
        subject: subject.to_owned(),
        email: "shared@example.test".to_owned(),
    }
}

fn operation() -> Result<String, Box<dyn Error>> {
    Ok(OperationId::generate()?.to_string())
}

fn refused(answer: &(u16, Value), status: u16, name: &str) {
    assert_eq!(answer.0, status, "{}", answer.1);
    assert_eq!(answer.1["refusal"], name, "{}", answer.1);
}

/// A service with Ada as the root authority and Bea, and their cookies.
struct Table {
    service: Service,
    seeded: Seeded,
    ada: String,
    bea: String,
}

impl Table {
    async fn decide(
        &self,
        cookie: &str,
        request: &str,
        act: &str,
        body: &Value,
    ) -> Result<(u16, Value), Box<dyn Error>> {
        self.service
            .post(&format!("/requests/{request}/{act}"), Some(cookie), body)
            .await
    }
}

fn approval() -> Result<Value, Box<dyn Error>> {
    Ok(json!({ "operation": operation()?, "route": "api", "source": null, "note": "another note" }))
}

/// A service that starts with Bea's request kept and Ada's approval of it
/// intended under `approving`, as it stands when the service stops after
/// keeping the intent.
async fn interrupted() -> Result<(Table, String, String), Box<dyn Error>> {
    let (asking, approving) = (operation()?, operation()?);
    let (id, under) = (asking.clone(), approving.clone());
    let (service, seeded) = Service::start_with(move |config| {
        let seeded = seed_configured(config, [ADMINISTRATOR, BEA])?;
        let (ada, bea) = (&seeded.people[0], &seeded.people[1]);
        let dir = config.requests_dir.as_deref().ok_or("no requests_dir")?;
        let mut store =
            RequestStore::open(dir, Arc::new(load_service_key(&config.event_key_file)?))?;
        store.ask(Asked {
            id: id.clone(),
            asked_by: bea.id.to_string(),
            responsible: bea.id.to_string(),
            resource_kind: "doc".to_owned(),
            resource_id: "1".to_owned(),
            relation: "beta".to_owned(),
            ends_at: Some(FAR),
            why: "to read the quarter's figures".to_owned(),
            asked_at: 5,
        })?;
        store.intend(Intended {
            id,
            by: ada.id.to_string(),
            operation: under,
            source: None,
            note: "for the quarter".to_owned(),
            intended_at: 6,
            answer: lys_identity_server::requests_store::Answer::AsAsked,
        })?;
        Ok(seeded)
    })
    .await?;
    let ada = service.sign_in(login(ADMINISTRATOR)).await?;
    let bea = service.sign_in(login(BEA)).await?;
    let table = Table {
        service,
        seeded,
        ada,
        bea,
    };
    Ok((table, asking, approving))
}

#[tokio::test]
async fn a_grant_issued_before_its_decision_was_kept_is_the_decision() -> TestResult {
    let (table, request, approving) = interrupted().await?;
    let bea = table.seeded.people[1].id.to_string();
    let root = json!({
        "operation": approving,
        "route": "api",
        "holder": bea,
        "resource": { "kind": "doc", "id": "1" },
        "relation": "beta",
        "pass_on": { "kind": "use_only" },
        "window": { "starts_at": 5, "ends_at": FAR },
    });
    let (status, issued) = table
        .service
        .post("/grants/roots", Some(&table.ada), &root)
        .await?;
    assert_eq!(status, 200, "{issued}");
    let (_, seen) = table.service.get("/requests", Some(&table.bea)).await?;
    assert_eq!(seen["requests"][0]["state"], "waiting");
    assert_eq!(
        seen["requests"][0]["held_by"],
        table.seeded.people[0].id.to_string()
    );

    let declined = table
        .decide(
            &table.ada,
            &request,
            "decline",
            &json!({ "note": "not now" }),
        )
        .await?;
    refused(&declined, 409, "RequestDecided");
    let (status, approved) = table
        .decide(&table.ada, &request, "approve", &approval()?)
        .await?;
    assert_eq!(
        status, 200,
        "another operation answers the kept decision: {approved}"
    );
    assert_eq!(approved["state"], "approved");
    assert_eq!(approved["decision"]["grant"], issued["grant"]);
    assert_eq!(approved["decision"]["note"], "for the quarter");
    assert_eq!(approved["held_by"], Value::Null);
    let (_, held) = table.service.get("/grants", Some(&table.ada)).await?;
    assert_eq!(
        held["grants"].as_array().map(Vec::len),
        Some(1),
        "no second grant is issued: {held}"
    );
    Ok(())
}

#[tokio::test]
async fn an_intent_the_grants_hold_nothing_for_is_withdrawn_at_the_next_decision() -> TestResult {
    let (table, request, _) = interrupted().await?;
    let (status, declined) = table
        .decide(
            &table.ada,
            &request,
            "decline",
            &json!({ "note": "not now" }),
        )
        .await?;
    assert_eq!(status, 200, "{declined}");
    assert_eq!(declined["state"], "declined");
    assert_eq!(declined["held_by"], Value::Null);
    let (_, held) = table.service.get("/grants", Some(&table.ada)).await?;
    assert_eq!(held["grants"], json!([]), "{held}");
    Ok(())
}

#[tokio::test]
async fn anyone_the_request_is_shown_to_settles_the_approval_without_deciding() -> TestResult {
    let (table, request, approving) = interrupted().await?;
    let path = format!("/requests/{request}/reconcile");
    let stranger = table.service.sign_in(login("stranger-subject")).await?;
    let hidden = table
        .service
        .post(&path, Some(&stranger), &json!({}))
        .await?;
    assert_ne!(hidden.0, 200, "{}", hidden.1);

    let bea = table.seeded.people[1].id.to_string();
    let root = json!({
        "operation": approving,
        "route": "api",
        "holder": bea,
        "resource": { "kind": "doc", "id": "1" },
        "relation": "beta",
        "pass_on": { "kind": "use_only" },
        "window": { "starts_at": 5, "ends_at": FAR },
    });
    let (status, issued) = table
        .service
        .post("/grants/roots", Some(&table.ada), &root)
        .await?;
    assert_eq!(status, 200, "{issued}");
    let (status, settled) = table
        .service
        .post(&path, Some(&table.bea), &json!({}))
        .await?;
    assert_eq!(status, 200, "{settled}");
    assert_eq!(settled["state"], "approved");
    assert_eq!(settled["decision"]["grant"], issued["grant"]);
    assert_eq!(
        settled["decision"]["by"],
        table.seeded.people[0].id.to_string()
    );
    assert_eq!(settled["held_by"], Value::Null);
    Ok(())
}

#[tokio::test]
async fn settling_an_approval_the_grants_hold_nothing_for_leaves_the_request_waiting() -> TestResult
{
    let (table, request, _) = interrupted().await?;
    let (status, settled) = table
        .service
        .post(
            &format!("/requests/{request}/reconcile"),
            Some(&table.bea),
            &json!({}),
        )
        .await?;
    assert_eq!(status, 200, "{settled}");
    assert_eq!(settled["state"], "waiting");
    assert_eq!(settled["held_by"], Value::Null);
    assert_eq!(settled["decision"], Value::Null);
    let (_, held) = table.service.get("/grants", Some(&table.ada)).await?;
    assert_eq!(held["grants"], json!([]), "{held}");
    Ok(())
}

/// DIRECTORY-080 R1 (box 12.5): only a person is given access from the root
/// authority. A connector's request approved with no source is refused by
/// name before any intent is kept, and it still waits.
#[tokio::test]
async fn a_connectors_request_approved_with_no_source_is_refused_before_its_intent() -> TestResult {
    let asking = operation()?;
    let id = asking.clone();
    let (service, _) = Service::start_with(move |config| {
        let seeded = seed_configured(config, [ADMINISTRATOR, BEA])?;
        let dir = config.requests_dir.as_deref().ok_or("no requests_dir")?;
        let mut store =
            RequestStore::open(dir, Arc::new(load_service_key(&config.event_key_file)?))?;
        store.ask(Asked {
            id,
            asked_by: lys_identity::ConnectorId::generate()?.to_string(),
            responsible: seeded.people[0].id.to_string(),
            resource_kind: "doc".to_owned(),
            resource_id: "1".to_owned(),
            relation: "beta".to_owned(),
            ends_at: Some(FAR),
            why: "to read the quarter's figures".to_owned(),
            asked_at: 5,
        })?;
        Ok(seeded)
    })
    .await?;
    let ada = service.sign_in(login(ADMINISTRATOR)).await?;
    let answer = service
        .post(
            &format!("/requests/{asking}/approve"),
            Some(&ada),
            &approval()?,
        )
        .await?;
    refused(&answer, 400, "RequestMalformed");
    assert!(
        answer
            .1
            .to_string()
            .contains("a connector's access is lent"),
        "{}",
        answer.1
    );
    let (_, seen) = service.get("/requests", Some(&ada)).await?;
    assert_eq!(seen["requests"][0]["id"], asking.as_str(), "{seen}");
    assert_eq!(seen["requests"][0]["state"], "waiting", "{seen}");
    let (_, held) = service.get("/grants", Some(&ada)).await?;
    assert_eq!(held["grants"], json!([]), "{held}");
    Ok(())
}
