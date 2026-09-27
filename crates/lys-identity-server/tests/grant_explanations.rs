//! The two questions: why the caller may act, and who can. Both use the one
//! evaluator at one revision, the reverse question pages every authorised
//! holder exactly once and names its continuation, and a grant the caller may
//! not inspect leaks neither its id, its labels nor its existence.

use std::collections::BTreeSet;
use std::error::Error;

use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_identity::OperationId;
use lys_identity::grants::GrantId;
use lys_identity_server::dev_seed::{Seeded, seed_configured};
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

const BEA: &str = "bea-subject";

fn login(subject: &str) -> Login {
    Login {
        subject: subject.to_owned(),
        email: "shared@example.test".to_owned(),
    }
}

async fn seeded() -> Result<(Service, Seeded), Box<dyn Error>> {
    Service::start_with(|config| Ok(seed_configured(config, [ADMINISTRATOR, BEA])?)).await
}

fn operation() -> Result<String, Box<dyn Error>> {
    Ok(OperationId::generate()?.to_string())
}

fn question(resource: &str) -> Value {
    json!({ "route": "api", "resource": { "kind": "doc", "id": resource }, "action": "read" })
}

async fn post_ok(
    service: &Service,
    path: &str,
    cookie: &str,
    body: &Value,
) -> Result<Value, Box<dyn Error>> {
    let (status, answer) = service.post(path, Some(cookie), body).await?;
    assert_eq!(status, 200, "{path}: {answer}");
    Ok(answer)
}

/// Ada holds a private root on doc 1. Bea holds a root on doc 1 that lends
/// read to agents, and has lent it to her reviewer. Answers the three grant ids.
async fn world(
    service: &Service,
    seeded: &Seeded,
    ada: &str,
    bea: &str,
) -> Result<(String, String, String), Box<dyn Error>> {
    let root = |holder: String, pass_on: Value| -> Result<Value, Box<dyn Error>> {
        Ok(json!({
            "operation": operation()?, "route": "api", "holder": holder,
            "resource": { "kind": "doc", "id": "1" }, "relation": "alpha", "pass_on": pass_on,
            "window": { "starts_at": 0, "ends_at": null },
        }))
    };
    let (ada_person, bea_person) = (&seeded.people[0], &seeded.people[1]);
    let private_body = root(ada_person.id.to_string(), json!({ "kind": "use_only" }))?;
    let private = post_ok(service, "/grants/roots", ada, &private_body).await?;
    let lending_body = root(
        bea_person.id.to_string(),
        json!({ "kind": "to", "actions": ["read"], "recipients": ["agent"] }),
    )?;
    let lending = post_ok(service, "/grants/roots", ada, &lending_body).await?;
    let lending_id = lending["grant"].as_str().ok_or("no grant")?.to_owned();
    let operation_id = operation()?;
    let lent_body = json!({
            "operation": operation_id, "route": "tool", "source": lending_id,
            "recipient": bea_person.agents[0].id.to_string(), "responsible": bea_person.id.to_string(),
            "resource": { "kind": "doc", "id": "1" }, "relation": "beta",
            "pass_on": { "kind": "use_only" }, "window": { "starts_at": 0, "ends_at": null },
    });
    let lent = post_ok(service, "/grants", bea, &lent_body).await?;
    Ok((
        private["grant"].as_str().ok_or("no grant")?.to_owned(),
        lending_id,
        lent["grant"].as_str().ok_or("no grant")?.to_owned(),
    ))
}

/// Every page of the who-can answer for `cookie`, one holder at a time.
async fn every_page(
    service: &Service,
    cookie: &str,
) -> Result<(Vec<Value>, usize), Box<dyn Error>> {
    let mut holders = Vec::new();
    let mut after = Value::Null;
    let mut pages = 0;
    loop {
        let mut body = question("1");
        body["page_size"] = json!(1);
        body["after"] = after.clone();
        let page = post_ok(service, "/grants/who", cookie, &body).await?;
        pages += 1;
        holders.extend(
            page["holders"]
                .as_array()
                .ok_or("no holders")?
                .iter()
                .cloned(),
        );
        if page["complete"] == true {
            assert_eq!(page["next"], Value::Null, "{page}");
            return Ok((holders, pages));
        }
        after = page["next"].clone();
        assert!(
            after.is_string(),
            "an incomplete page names its continuation: {page}"
        );
        assert!(pages <= 10, "the pages never completed");
    }
}

