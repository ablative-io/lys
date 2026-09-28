#![cfg(test)]
//! R3: every grant and permission check the identity server makes is one
//! `CheckPermission` call through the one evaluator, at least as fresh as
//! the projection; an engine that cannot answer refuses by name; an expired
//! grant is refused by the caveat's time from the evaluator's clock; an
//! answer missing that context is refused; and the administrator's admission
//! never asks `SpiceDB`.

mod spicedb_support;

use std::error::Error;
use std::os::unix::fs::PermissionsExt;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use identity_contract::fake_issuer::{CLIENT_ID, CLIENT_SECRET, FakeIssuer, Login};
use identity_contract::harness::{
    ADMINISTRATOR, GRANT_MODEL, GRANT_ORIGIN, LINK_AUDIT_SOURCE, ORIGIN,
};
use lys_identity::grants::{ExerciseRequest, GrantError, Question, Route};
use lys_identity::{IdentityId, OperationId};
use lys_identity_server::config::ConfiguredLogin;
use lys_identity_server::dev_seed::seed_configured;
use lys_identity_server::spicedb::freshness::Fresh;
use lys_identity_server::spicedb::schema::ensure;
use lys_identity_server::spicedb::wire::authzed::api::v1::check_permission_response::Permissionship;
use lys_identity_server::spicedb::{Clock, Engine, Evaluator};
use lys_identity_server::{Config, service_engaged};
use serde_json::{Value, json};
use spicedb_support::server::SpiceDb;
use spicedb_support::{
    Counting, T0, TestResult, World, checks, consistency, last_written, project, read, stored,
};

/// The end of grant G in the expiry fixture.
const E: u64 = T0 + 1_000;

/// A clock the test sets.
struct Clockwork(AtomicU64);

impl Clock for Clockwork {
    fn now(&self) -> u64 {
        self.0.load(Ordering::SeqCst)
    }
}

/// The identity server on a local port, signing in through the fake
/// issuer, with its step-2 engine over the test's counting client.
struct Served {
    base: String,
    issuer: FakeIssuer,
    http: reqwest::Client,
}

fn secret(path: &std::path::Path, bytes: &[u8]) -> TestResult {
    std::fs::write(path, bytes)?;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    Ok(())
}

fn location(answer: &reqwest::Response) -> Result<String, Box<dyn Error>> {
    Ok(answer
        .headers()
        .get(reqwest::header::LOCATION)
        .ok_or("no redirect")?
        .to_str()?
        .to_owned())
}

/// Start the service with Ada bound to the administrator's login, so she is
/// the root authority, and Bea, over the test's `client`.
async fn serve(
    client: Arc<Counting>,
) -> Result<
    (
        Served,
        lys_identity_server::dev_seed::Seeded,
        tempfile::TempDir,
    ),
    Box<dyn Error>,
