//! The grant API's admission: every refused request is refused by name and
//! writes nothing, and every route a permitted caller uses is exercised
//! through the server, never through button visibility. The service runs with
//! only its own disposable log and the in-process issuer, and no other server.

use std::error::Error;
use std::os::unix::fs::PermissionsExt;
use std::time::{SystemTime, UNIX_EPOCH};

use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_identity::OperationId;
use lys_identity::grants::GrantError;
use lys_identity_server::dev_seed::{Seeded, seed_configured};
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

const BEA: &str = "bea-subject";
const STRANGER: &str = "stranger-subject";

fn login(subject: &str) -> Login {
    Login {
        subject: subject.to_owned(),
        email: "shared@example.test".to_owned(),
    }
}

/// The service with Ada bound to the administrator's login, so she is the
/// root authority, and Bea, each with their agents.
async fn seeded() -> Result<(Service, Seeded), Box<dyn Error>> {
    Service::start_with(|config| Ok(seed_configured(config, [ADMINISTRATOR, BEA])?)).await
}

fn operation() -> Result<String, Box<dyn Error>> {
    Ok(OperationId::generate()?.to_string())
}

fn root_body(holder: &str, resource: &str, pass_on: &Value) -> Result<Value, Box<dyn Error>> {
    Ok(json!({
        "operation": operation()?,
        "route": "api",
        "holder": holder,
        "resource": { "kind": "doc", "id": resource },
        "relation": "alpha",
        "pass_on": pass_on,
        "window": { "starts_at": 0, "ends_at": null },
    }))
}

fn delegate_body(
    source: &str,
    recipient: &str,
    responsible: &str,
    resource: &str,
    route: &str,
) -> Result<Value, Box<dyn Error>> {
    Ok(json!({
        "operation": operation()?,
        "route": route,
        "source": source,
        "recipient": recipient,
        "responsible": responsible,
        "resource": { "kind": "doc", "id": resource },
        "relation": "beta",
        "pass_on": { "kind": "use_only" },
        "window": { "starts_at": 0, "ends_at": null },
    }))
}

fn pass_on_read_to_agents() -> Value {
    json!({ "kind": "to", "actions": ["read"], "recipients": ["agent"] })
}

async fn revision(service: &Service, cookie: &str) -> Result<u64, Box<dyn Error>> {
    let (status, body) = service.get("/grants", Some(cookie)).await?;
    assert_eq!(status, 200, "{body}");
    Ok(body["revision"].as_u64().ok_or("no revision")?)
}

fn refused(answer: &(u16, Value), status: u16, name: &str) {
    assert_eq!(answer.0, status, "{}", answer.1);
    assert_eq!(answer.1["refusal"], name, "{}", answer.1);
}

