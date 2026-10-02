//! The access request routes: a request gives no access, only a caller the
//! grants would let give the access can approve it, an approval issues a
//! grant that ends when the request said, and a request is shown only to
//! those it concerns. The service runs with only its own disposable log and
//! the in-process issuer, and no other server.

use std::error::Error;

use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_identity::OperationId;
use lys_identity_server::dev_seed::{Seeded, seed_configured};
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

const BEA: &str = "bea-subject";
const STRANGER: &str = "stranger-subject";
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
    async fn set() -> Result<Self, Box<dyn Error>> {
        let (service, seeded) =
            Service::start_with(|config| Ok(seed_configured(config, [ADMINISTRATOR, BEA])?))
                .await?;
        let ada = service.sign_in(login(ADMINISTRATOR)).await?;
        let bea = service.sign_in(login(BEA)).await?;
        Ok(Self {
            service,
            seeded,
            ada,
            bea,
        })
    }

    fn ask_body(resource: &str, relation: &str) -> Result<Value, Box<dyn Error>> {
        Ok(json!({
            "operation": operation()?,
            "resource": { "kind": "doc", "id": resource },
            "relation": relation,
            "ends_at": FAR,
            "why": "to read the quarter's figures",
        }))
    }

    async fn ask(
        &self,
        cookie: &str,
        resource: &str,
        relation: &str,
    ) -> Result<Value, Box<dyn Error>> {
        let body = Self::ask_body(resource, relation)?;
        let (status, asked) = self.service.post("/requests", Some(cookie), &body).await?;
        assert_eq!(status, 200, "{asked}");
        assert_eq!(asked["id"], body["operation"]);
        assert_eq!(asked["state"], "waiting");
        Ok(asked)
    }

    async fn decide(
        &self,
        cookie: &str,
        request: &Value,
        act: &str,
        body: &Value,
    ) -> Result<(u16, Value), Box<dyn Error>> {
        let id = request["id"].as_str().ok_or("no id")?;
        self.service
            .post(&format!("/requests/{id}/{act}"), Some(cookie), body)
            .await
    }

    async fn listed(&self, cookie: &str) -> Result<Vec<String>, Box<dyn Error>> {
        let (status, body) = self.service.get("/requests", Some(cookie)).await?;
        assert_eq!(status, 200, "{body}");
        Ok(body["requests"]
            .as_array()
            .ok_or("not a list")?
            .iter()
            .filter_map(|request| request["id"].as_str().map(str::to_owned))
            .collect())
    }

    /// Ada issues Bea a root grant on `resource` that Bea may lend to people for reading.
    async fn lendable(&self, resource: &str) -> Result<String, Box<dyn Error>> {
        let root = json!({
            "operation": operation()?,
            "route": "api",
            "holder": self.seeded.people[1].id.to_string(),
            "resource": { "kind": "doc", "id": resource },
            "relation": "alpha",
            "pass_on": { "kind": "to", "actions": ["read"], "recipients": ["person"] },
            "window": { "starts_at": 0, "ends_at": null },
        });
        let (status, issued) = self
            .service
            .post("/grants/roots", Some(&self.ada), &root)
            .await?;
        assert_eq!(status, 200, "{issued}");
        Ok(issued["grant"].as_str().ok_or("no grant")?.to_owned())
    }
}

fn approval(source: Option<&str>) -> Result<Value, Box<dyn Error>> {
    Ok(
        json!({ "operation": operation()?, "route": "api", "source": source, "note": "for the quarter" }),
    )
}