> {
    let dir = tempfile::TempDir::new()?;
    secret(&dir.path().join("issuer.key"), &[3; 32])?;
    secret(&dir.path().join("service.key"), &[9; 32])?;
    secret(&dir.path().join("client.secret"), CLIENT_SECRET.as_bytes())?;
    let issuer = FakeIssuer::start(&dir.path().join("issuer.key")).await?;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let listen = listener.local_addr()?;
    let base = format!("http://{listen}");
    let configured = |subject: &str| ConfiguredLogin {
        issuer: issuer.issuer().to_owned(),
        subject: subject.to_owned(),
    };
    let config = Config {
        listen,
        log_dir: dir.path().join("log"),
        log_origin: ORIGIN.to_owned(),
        event_key_file: dir.path().join("service.key"),
        issuer: issuer.issuer().to_owned(),
        client_id: CLIENT_ID.to_owned(),
        client_secret_file: dir.path().join("client.secret"),
        redirect_url: format!("{base}/callback"),
        administrator: configured(ADMINISTRATOR),
        link_audit_source: configured(LINK_AUDIT_SOURCE),
        session_seconds: 600,
        secure_cookie: false,
        grant_log_dir: dir.path().join("grant-log"),
        grant_log_origin: GRANT_ORIGIN.to_owned(),
        grant_model_file: dir.path().join("grant-model.json"),
        spicedb: None,
        secrets: None,
        requests_dir: None,
        certificates_dir: None,
        network_file: None,
        roles_file: None,
        provisioning_file: None,
        homes_dir: None,
        runtime_dir: None,
        service_accounts_dir: None,
        teams_dir: None,
        stops_dir: None,
        reviews_dir: None,
        sign_in_providers: None,
        surface_dir: None,
    };
    std::fs::write(&config.grant_model_file, GRANT_MODEL)?;
    config.validate()?;
    let seeded = seed_configured(&config, [ADMINISTRATOR, "bea-subject"])?;
    let clock: Arc<dyn Clock> = Arc::new(lys_identity_server::spicedb::ServiceClock);
    let (engine, _) = Engine::engage(client, clock, 1000, &config.grant_log_dir)?;
    let app = service_engaged(&config, Arc::new(|_| {}), Some(engine)).await?;
    tokio::spawn(async move { axum::serve(listener, app).await });
    let http = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    Ok((Served { base, issuer, http }, seeded, dir))
}

impl Served {
    async fn sign_in(&self, subject: &str) -> Result<String, Box<dyn Error>> {
        self.issuer.sign_in_as(Login {
            subject: subject.to_owned(),
            email: "shared@example.test".to_owned(),
        });
        let to_issuer = self.http.get(format!("{}/login", self.base)).send().await?;
        let authorize = location(&to_issuer)?;
        let back = self.http.get(authorize).send().await?;
        let callback = location(&back)?;
        let signed_in = self.http.get(callback).send().await?;
        let cookie = signed_in
            .headers()
            .get(reqwest::header::SET_COOKIE)
            .ok_or("sign-in set no session")?
            .to_str()?
            .split(';')
            .next()
            .ok_or("the session cookie is empty")?
            .to_owned();
        Ok(cookie)
    }

    async fn post(
        &self,
        path: &str,
        cookie: &str,
        body: &Value,
    ) -> Result<(u16, Value), Box<dyn Error>> {
        let answer = self
            .http
            .post(format!("{}{path}", self.base))
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .header(reqwest::header::COOKIE, cookie)
            .body(body.to_string())
            .send()
            .await?;
        let status = answer.status().as_u16();
        Ok((status, serde_json::from_str(&answer.text().await?)?))
    }
}

#[tokio::test]
async fn six_requests_on_three_routes_are_six_checks_each_answered_as_spicedb_answered()
-> TestResult {
    let spicedb = SpiceDb::start(None)?;
    let client = spicedb.client()?;
    let (served, seeded, dir) = serve(Arc::clone(&client)).await?;
    let ada = served.sign_in(ADMINISTRATOR).await?;
    let root = json!({
        "operation": OperationId::generate()?.to_string(), "route": "api",
        "holder": seeded.people[0].id.to_string(),
        "resource": { "kind": "doc", "id": "1" }, "relation": "alpha",
        "pass_on": { "kind": "use_only" }, "window": { "starts_at": 0, "ends_at": null },
    });
    let (status, issued) = served.post("/grants/roots", &ada, &root).await?;
    assert_eq!(status, 200, "{issued}");
    let before = checks(&client).len();
    let mut answers = Vec::new();
    for route in ["browser", "api", "tool"] {
        for doc in ["1", "2"] {
            let question = json!({
                "route": route, "resource": { "kind": "doc", "id": doc }, "action": "read",
            });
            answers.push((doc, served.post("/grants/check", &ada, &question).await?));
        }
    }
    let asked = checks(&client)[before..].to_vec();
    assert_eq!(asked.len(), 6);
    let token = last_written(&client).ok_or("the projector wrote nothing")?;
    for ((doc, (status, body)), (question, answer)) in answers.iter().zip(&asked) {
        let permissionship = answer
            .as_ref()
            .map(|answer| answer.permissionship)
            .ok_or("a check went unanswered")?;
        let permitted = permissionship == i32::from(Permissionship::HasPermission);
        assert_eq!(permitted, *status == 200, "{doc}: {body}");
        assert_eq!(permitted, *doc == "1", "{doc}: {body}");
        if !permitted {
            assert_eq!(body["refusal"], "no_grant", "{body}");
        }
        assert_eq!(
            consistency(question.consistency.as_ref()),
            format!("at_least_as_fresh {token}")
        );
    }
    let loose = asked
        .iter()
        .filter(|(question, _)| consistency(question.consistency.as_ref()) == "minimize_latency")
        .count();
    assert_eq!(loose, 0);
    drop(dir);
    Ok(())
}