#[tokio::test]
async fn every_refused_grant_request_is_named_and_writes_nothing() -> TestResult {
    let (service, seeded) = seeded().await?;
    let (ada, bea) = (&seeded.people[0], &seeded.people[1]);
    let bea_agent = bea.agents[0].id.to_string();
    let ada_cookie = service.sign_in(login(ADMINISTRATOR)).await?;
    let bea_cookie = service.sign_in(login(BEA)).await?;
    let stranger_cookie = service.sign_in(login(STRANGER)).await?;

    let (status, issued) = service
        .post(
            "/grants/roots",
            Some(&ada_cookie),
            &root_body(&bea.id.to_string(), "1", &pass_on_read_to_agents())?,
        )
        .await?;
    assert_eq!(status, 200, "{issued}");
    let lending = issued["grant"].as_str().ok_or("no grant")?.to_owned();
    let (status, issued) = service
        .post(
            "/grants/roots",
            Some(&ada_cookie),
            &root_body(&bea.id.to_string(), "2", &json!({ "kind": "use_only" }))?,
        )
        .await?;
    assert_eq!(status, 200, "{issued}");
    let use_only = issued["grant"].as_str().ok_or("no grant")?.to_owned();
    let before = revision(&service, &ada_cookie).await?;

    let mut refusals = 0;
    for (answer, status, name) in [
        (
            service
                .post("/grants/roots", None, &root_body(&bea.id.to_string(), "3", &json!({ "kind": "use_only" }))?)
                .await?,
            401,
            "NotSignedIn",
        ),
        (
            service
                .post(
                    "/grants",
                    Some(&stranger_cookie),
                    &delegate_body(&lending, &bea_agent, &bea.id.to_string(), "1", "api")?,
                )
                .await?,
            403,
            "NoPerson",
        ),
        (
            service
                .post(
                    "/grants/roots",
                    Some(&bea_cookie),
                    &root_body(&bea.id.to_string(), "3", &json!({ "kind": "use_only" }))?,
                )
                .await?,
            403,
            "RootAuthorityRefused",
        ),
        (
            service
                .post(
                    "/grants",
                    Some(&bea_cookie),
                    &delegate_body(&use_only, &bea_agent, &bea.id.to_string(), "2", "tool")?,
                )
                .await?,
            403,
            "UseOnly",
        ),
        (
            service
                .post(
                    &format!("/grants/{lending}/revoke"),
                    Some(&bea_cookie),
                    &json!({ "operation": operation()?, "route": "browser", "reason": "no longer needed" }),
                )
                .await?,
            403,
            "RevokeRefused",
        ),
    ] {
        refused(&(answer.0, answer.1), status, name);
        refusals += 1;
        assert_eq!(revision(&service, &ada_cookie).await?, before, "{name} wrote");
    }
    assert_eq!(refusals, 5);
    assert_ne!(ada.id, bea.id);
    Ok(())
}

#[tokio::test]
async fn the_same_request_is_permitted_through_the_api_the_tool_and_the_browser() -> TestResult {
    let (service, seeded) = seeded().await?;
    let bea = &seeded.people[1];
    let bea_agent = bea.agents[0].id.to_string();
    let ada_cookie = service.sign_in(login(ADMINISTRATOR)).await?;
    let bea_cookie = service.sign_in(login(BEA)).await?;
    let (status, issued) = service
        .post(
            "/grants/roots",
            Some(&ada_cookie),
            &root_body(&bea.id.to_string(), "1", &pass_on_read_to_agents())?,
        )
        .await?;
    assert_eq!(status, 200, "{issued}");
    let lending = issued["grant"].as_str().ok_or("no grant")?.to_owned();

    let mut routes = 0;
    for route in ["api", "tool", "browser"] {
        let before = revision(&service, &ada_cookie).await?;
        let (status, passed) = service
            .post(
                "/grants",
                Some(&bea_cookie),
                &delegate_body(&lending, &bea_agent, &bea.id.to_string(), "1", route)?,
            )
            .await?;
        assert_eq!(status, 200, "{route}: {passed}");
        let passed_on = passed["grant"].as_str().ok_or("no grant")?;
        assert_eq!(
            revision(&service, &ada_cookie).await?,
            before + 1,
            "{route}"
        );
        let (status, read) = service
            .get(&format!("/grants/{passed_on}"), Some(&bea_cookie))
            .await?;
        assert_eq!(status, 200, "{read}");
        assert_eq!(read["holder"], bea_agent.as_str());
        assert_eq!(read["source"], lending.as_str());
        let (status, revoked) = service
            .post(
                &format!("/grants/{passed_on}/revoke"),
                Some(&bea_cookie),
                &json!({ "operation": operation()?, "route": route, "reason": "the test is done with it" }),
            )
            .await?;
        assert_eq!(status, 200, "{route}: {revoked}");
        routes += 1;
    }
    assert_eq!(routes, 3);
    Ok(())
}