#[tokio::test]
async fn forward_and_reverse_agree_and_every_holder_is_paged_once() -> TestResult {
    let (service, seeded) = seeded().await?;
    let ada = service.sign_in(login(ADMINISTRATOR)).await?;
    let bea = service.sign_in(login(BEA)).await?;
    let (private, lending, lent) = world(&service, &seeded, &ada, &bea).await?;

    let forward = post_ok(&service, "/grants/why", &bea, &question("1")).await?;
    assert_eq!(forward["permitted"], true, "{forward}");
    assert_eq!(forward["grant"], lending.as_str());
    assert_eq!(forward["path"], json!([lending]));
    assert_eq!(forward["responsible"], seeded.people[1].id.to_string());

    let (holders, pages) = every_page(&service, &ada).await?;
    let ids: Vec<&str> = holders
        .iter()
        .filter_map(|holder| holder["holder"].as_str())
        .collect();
    let unique: BTreeSet<&str> = ids.iter().copied().collect();
    assert_eq!(ids.len(), unique.len(), "a holder was paged twice: {ids:?}");
    let expected: BTreeSet<String> = [
        seeded.people[0].id.to_string(),
        seeded.people[1].id.to_string(),
        seeded.people[1].agents[0].id.to_string(),
    ]
    .into();
    assert_eq!(
        unique
            .iter()
            .map(|id| (*id).to_owned())
            .collect::<BTreeSet<_>>(),
        expected
    );
    assert!(pages >= 3, "one holder a page takes at least three pages");
    let bea_answer = holders
        .iter()
        .find(|holder| holder["holder"] == seeded.people[1].id.to_string())
        .ok_or("Bea is not among the holders")?;
    for member in [
        "grant",
        "path",
        "responsible",
        "scope",
        "model_version",
        "revision",
    ] {
        assert_eq!(
            bea_answer[member], forward[member],
            "{member} differs between the two questions"
        );
    }
    let agent = holders
        .iter()
        .find(|holder| holder["holder"] == seeded.people[1].agents[0].id.to_string())
        .ok_or("the reviewer is not among the holders")?;
    assert_eq!(agent["path"], json!([lent, lending]));
    assert_ne!(private, lending);
    Ok(())
}

#[tokio::test]
async fn a_grant_the_caller_may_not_inspect_leaks_nothing() -> TestResult {
    let (service, seeded) = seeded().await?;
    let ada = service.sign_in(login(ADMINISTRATOR)).await?;
    let bea = service.sign_in(login(BEA)).await?;
    let (private, _, _) = world(&service, &seeded, &ada, &bea).await?;
    let ada_id = seeded.people[0].id.to_string();

    let (holders, _) = every_page(&service, &bea).await?;
    let text = serde_json::to_string(&holders)?;
    assert!(
        !text.contains(&private) && !text.contains(&ada_id),
        "{text}"
    );
    let (status, listed) = service.get("/grants", Some(&bea)).await?;
    assert_eq!(status, 200);
    assert!(!listed.to_string().contains(&private), "{listed}");

    let unknown = GrantId::generate()?.to_string();
    let hidden = service
        .get(&format!("/grants/{private}"), Some(&bea))
        .await?;
    let absent = service
        .get(&format!("/grants/{unknown}"), Some(&bea))
        .await?;
    assert_eq!(hidden.0, absent.0);
    assert_eq!(hidden.1["refusal"], absent.1["refusal"]);
    assert_eq!(hidden.1["refusal"], "GrantNotVisible");

    let revoke = |grant: &str| -> Result<(String, Value), Box<dyn Error>> {
        Ok((
            format!("/grants/{grant}/revoke"),
            json!({ "operation": operation()?, "route": "api", "reason": "probing" }),
        ))
    };
    let (path, body) = revoke(&private)?;
    let hidden = service.post(&path, Some(&bea), &body).await?;
    let (path, body) = revoke(&unknown)?;
    let absent = service.post(&path, Some(&bea), &body).await?;
    assert_eq!(hidden.0, absent.0);
    assert_eq!(hidden.1["refusal"], absent.1["refusal"]);
    assert_eq!(
        hidden.1["reason"]
            .as_str()
            .map(|reason| reason.replace(&private, "")),
        absent.1["reason"]
            .as_str()
            .map(|reason| reason.replace(&unknown, "")),
        "the two refusals differ in more than the id each was asked about"
    );
    Ok(())
}