#[tokio::test]
async fn the_administrators_mutation_is_admitted_without_asking_spicedb() -> TestResult {
    let spicedb = SpiceDb::start(None)?;
    let client = spicedb.client()?;
    let (served, _, dir) = serve(Arc::clone(&client)).await?;
    assert!(stored(client.as_ref())?.is_empty());
    let ada = served.sign_in(ADMINISTRATOR).await?;
    let before = checks(&client).len();
    let person = json!({
        "operation": OperationId::generate()?.to_string(),
        "display_name": "Cleo (test person)",
    });
    let (status, body) = served.post("/people", &ada, &person).await?;
    assert_eq!(status, 200, "{body}");
    assert_eq!(checks(&client).len() - before, 0);
    drop(dir);
    Ok(())
}

/// P holds a root grant on X ending after `E` plus one second, and A holds
/// grant G of read on X from P ending at `E`, projected into `client`'s store.
fn expiring(client: &Arc<Counting>) -> Result<(World, IdentityId), Box<dyn Error>> {
    let mut world = World::new()?;
    let on_x = project("x")?;
    let person = world.person("P")?;
    let agent = IdentityId::Agent(world.agent(person, "A")?);
    let root = world.root_grant(person, &on_x, Some(E + 100))?;
    world.delegate(
        root.event.grant(),
        (IdentityId::Person(person), agent),
        &on_x,
        Some(E),
    )??;
    ensure(client.as_ref())?;
    let mut projector = world.projector(client, 1000)?;
    projector.catch_up(world.book(), world.grants.revision())?;
    Ok((world, agent))
}

/// The `now` a check carried as its caveat's context, written whole.
fn carried_now(client: &Counting) -> Result<String, Box<dyn Error>> {
    let (question, _) = checks(client).pop().ok_or("no check was made")?;
    let now = question
        .context
        .as_ref()
        .and_then(|context| context.fields.get("now"))
        .and_then(|value| value.kind.clone())
        .ok_or("the check carried no time")?;
    match now {
        prost_types::value::Kind::NumberValue(now) => Ok(format!("{now:.0}")),
        other => Err(format!("the check carried {other:?} as its time").into()),
    }
}

#[test]
fn an_expired_grant_is_refused_by_the_clocks_time_through_both_checks() -> TestResult {
    let spicedb = SpiceDb::start(None)?;
    let client = spicedb.client()?;
    let (world, agent) = expiring(&client)?;
    let token = last_written(&client);
    let clock = Arc::new(Clockwork(AtomicU64::new(E - 1)));
    let asking: Arc<Counting> = Arc::clone(&client);
    let ticking: Arc<Clockwork> = Arc::clone(&clock);
    let evaluator = Evaluator::new(asking, ticking);
    let (on_x, action) = (project("x")?, read()?);
    let question = Question {
        subject: agent,
        resource: &on_x,
        action: &action,
    };
    for (at, expected) in [(E - 1, true), (E, false), (E + 1, false)] {
        clock.0.store(at, Ordering::SeqCst);
        let before = checks(&client).len();
        let plain = evaluator.check(&question, token.as_deref())?;
        assert_eq!(plain.permitted, expected, "plain check at {at}");
        assert_eq!(checks(&client).len() - before, 1);
        assert_eq!(carried_now(&client)?, at.to_string());
        let before = checks(&client).len();
        let traced = evaluator.check_traced(&question, token.as_deref(), None)?;
        assert_eq!(traced.permitted, expected, "traced check at {at}");
        assert_eq!(checks(&client).len() - before, 1);
        assert_eq!(carried_now(&client)?, at.to_string());
    }
    assert_eq!(world.now, T0 + 10);
    Ok(())
}