#[tokio::test]
async fn a_check_records_its_use_and_an_explanation_records_nothing() -> TestResult {
    let (service, seeded) = seeded().await?;
    let bea = &seeded.people[1];
    let ada_cookie = service.sign_in(login(ADMINISTRATOR)).await?;
    let bea_cookie = service.sign_in(login(BEA)).await?;
    let (status, issued) = service
        .post(
            "/grants/roots",
            Some(&ada_cookie),
            &root_body(&bea.id.to_string(), "1", &json!({ "kind": "use_only" }))?,
        )
        .await?;
    assert_eq!(status, 200, "{issued}");
    let grant = issued["grant"].as_str().ok_or("no grant")?.to_owned();
    let question =
        json!({ "route": "tool", "resource": { "kind": "doc", "id": "1" }, "action": "read" });

    let (_, read) = service
        .get(&format!("/grants/{grant}"), Some(&bea_cookie))
        .await?;
    assert_eq!(
        read["last_use"],
        json!({ "seen": false, "recorded": 0, "source": "reported" }),
        "{read}"
    );

    let before = revision(&service, &ada_cookie).await?;
    for path in ["/grants/why", "/grants/who"] {
        let mut body = question.clone();
        if path == "/grants/who" {
            body["page_size"] = json!(10);
            body["after"] = Value::Null;
        }
        let (status, answer) = service.post(path, Some(&bea_cookie), &body).await?;
        assert_eq!(status, 200, "{path}: {answer}");
        assert!(
            answer.get("use_event").is_none(),
            "{path} records no use: {answer}"
        );
    }
    assert_eq!(
        revision(&service, &ada_cookie).await?,
        before,
        "an explanation wrote"
    );
    let (_, read) = service
        .get(&format!("/grants/{grant}"), Some(&bea_cookie))
        .await?;
    assert_eq!(
        read["last_use"],
        json!({ "seen": false, "recorded": 0, "source": "reported" }),
        "{read}"
    );

    let (status, checked) = service
        .post("/grants/check", Some(&bea_cookie), &question)
        .await?;
    assert_eq!(status, 200, "{checked}");
    assert_eq!(checked["use_event"]["recorded"], true, "{checked}");
    let index = checked["use_event"]["index"].clone();
    assert_eq!(
        revision(&service, &ada_cookie).await?,
        before + 1,
        "one use event"
    );
    let (_, read) = service
        .get(&format!("/grants/{grant}"), Some(&bea_cookie))
        .await?;
    assert_eq!(read["last_use"]["seen"], true, "{read}");
    assert_eq!(read["last_use"]["route"], "tool", "{read}");
    assert_eq!(read["last_use"]["use_event"], index, "{read}");
    assert_eq!(read["last_use"]["recorded"], 1, "{read}");
    assert_eq!(read["last_use"]["source"], "reported", "{read}");

    let (status, refused) = service
        .post(
            "/grants/check",
            Some(&bea_cookie),
            &json!({ "route": "api", "resource": { "kind": "doc", "id": "2" }, "action": "read" }),
        )
        .await?;
    assert_eq!(status, 403, "{refused}");
    assert_eq!(refused["refusal"], "NotHeld", "{refused}");
    assert_eq!(
        revision(&service, &ada_cookie).await?,
        before + 1,
        "a refused check wrote"
    );
    Ok(())
}

#[tokio::test]
async fn the_permission_model_is_served_to_a_signed_in_caller_only() -> TestResult {
    let (service, _) = seeded().await?;
    let bea = service.sign_in(login(BEA)).await?;
    let (status, model) = service.get("/grants/model", Some(&bea)).await?;
    assert_eq!(status, 200, "{model}");
    assert_eq!(
        model,
        json!({ "version": 1, "relations": { "alpha": ["read", "write"], "beta": ["read"] } })
    );
    let (status, _) = service.get("/grants/model", None).await?;
    assert_eq!(status, 401);
    Ok(())
}