#[tokio::test]
async fn the_root_authority_approves_a_persons_request_and_the_grant_ends_as_asked() -> TestResult {
    let table = Table::set().await?;
    let (ada, bea) = (&table.seeded.people[0], &table.seeded.people[1]);
    let asked = table.ask(&table.bea, "9", "alpha").await?;
    assert_eq!(asked["asked_by"], bea.id.to_string());
    assert_eq!(asked["responsible"]["id"], bea.id.to_string());
    assert_eq!(asked["actions"], json!(["read", "write"]));
    assert_eq!(asked["approvers"][0]["id"], ada.id.to_string());
    assert_eq!(asked["approvers"].as_array().map(Vec::len), Some(1));
    let (_, before) = table.service.get("/grants", Some(&table.bea)).await?;
    assert_eq!(before["grants"], json!([]), "a request gives no access");

    let (status, approved) = table
        .decide(&table.ada, &asked, "approve", &approval(None)?)
        .await?;
    assert_eq!(status, 200, "{approved}");
    assert_eq!(approved["state"], "approved");
    assert_eq!(approved["decision"]["by"], ada.id.to_string());
    let grant = approved["decision"]["grant"].as_str().ok_or("no grant")?;
    let (status, held) = table
        .service
        .get(&format!("/grants/{grant}"), Some(&table.bea))
        .await?;
    assert_eq!(status, 200, "{held}");
    assert_eq!(held["holder"], bea.id.to_string());
    assert_eq!(held["resource"], json!({ "kind": "doc", "id": "9" }));
    assert_eq!(held["relation"], "alpha");
    assert_eq!(held["window"]["ends_at"], FAR);
    assert_eq!(held["pass_on"]["kind"], "use_only");

    let again = table
        .decide(&table.ada, &asked, "approve", &approval(None)?)
        .await?;
    assert_eq!(again.0, 200, "{}", again.1);
    assert_eq!(
        again.1["decision"]["grant"], grant,
        "approved again, no second grant"
    );
    let (_, after) = table.service.get("/grants", Some(&table.bea)).await?;
    assert_eq!(after["grants"].as_array().map(Vec::len), Some(1));
    Ok(())
}

#[tokio::test]
async fn a_person_holding_a_lendable_grant_approves_from_it() -> TestResult {
    let table = Table::set().await?;
    let (ada, bea) = (&table.seeded.people[0], &table.seeded.people[1]);
    let source = table.lendable("1").await?;
    let asked = table.ask(&table.ada, "1", "beta").await?;
    let approvers: Vec<&str> = asked["approvers"]
        .as_array()
        .ok_or("not a list")?
        .iter()
        .filter_map(|person| person["id"].as_str())
        .collect();
    let (ada_id, bea_id) = (ada.id.to_string(), bea.id.to_string());
    assert_eq!(approvers, [ada_id.as_str(), bea_id.as_str()]);
    assert_eq!(
        table.listed(&table.bea).await?,
        [asked["id"].as_str().ok_or("no id")?]
    );

    let wider = table.ask(&table.ada, "1", "alpha").await?;
    assert_eq!(
        wider["approvers"].as_array().map(Vec::len),
        Some(1),
        "Bea may lend reading only, so she cannot approve alpha: {wider}"
    );

    let (status, approved) = table
        .decide(&table.bea, &asked, "approve", &approval(Some(&source))?)
        .await?;
    assert_eq!(status, 200, "{approved}");
    assert_eq!(approved["decision"]["by"], bea_id);
    let grant = approved["decision"]["grant"].as_str().ok_or("no grant")?;
    let (_, held) = table
        .service
        .get(&format!("/grants/{grant}"), Some(&table.ada))
        .await?;
    assert_eq!(held["holder"], ada_id);
    assert_eq!(held["source"], source);
    assert_eq!(held["actions"], json!(["read"]));
    Ok(())
}

#[tokio::test]
async fn a_declined_request_stays_declined_and_gives_nothing() -> TestResult {
    let table = Table::set().await?;
    let asked = table.ask(&table.bea, "9", "beta").await?;
    let own = table
        .decide(&table.bea, &asked, "approve", &approval(None)?)
        .await?;
    refused(&own, 403, "NotAdmitted");
    let note = json!({ "note": "not this quarter" });
    let (status, declined) = table.decide(&table.ada, &asked, "decline", &note).await?;
    assert_eq!(status, 200, "{declined}");
    assert_eq!(declined["state"], "declined");
    assert_eq!(declined["decision"]["grant"], Value::Null);
    let (status, again) = table.decide(&table.ada, &asked, "decline", &note).await?;
    assert_eq!(status, 200, "{again}");
    assert_eq!(again["decision"], declined["decision"]);
    let late = table
        .decide(&table.ada, &asked, "approve", &approval(None)?)
        .await?;
    refused(&late, 409, "RequestDecided");
    let (_, grants) = table.service.get("/grants", Some(&table.bea)).await?;
    assert_eq!(grants["grants"], json!([]));
    Ok(())
}