#[test]
fn an_answer_missing_the_caveats_context_is_refused_and_never_admitted() -> TestResult {
    let spicedb = SpiceDb::start(None)?;
    let client = spicedb.client()?;
    let (mut world, agent) = expiring(&client)?;
    let clock: Arc<dyn Clock> = Arc::new(Clockwork(AtomicU64::new(E - 1)));
    let asking: Arc<Counting> = Arc::clone(&client);
    let evaluator = Evaluator::new(asking, clock);
    client.strip_context.store(true, Ordering::SeqCst);
    let request = ExerciseRequest {
        caller: agent,
        route: Route::Api,
        resource: project("x")?,
        action: read()?,
    };
    let events = world.grants.revision();
    let reached = lys_identity_server::spicedb::Reached {
        position: events,
        token: last_written(&client),
    };
    let decided = world.check_with(&request, &Fresh::new(&evaluator, &reached), E - 1)?;
    assert!(
        matches!(decided, Err(GrantError::PermissionConditional { .. })),
        "{decided:?}"
    );
    let words = decided
        .err()
        .map(|refusal| refusal.to_string())
        .unwrap_or_default();
    assert!(words.starts_with("permission_conditional"), "{words}");
    let (_, answer) = checks(&client).pop().ok_or("no check was made")?;
    assert_eq!(
        answer.map(|answer| answer.permissionship),
        Some(i32::from(Permissionship::ConditionalPermission))
    );
    assert_eq!(world.grants.revision() - events, 0, "a use was recorded");
    Ok(())
}

#[test]
fn a_permitted_request_is_refused_by_name_once_spicedb_is_stopped() -> TestResult {
    let spicedb = SpiceDb::start(None)?;
    let client = spicedb.client()?;
    let (mut world, agent) = expiring(&client)?;
    let clock: Arc<dyn Clock> = Arc::new(Clockwork(AtomicU64::new(E - 1)));
    let asking: Arc<Counting> = Arc::clone(&client);
    let evaluator = Evaluator::new(asking, clock);
    let request = ExerciseRequest {
        caller: agent,
        route: Route::Tool,
        resource: project("x")?,
        action: read()?,
    };
    let reached = lys_identity_server::spicedb::Reached {
        position: world.grants.revision(),
        token: last_written(&client),
    };
    let permitted = world.check_with(&request, &Fresh::new(&evaluator, &reached), E - 1)??;
    assert!(permitted.use_event.is_some());
    drop(spicedb);
    let stopped = world.check_with(&request, &Fresh::new(&evaluator, &reached), E - 1)?;
    let admitted = usize::from(stopped.is_ok());
    assert_eq!(admitted, 0, "{stopped:?}");
    let words = stopped
        .err()
        .map(|refusal| refusal.to_string())
        .unwrap_or_default();
    assert!(
        words.starts_with("permission_engine_unavailable"),
        "{words}"
    );
    Ok(())
}