#[tokio::test]
async fn a_revoked_grant_names_when_and_at_which_revision_it_was_revoked() -> TestResult {
    let (service, seeded) = seeded().await?;
    let ada = service.sign_in(login(ADMINISTRATOR)).await?;
    let bea = seeded.people[1].id.to_string();
    let (status, issued) = service
        .post(
            "/grants/roots",
            Some(&ada),
            &root_body(&bea, "1", &json!({ "kind": "use_only" }))?,
        )
        .await?;
    assert_eq!(status, 200, "{issued}");
    let grant = issued["grant"].as_str().ok_or("no grant")?.to_owned();
    let (_, standing) = service.get(&format!("/grants/{grant}"), Some(&ada)).await?;
    assert_eq!(standing["revoked_at"], Value::Null, "{standing}");
    assert_eq!(standing["revoked_revision"], Value::Null, "{standing}");

    let body = json!({ "operation": operation()?, "route": "api", "reason": "finished" });
    let (status, revoked) = service
        .post(&format!("/grants/{grant}/revoke"), Some(&ada), &body)
        .await?;
    assert_eq!(status, 200, "{revoked}");
    let after = revision(&service, &ada).await?;
    let (_, read) = service.get(&format!("/grants/{grant}"), Some(&ada)).await?;
    assert_eq!(read["revoked"], true, "{read}");
    assert!(read["revoked_at"].is_u64(), "{read}");
    assert_eq!(read["revoked_revision"], json!(after), "{read}");
    Ok(())
}

fn leaves_writable(service: &Service, writable: bool) -> TestResult {
    let mode = if writable { 0o755 } else { 0o555 };
    std::fs::set_permissions(
        service.dir.path().join("grant-log").join("leaves"),
        std::fs::Permissions::from_mode(mode),
    )?;
    Ok(())
}

/// `GRANT_LAST_USED` at the service: a zero count is answered as reported, a
/// permitted exercise whose use event the grant log refused is answered as a
/// missing report, and a restart answers the recorded use and count from the
/// log.
#[tokio::test]
async fn a_missing_use_report_is_named_and_the_recorded_use_survives_a_restart() -> TestResult {
    let (mut service, seeded) = seeded().await?;
    let bea = seeded.people[1].id.to_string();
    let ada = service.sign_in(login(ADMINISTRATOR)).await?;
    let bea_cookie = service.sign_in(login(BEA)).await?;
    let (status, issued) = service
        .post(
            "/grants/roots",
            Some(&ada),
            &root_body(&bea, "1", &json!({ "kind": "use_only" }))?,
        )
        .await?;
    assert_eq!(status, 200, "{issued}");
    let grant = issued["grant"].as_str().ok_or("no grant")?.to_owned();
    let path = format!("/grants/{grant}");
    let question =
        json!({ "route": "api", "resource": { "kind": "doc", "id": "1" }, "action": "read" });

    leaves_writable(&service, false)?;
    let before = revision(&service, &ada).await?;
    let (status, checked) = service
        .post("/grants/check", Some(&bea_cookie), &question)
        .await?;
    leaves_writable(&service, true)?;
    assert_eq!(status, 200, "{checked}");
    assert_eq!(checked["use_event"]["recorded"], false, "{checked}");
    assert_eq!(
        revision(&service, &ada).await?,
        before,
        "nothing was recorded"
    );
    let (_, read) = service.get(&path, Some(&bea_cookie)).await?;
    let missing = &read["last_use"];
    assert_eq!(missing["seen"], false, "{read}");
    assert_eq!(missing["recorded"], 0, "{read}");
    assert_eq!(
        missing["source"], "missing",
        "a missing report is not a zero count: {read}"
    );
    assert_eq!(missing["unreported"]["count"], 1, "{read}");
    assert_eq!(missing["unreported"]["route"], "api", "{read}");
    assert_eq!(
        missing["unreported"]["reason"], checked["use_event"]["reason"],
        "{read}"
    );
    let (_, listed) = service.get("/grants", Some(&bea_cookie)).await?;
    let in_list = listed["grants"]
        .as_array()
        .ok_or("no grants")?
        .iter()
        .find(|held| held["id"] == grant.as_str())
        .ok_or("the grant is not listed")?;
    assert_eq!(
        in_list["last_use"], read["last_use"],
        "the list and the read agree"
    );

    let tool =
        json!({ "route": "tool", "resource": { "kind": "doc", "id": "1" }, "action": "read" });
    let (status, checked) = service
        .post("/grants/check", Some(&bea_cookie), &tool)
        .await?;
    assert_eq!(status, 200, "{checked}");
    let index = checked["use_event"]["index"].clone();
    assert!(index.is_u64(), "{checked}");
    let (_, read) = service.get(&path, Some(&bea_cookie)).await?;
    assert_eq!(read["last_use"]["seen"], true, "{read}");
    assert_eq!(read["last_use"]["recorded"], 1, "{read}");
    assert_eq!(
        read["last_use"]["source"], "missing",
        "a later recorded use does not make the count whole: {read}"
    );
    let seen_at = read["last_use"]["at"].clone();

    service.restart().await?;
    let bea_cookie = service.sign_in(login(BEA)).await?;
    let (status, read) = service.get(&path, Some(&bea_cookie)).await?;
    assert_eq!(status, 200, "{read}");
    assert_eq!(
        read["last_use"],
        json!({
            "seen": true,
            "at": seen_at,
            "route": "tool",
            "use_event": index,
            "recorded": 1,
            "source": "reported",
        }),
        "the recorded attribution and count survive the restart; the missing report was in no log"
    );
    Ok(())
}