#[tokio::test]
async fn a_request_is_shown_only_to_those_it_concerns() -> TestResult {
    let table = Table::set().await?;
    let asked = table.ask(&table.ada, "7", "beta").await?;
    assert_eq!(table.listed(&table.ada).await?.len(), 1);
    assert_eq!(table.listed(&table.bea).await?, Vec::<String>::new());
    let hidden = table
        .decide(&table.bea, &asked, "decline", &json!({ "note": "no" }))
        .await?;
    refused(&hidden, 404, "RequestUnknown");
    let unknown = json!({ "id": operation()? });
    let absent = table
        .decide(&table.bea, &unknown, "decline", &json!({ "note": "no" }))
        .await?;
    assert_eq!(
        absent, hidden,
        "a hidden request answers as one that is not kept"
    );

    let none = table.service.get("/requests", None).await?;
    refused(&none, 401, "NotSignedIn");
    let stranger = table.service.sign_in(login(STRANGER)).await?;
    let outside = table.service.get("/requests", Some(&stranger)).await?;
    refused(&outside, 403, "NoPerson");
    let body = Table::ask_body("7", "beta")?;
    let outside = table
        .service
        .post("/requests", Some(&stranger), &body)
        .await?;
    refused(&outside, 403, "NoPerson");
    Ok(())
}

#[tokio::test]
async fn a_request_the_route_does_not_take_is_refused_by_name_and_kept_nowhere() -> TestResult {
    let table = Table::set().await?;
    let good = Table::ask_body("3", "beta")?;
    let mut refusals = 0;
    for (member, value, status, name) in [
        ("holder", json!("someone-else"), 400, "RequestMalformed"),
        ("why", json!("   "), 400, "RequestMalformed"),
        ("why", json!("a".repeat(501)), 400, "RequestMalformed"),
        ("ends_at", json!(1), 400, "RequestMalformed"),
        ("relation", json!("omega"), 403, "RelationUnknown"),
    ] {
        let mut body = good.clone();
        body[member] = value;
        let answer = table
            .service
            .post("/requests", Some(&table.bea), &body)
            .await?;
        refused(&answer, status, name);
        refusals += 1;
    }
    assert_eq!(refusals, 5);
    assert_eq!(table.listed(&table.ada).await?, Vec::<String>::new());

    let (status, first) = table
        .service
        .post("/requests", Some(&table.bea), &good)
        .await?;
    assert_eq!(status, 200, "{first}");
    let (status, second) = table
        .service
        .post("/requests", Some(&table.bea), &good)
        .await?;
    assert_eq!(status, 200, "{second}");
    assert_eq!(first["id"], second["id"]);
    assert_eq!(table.listed(&table.bea).await?.len(), 1);
    let mut other = good.clone();
    other["why"] = json!("other words");
    let reused = table
        .service
        .post("/requests", Some(&table.bea), &other)
        .await?;
    refused(&reused, 409, "RequestReused");
    Ok(())
}

#[tokio::test]
async fn a_kept_request_is_answered_again_after_the_end_it_asked_for() -> TestResult {
    let table = Table::set().await?;
    let soon = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs()
        + 2;
    let mut body = Table::ask_body("4", "beta")?;
    body["ends_at"] = json!(soon);
    let (status, first) = table
        .service
        .post("/requests", Some(&table.bea), &body)
        .await?;
    assert_eq!(status, 200, "{first}");
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    let (status, replayed) = table
        .service
        .post("/requests", Some(&table.bea), &body)
        .await?;
    assert_eq!(
        status, 200,
        "the kept request is answered, not judged again: {replayed}"
    );
    assert_eq!(replayed["id"], first["id"]);
    assert_eq!(replayed["asked_at"], first["asked_at"]);
    let mut late = Table::ask_body("4", "beta")?;
    late["ends_at"] = json!(soon);
    let fresh = table
        .service
        .post("/requests", Some(&table.bea), &late)
        .await?;
    refused(&fresh, 400, "RequestMalformed");
    Ok(())
}

#[tokio::test]
async fn the_caller_is_shown_the_grants_it_could_lend_from() -> TestResult {
    let table = Table::set().await?;
    let source = table.lendable("1").await?;
    let asked = table.ask(&table.ada, "1", "beta").await?;
    assert_eq!(
        asked["sources"],
        json!([]),
        "Ada holds no grant to lend from"
    );
    let (_, seen) = table.service.get("/requests", Some(&table.bea)).await?;
    assert_eq!(seen["requests"][0]["sources"], json!([source]));
    Ok(())
}

#[tokio::test]
async fn only_the_root_authority_is_shown_that_it_could_issue_directly() -> TestResult {
    let table = Table::set().await?;
    table.lendable("1").await?;
    let asked = table.ask(&table.bea, "1", "beta").await?;
    assert_eq!(
        asked["can_issue_root"], false,
        "Bea holds a grant she could lend from, and is not the root authority: {asked}"
    );
    let (_, seen) = table.service.get("/requests", Some(&table.ada)).await?;
    assert_eq!(seen["requests"][0]["id"], asked["id"]);
    assert_eq!(seen["requests"][0]["can_issue_root"], true);
    assert_eq!(seen["requests"][0]["sources"], json!([]));
    Ok(())
}