#[test]
fn a_suspended_identity_is_refused_at_its_next_check_as_the_directory_rules() -> TestResult {
    let spicedb = SpiceDb::start(None)?;
    let client = spicedb.client()?;
    let (mut world, agent) = expiring(&client)?;
    let clock: Arc<dyn Clock> = Arc::new(Clockwork(AtomicU64::new(E - 1)));
    let asking: Arc<Counting> = Arc::clone(&client);
    let evaluator = Evaluator::new(asking, clock);
    let request = ExerciseRequest {
        caller: agent,
        route: Route::Api,
        resource: project("x")?,
        action: read()?,
    };
    let reached = lys_identity_server::spicedb::Reached {
        position: world.grants.revision(),
        token: last_written(&client),
    };
    let mut admitted = 0;

    let before = checks(&client).len();
    let permitted = world.check_with(&request, &Fresh::new(&evaluator, &reached), E - 1)??;
    admitted += 1;
    assert_eq!(checks(&client).len() - before, 1);
    let person = IdentityId::Person(permitted.root_person);

    // The person the agent's grant derives through is suspended: SpiceDB
    // still permits, and the permit is refused by the directory's rule.
    world.suspend(person)?;
    let before = checks(&client).len();
    let refused = world.check_with(&request, &Fresh::new(&evaluator, &reached), E - 1)?;
    admitted += usize::from(refused.is_ok());
    let words = refused
        .as_ref()
        .err()
        .map(ToString::to_string)
        .unwrap_or_default();
    assert!(
        matches!(refused, Err(GrantError::IdentityNotActive { .. })),
        "{refused:?}"
    );
    assert!(words.contains(&person.to_string()), "{words}");
    assert_eq!(checks(&client).len() - before, 1);

    // The caller itself is suspended: refused before SpiceDB is asked.
    world.suspend(agent)?;
    let before = checks(&client).len();
    let refused = world.check_with(&request, &Fresh::new(&evaluator, &reached), E - 1)?;
    admitted += usize::from(refused.is_ok());
    let words = refused
        .as_ref()
        .err()
        .map(ToString::to_string)
        .unwrap_or_default();
    assert!(words.starts_with("IdentityNotActive"), "{words}");
    assert!(words.contains(&agent.to_string()), "{words}");
    assert_eq!(checks(&client).len() - before, 0);
    assert_eq!(admitted, 1);
    Ok(())
}

#[tokio::test]
async fn a_configuration_naming_no_grpc_address_decides_no_grant() -> TestResult {
    let dir = tempfile::TempDir::new()?;
    let key_file = dir.path().join("spicedb.key");
    secret(&key_file, b"never-sent")?;
    let gateway_only = lys_identity_server::spicedb::SpiceDbSettings {
        endpoint: "does-not-exist.invalid:8443".to_owned(),
        key_file,
        mirror: "grants".to_owned(),
        grpc: None,
        max_updates_per_write: 1000,
    };
    let (service, seeded) = identity_contract::harness::Service::start_judging(
        GRANT_MODEL,
        Some(gateway_only),
        |config| Ok(seed_configured(config, [ADMINISTRATOR, "bea-subject"])?),
    )
    .await?;
    let ada = service
        .sign_in(Login {
            subject: ADMINISTRATOR.to_owned(),
            email: "shared@example.test".to_owned(),
        })
        .await?;
    let root = json!({
        "operation": OperationId::generate()?.to_string(), "route": "api",
        "holder": seeded.people[0].id.to_string(),
        "resource": { "kind": "doc", "id": "1" }, "relation": "alpha",
        "pass_on": { "kind": "use_only" }, "window": { "starts_at": 0, "ends_at": null },
    });
    let question = json!({
        "route": "api", "resource": { "kind": "doc", "id": "1" }, "action": "read",
    });
    let mut refused = 0;
    for (path, body) in [
        ("/grants/roots", Some(&root)),
        ("/grants/check", Some(&question)),
        ("/grants/why", Some(&question)),
        ("/grants", None),
    ] {
        let (status, answer) = match body {
            Some(body) => service.post(path, Some(&ada), body).await?,
            None => service.get(path, Some(&ada)).await?,
        };
        assert_eq!(status, 503, "{path}: {answer}");
        assert_eq!(answer["refusal"], "spicedb_grpc_absent", "{path}: {answer}");
        let reason = answer["reason"].as_str().unwrap_or_default();
        assert!(reason.contains("does-not-exist.invalid:8443"), "{reason}");
        assert!(!reason.contains("never-sent"), "{reason}");
        refused += 1;
    }
    assert_eq!(refused, 4);
    Ok(())
}