/// Seconds since the epoch, as the service's clock reads them.
fn clock() -> Result<u64, Box<dyn Error>> {
    Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs())
}

/// A root grant of `alpha` on doc `resource` to `holder`, ending at `ends`.
fn root_ending(
    holder: &str,
    resource: &str,
    pass_on: &Value,
    ends: Option<u64>,
) -> Result<Value, Box<dyn Error>> {
    let mut body = root_body(holder, resource, pass_on)?;
    body["window"]["ends_at"] = json!(ends);
    Ok(body)
}

/// A grant of `beta` on doc `resource` passed on from `source` to a
/// recipient and the person answering for it, ending at `ends`.
fn passed_on(
    source: &str,
    (recipient, responsible): (&str, &str),
    resource: &str,
    pass_on: &Value,
    ends: Option<u64>,
) -> Result<Value, Box<dyn Error>> {
    let mut body = delegate_body(source, recipient, responsible, resource, "api")?;
    body["pass_on"] = pass_on.clone();
    body["window"]["ends_at"] = json!(ends);
    Ok(body)
}

/// POST `body` to `path` as `cookie`, which must answer 200, and the grant it names.
async fn issued(
    service: &Service,
    path: &str,
    cookie: &str,
    body: &Value,
) -> Result<String, Box<dyn Error>> {
    let (status, answer) = service.post(path, Some(cookie), body).await?;
    assert_eq!(status, 200, "{path}: {answer}");
    Ok(answer["grant"].as_str().ok_or("no grant")?.to_owned())
}

/// The grant `id` as GET /grants answers it to `cookie`.
async fn listed(service: &Service, cookie: &str, id: &str) -> Result<Value, Box<dyn Error>> {
    let (status, body) = service.get("/grants", Some(cookie)).await?;
    assert_eq!(status, 200, "{body}");
    Ok(body["grants"]
        .as_array()
        .ok_or("no grants")?
        .iter()
        .find(|grant| grant["id"] == id)
        .ok_or("the grant is not listed")?
        .clone())
}

/// The grant `id` as GET /grants/{id} answers it to `cookie`.
async fn read_one(service: &Service, cookie: &str, id: &str) -> Result<Value, Box<dyn Error>> {
    let (status, body) = service.get(&format!("/grants/{id}"), Some(cookie)).await?;
    assert_eq!(status, 200, "{body}");
    Ok(body)
}