#[tokio::test]
async fn an_approval_the_grants_refuse_holds_nothing_and_gives_nothing() -> TestResult {
    let table = Table::set().await?;
    let source = table.lendable("1").await?;
    let wider = table.ask(&table.ada, "1", "alpha").await?;
    assert_eq!(wider["can_decide"], true, "Ada is the root authority");
    let (_, seen) = table.service.get("/requests", Some(&table.bea)).await?;
    assert_eq!(seen["requests"], json!([]), "Bea cannot give alpha: {seen}");

    let reading = table.ask(&table.ada, "1", "beta").await?;
    let (_, seen) = table.service.get("/requests", Some(&table.bea)).await?;
    assert_eq!(seen["requests"][0]["can_decide"], true);
    let mut unknown = source.clone();
    unknown.replace_range(unknown.len() - 4.., "0000");
    let first = approval(Some(&unknown))?;
    let turned = table
        .decide(&table.bea, &reading, "approve", &first)
        .await?;
    assert_ne!(turned.0, 200, "{}", turned.1);
    assert_eq!(turned.1["refusal"], "SourceUnknown", "{}", turned.1);
    let (_, seen) = table.service.get("/requests", Some(&table.bea)).await?;
    assert_eq!(seen["requests"][0]["state"], "waiting");
    assert_eq!(
        seen["requests"][0]["held_by"],
        Value::Null,
        "the grants hold nothing for the operation, so nothing is held"
    );

    let (status, declined) = table
        .decide(
            &table.ada,
            &reading,
            "decline",
            &json!({ "note": "not now" }),
        )
        .await?;
    assert_eq!(status, 200, "{declined}");
    let again = table
        .decide(&table.bea, &reading, "approve", &first)
        .await?;
    refused(&again, 409, "RequestDecided");
    let (_, held) = table.service.get("/grants", Some(&table.ada)).await?;
    assert_eq!(
        held["grants"].as_array().map(Vec::len),
        Some(1),
        "only the grant Bea lends from exists: {held}"
    );
    Ok(())
}

#[tokio::test]
async fn a_once_answer_lends_a_grant_that_admits_exactly_one_exercise() -> TestResult {
    let table = Table::set().await?;
    let source = table.lendable("1").await?;
    let asked = table.ask(&table.ada, "1", "beta").await?;
    let mut once = approval(Some(&source))?;
    once["answer"] = json!({ "kind": "once" });
    let (status, approved) = table.decide(&table.bea, &asked, "approve", &once).await?;
    assert_eq!(status, 200, "{approved}");
    let grant = approved["decision"]["grant"].as_str().ok_or("no grant")?;
    let exercise =
        json!({"route": "api", "resource": {"kind": "doc", "id": "1"}, "action": "read"});
    let first = table
        .service
        .post("/grants/check", Some(&table.ada), &exercise)
        .await?;
    assert_eq!(first.0, 200, "{}", first.1);
    assert_eq!(first.1["grant"], grant);
    let second = table
        .service
        .post("/grants/check", Some(&table.ada), &exercise)
        .await?;
    refused(&second, 403, "Revoked");
    Ok(())
}

#[tokio::test]
async fn a_for_a_while_answer_ends_when_the_approver_says_and_ongoing_has_no_end() -> TestResult {
    let table = Table::set().await?;
    let source = table.lendable("1").await?;
    for (answer, ends) in [
        (
            json!({ "kind": "until", "ends_at": FAR - 60 }),
            json!(FAR - 60),
        ),
        (json!({ "kind": "ongoing" }), Value::Null),
    ] {
        let asked = table.ask(&table.ada, "1", "beta").await?;
        let mut body = approval(Some(&source))?;
        body["answer"] = answer;
        let (status, approved) = table.decide(&table.bea, &asked, "approve", &body).await?;
        assert_eq!(status, 200, "{approved}");
        let grant = approved["decision"]["grant"].as_str().ok_or("no grant")?;
        let (_, held) = table
            .service
            .get(&format!("/grants/{grant}"), Some(&table.ada))
            .await?;
        assert_eq!(held["window"]["ends_at"], ends, "{held}");
    }
    Ok(())
}