#[tokio::test]
async fn a_derived_grant_whose_source_is_revoked_reads_void_naming_the_revoked_source() -> TestResult
{
    let (service, seeded) = seeded().await?;
    let bea = &seeded.people[1];
    let bea_agent = bea.agents[0].id.to_string();
    let ada_cookie = service.sign_in(login(ADMINISTRATOR)).await?;
    let bea_cookie = service.sign_in(login(BEA)).await?;
    let root = issued(
        &service,
        "/grants/roots",
        &ada_cookie,
        &root_body(&bea.id.to_string(), "1", &pass_on_read_to_agents())?,
    )
    .await?;
    let derived = issued(
        &service,
        "/grants",
        &bea_cookie,
        &delegate_body(&root, &bea_agent, &bea.id.to_string(), "1", "api")?,
    )
    .await?;
    let body = json!({ "operation": operation()?, "route": "api", "reason": "finished" });
    let (status, revoked) = service
        .post(&format!("/grants/{root}/revoke"), Some(&ada_cookie), &body)
        .await?;
    assert_eq!(status, 200, "{revoked}");

    let read = listed(&service, &bea_cookie, &derived).await?;
    let grant = root.clone();
    let reason = GrantError::Revoked { grant }.to_string();
    assert_eq!(
        read["standing"],
        json!({ "stands": false, "refusal": "Revoked", "grant": root, "reason": reason }),
        "{read}"
    );
    assert_eq!(read["revoked"], false, "{read}");
    Ok(())
}

#[tokio::test]
async fn a_delegation_ending_after_its_ancestor_is_refused_and_effective_end_is_the_grants_own()
-> TestResult {
    let (service, seeded) = seeded().await?;
    let bea = &seeded.people[1];
    let bea_id = bea.id.to_string();
    let bea_agent = bea.agents[0].id.to_string();
    let to_agent = (bea_agent.as_str(), bea_id.as_str());
    let ada_cookie = service.sign_in(login(ADMINISTRATOR)).await?;
    let bea_cookie = service.sign_in(login(BEA)).await?;
    let use_only = json!({ "kind": "use_only" });
    let t = clock()?;
    let root = issued(
        &service,
        "/grants/roots",
        &ada_cookie,
        &root_ending(&bea_id, "1", &pass_on_read_to_agents(), Some(t + 1000))?,
    )
    .await?;

    let before = revision(&service, &ada_cookie).await?;
    let (_, refusal) = service
        .post(
            "/grants",
            Some(&bea_cookie),
            &passed_on(&root, to_agent, "1", &use_only, Some(t + 1001))?,
        )
        .await?;
    assert_eq!(refusal["refusal"], "ExpiryBeyondSource", "{refusal}");
    assert_eq!(
        revision(&service, &ada_cookie).await?,
        before,
        "a refused grant was written"
    );

    let derived = issued(
        &service,
        "/grants",
        &bea_cookie,
        &passed_on(&root, to_agent, "1", &use_only, Some(t + 500))?,
    )
    .await?;
    let mut reads = 0;
    for read in [
        listed(&service, &bea_cookie, &derived).await?,
        read_one(&service, &bea_cookie, &derived).await?,
    ] {
        let own = &read["window"]["ends_at"];
        assert_eq!(read["effective_ends_at"], json!(t + 500), "{read}");
        assert_eq!(&read["effective_ends_at"], own, "{read}");
        assert_eq!(read["standing"], json!({ "stands": true }), "{read}");
        reads += 1;
    }
    for read in [
        listed(&service, &bea_cookie, &root).await?,
        read_one(&service, &bea_cookie, &root).await?,
    ] {
        assert_eq!(read["effective_ends_at"], json!(t + 1000), "{read}");
        assert_eq!(read["standing"], json!({ "stands": true }), "{read}");
        reads += 1;
    }

    let endless = issued(
        &service,
        "/grants/roots",
        &ada_cookie,
        &root_ending(&bea_id, "2", &pass_on_read_to_agents(), None)?,
    )
    .await?;
    let endless_derived = issued(
        &service,
        "/grants",
        &bea_cookie,
        &passed_on(&endless, to_agent, "2", &use_only, None)?,
    )
    .await?;
    for id in [&endless, &endless_derived] {
        let read = listed(&service, &bea_cookie, id).await?;
        assert_eq!(read["effective_ends_at"], Value::Null, "{read}");
        reads += 1;
    }
    assert_eq!(reads, 6);
    Ok(())
}

const CORA: (&str, &str) = ("Cora (test person)", "cora-subject");
const DOV: (&str, &str) = ("Dov (test person)", "dov-subject");

/// Move `identity` through `transition`, as the administrator.
async fn moved(service: &Service, admin: &str, identity: &str, transition: &str) -> TestResult {
    let body = json!({ "operation": operation()?, "transition": transition, "reason": "test" });
    let path = format!("/identities/{identity}/transitions");
    let (status, answer) = service.post(&path, Some(admin), &body).await?;
    assert_eq!(status, 200, "{answer}");
    Ok(())
}

/// Register a person named `name` through POST /people, bind a login of
/// `subject` to them and activate them, as the administrator.
async fn active_person(
    service: &Service,
    admin: &str,
    (name, subject): (&str, &str),
) -> Result<String, Box<dyn Error>> {
    let body = json!({ "operation": operation()?, "display_name": name });
    let (status, answer) = service.post("/people", Some(admin), &body).await?;
    assert_eq!(status, 200, "{answer}");
    let person = answer["person"].as_str().ok_or("no person")?.to_owned();
    let body = json!({
        "operation": operation()?,
        "issuer": service.issuer.issuer(),
        "subject": subject,
    });
    let path = format!("/people/{person}/logins");
    let (status, answer) = service.post(&path, Some(admin), &body).await?;
    assert_eq!(status, 200, "{answer}");
    moved(service, admin, &person, "activate").await?;
    Ok(person)
}

#[tokio::test]
async fn a_derived_grant_whose_chain_names_an_identity_the_caller_may_not_see_reads_withheld()
-> TestResult {
    let (service, seeded) = seeded().await?;
    let bea_id = seeded.people[1].id.to_string();
    let ada = service.sign_in(login(ADMINISTRATOR)).await?;
    let first = active_person(&service, &ada, CORA).await?;
    let second = active_person(&service, &ada, DOV).await?;
    let first_cookie = service.sign_in(login(CORA.1)).await?;
    let second_cookie = service.sign_in(login(DOV.1)).await?;
    let bea_cookie = service.sign_in(login(BEA)).await?;
    let to_persons = json!({ "kind": "to", "actions": ["read"], "recipients": ["person"] });
    let use_only = json!({ "kind": "use_only" });
    let to_second = (second.as_str(), second.as_str());
    let to_bea = (bea_id.as_str(), bea_id.as_str());

    let root = issued(
        &service,
        "/grants/roots",
        &ada,
        &root_body(&first, "1", &to_persons)?,
    )
    .await?;
    let middle = issued(
        &service,
        "/grants",
        &first_cookie,
        &passed_on(&root, to_second, "1", &to_persons, None)?,
    )
    .await?;
    let beas = issued(
        &service,
        "/grants",
        &second_cookie,
        &passed_on(&middle, to_bea, "1", &use_only, None)?,
    )
    .await?;
    moved(&service, &ada, &first, "suspend").await?;

    let (status, body) = service.get("/grants", Some(&bea_cookie)).await?;
    assert_eq!(status, 200, "{body}");
    let read = listed(&service, &bea_cookie, &beas).await?;
    assert_eq!(
        read["standing"],
        json!({
            "stands": false,
            "refusal": "IdentityNotActive",
            "grant": null,
            "reason": "IdentityNotActive: the refusal names a grant or identity the caller may not inspect",
        }),
        "{read}"
    );
    let text = body.to_string();
    assert_eq!(text.matches(first.as_str()).count(), 0, "{text}");
    assert_eq!(text.matches(root.as_str()).count(), 0, "{text}");
    Ok(())
}
