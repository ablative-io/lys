#![cfg(test)]

use std::error::Error;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};

use axum::Json;
use axum::http::{HeaderMap, header};
use lys_core::Ed25519Identity;
use lys_identity::grants::{
    Action, DelegateRequest, GrantError, Grants, MemoryRelationships, Model, PassOn, RecipientKind,
    Relation, RelationshipStore, Resource, RootRequest, Route, Window,
};
use lys_identity::log::Reopen;
use lys_identity::{
    Actor, AgentId, AuthMethod, IdentityId, LoginBinding, OperationId, PersonId, Provenance,
};
use lys_log_store::{FileLeafStore, LeafStore};
use serde_json::json;

use crate::error::ServerError;
use crate::goals_state::{Goal, Holder, HolderKind, Item, Kind, Standing};
use crate::grants::GrantSetup;
use crate::routes::AppState;
use crate::spicedb::Relationships;

#[path = "../../lys-identity/tests/support/world.rs"]
mod world;

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

fn wire(value: impl serde::Serialize) -> Result<serde_json::Value, ServerError> {
    serde_json::to_value(value).map_err(|error| ServerError::RequestMalformed {
        reason: format!("grant fixture answer could not be encoded: {error}"),
    })
}

fn refusal() -> GrantError {
    GrantError::PermissionEngineUnavailable {
        reason: "instance-private settlement refusal".to_owned(),
    }
}

async fn discover(
    config: &crate::Config,
    listener: tokio::net::TcpListener,
) -> TestResult<crate::oidc::Oidc> {
    let issuer = &config.issuer;
    let document = json!({
        "issuer": issuer,
        "authorization_endpoint": format!("{issuer}/authorize"),
        "token_endpoint": format!("{issuer}/token"),
        "jwks_uri": format!("{issuer}/jwks"),
        "response_types_supported": ["code"],
        "subject_types_supported": ["public"],
        "id_token_signing_alg_values_supported": ["EdDSA"],
    });
    let router = axum::Router::new().route(
        "/.well-known/openid-configuration",
        axum::routing::get(move || {
            let answer = document.clone();
            async move { Json(answer) }
        }),
    )
    .route(
        "/jwks",
        axum::routing::get(|| async { Json(json!({ "keys": [] })) }),
    );
    let worker = tokio::spawn(async move { axum::serve(listener, router).await });
    let result = crate::oidc::Oidc::discover(config).await;
    worker.abort();
    match worker.await {
        Ok(ended) => ended?,
        Err(error) if error.is_cancelled() => {}
        Err(error) => return Err(error.into()),
    }
    Ok(result?)
}

struct Table {
    state: AppState,
    fault: Arc<AtomicBool>,
    engine: MemoryRelationships,
    dir: Arc<tempfile::TempDir>,
    asker: PersonId,
    agent: AgentId,
    holder: AgentId,
    actor: Actor,
}

impl Table {
    fn projection_pending(&self) -> TestResult {
        let path = self.dir.path().join("grants");
        let administrator = self.state.admission.administrator_login()?
            .ok_or("administrator absent")?;
        let root = self.state.directory.lock().map_err(|error| error.to_string())?
            .projection()?.person_for(&administrator).ok_or("administrator unbound")?;
        let grants = Grants::open(
            Box::new(move || FileLeafStore::open(&path)),
            Ed25519Identity::load(&self.state.grant_setup.key_file)?,
            Relationships::faulted_projection(
                self.engine.clone(), Arc::clone(&self.fault), Arc::new(AtomicU64::new(0)),
            ), self.state.grant_setup.model()?, root,
        )?;
        *self.state.grants.lock().map_err(|error| error.to_string())? = Some(grants);
        self.fault.store(true, Ordering::Release);
        crate::grants::with_grants(&self.state, |judged| {
            let request = RootRequest {
                operation: OperationId::generate()?, caller: IdentityId::Person(judged.root),
                route: Route::Api, holder: self.asker,
                resource: Resource::new("agent", &self.holder.to_string())?,
                relation: Relation::new("reader")?, pass_on: PassOn::UseOnly,
                window: Window::new(0, None)?,
            };
            match judged.grants.issue_root(judged.directory, &request, crate::session::now()) {
                Err(GrantError::ProjectionPending { .. }) => Ok(()),
                Err(error) => Err(error.into()),
                Ok(_) => Err(ServerError::RequestMalformed {
                    reason: "pending projection was acknowledged".to_owned(),
                }),
            }
        })?;
        Ok(())
    }

    async fn fresh() -> TestResult<Self> {
        let fault = Arc::new(AtomicBool::new(false));
        let engine = MemoryRelationships::default();
        let mut held = world::World::with(
            Box::new(|path: &Path| -> Reopen<FileLeafStore> {
                let path = path.to_owned();
                Box::new(move || FileLeafStore::open(&path))
            }),
            Relationships::faulted(engine.clone(), Arc::clone(&fault)),
        )?;
        let path = held.dir.path();
        let key_file = path.join("service.key");
        let model_file = path.join("model.json");
        std::fs::write(&model_file, br#"{"version":1,"relations":{"operator":["read","operate"],"reader":["read"]}}"#)?;
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let issuer = format!("http://{}", listener.local_addr()?);
        let config: crate::Config = serde_json::from_value(json!({
            "listen": "127.0.0.1:0", "log_dir": path.join("log"),
            "log_origin": "example.test/lys/directory", "event_key_file": key_file,
            "issuer": issuer, "client_id": "settlement-test", "client_secret_file": path.join("client-secret"),
            "redirect_url": "http://127.0.0.1/callback", "session_seconds": 300, "secure_cookie": false,
            "link_audit_source": {"issuer": issuer, "subject": "audit"},
            "grant_log_dir": path.join("grants"), "grant_log_origin": "example.test/lys/grants",
            "grant_model_file": model_file,
        }))?;
        std::fs::write(&config.client_secret_file, b"synthetic-test-credential")?;
        let oidc = discover(&config, listener).await?;
        let administrator = LoginBinding::new(&issuer, "administrator")?;
        let actor = Actor::new(
            LoginBinding::new(&issuer, "asker")?,
            Provenance::new(AuthMethod::Oidc, crate::session::now()),
        );
        let setup_actor = Actor::new(
            administrator.clone(),
            Provenance::new(AuthMethod::Oidc, crate::session::now()),
        );
        held.directory.bind_login(setup_actor.clone(), OperationId::generate()?, held.admin, administrator.clone(), held.now)?;
        held.directory.bind_login(setup_actor, OperationId::generate()?, held.tom, actor.binding().clone(), held.now)?;
        let model: Model = config.grant_model()?;
        let log_dir = path.join("grants");
        held.grants = Grants::open(
            Box::new(move || FileLeafStore::open(&log_dir)),
            Ed25519Identity::load(&key_file)?,
            Relationships::faulted(engine.clone(), Arc::clone(&fault)),
            model.clone(),
            held.admin,
        )?;
        let resource = Resource::new("agent", &held.dana_agent.to_string())?;
        let pass_on = PassOn::to(
            [Action::new("read")?].into_iter().collect(),
            [RecipientKind::Agent].into_iter().collect(),
        )?;
        let root = held.grants.issue_root(held.directory.projection()?, &RootRequest {
            operation: OperationId::generate()?, caller: IdentityId::Person(held.admin), route: Route::Api,
            holder: held.tom, resource: resource.clone(), relation: Relation::new("operator")?,
            pass_on, window: Window::new(0, None)?,
        }, held.now)?.event.grant();
        held.grants.delegate(held.directory.projection()?, &DelegateRequest {
            operation: OperationId::generate()?, caller: IdentityId::Person(held.tom), route: Route::Api,
            source: root, recipient: IdentityId::Agent(held.tom_agent), responsible: held.tom,
            resource, relation: Relation::new("reader")?, pass_on: PassOn::UseOnly, window: Window::new(0, None)?,
        }, held.now)?;
        let key = Arc::new(Ed25519Identity::load(&key_file)?);
        let say: crate::Say = Arc::new(|_| {});
        let apps = crate::apps_api::opened(&config, Arc::clone(&key), &*say)?;
        let model_revision = apps.model_revision();
        let state = AppState {
            changes: crate::changes::Changes::new()?, directory: Mutex::new(held.directory), oidc,
            sign_in: crate::sign_in::IssuerSignIn::configured(&config)?, provider: None, setup: None,
            setup_lock: tokio::sync::Mutex::new(()), import_credential_file: None,
            identity_upstream: "http://127.0.0.1".to_owned(), estate_plan_file: path.join("estate.json"),
            password_policy: None, model_proxy: None, proxy_dir: None,
            sessions: crate::session::Sessions::new(300, false),
            admission: crate::admission::Admission::new(Some(administrator), config.link_audit_binding()?),
            grants: Mutex::new(Some(held.grants)),
            grant_setup: GrantSetup {
                log_dir: config.grant_log_dir.clone(), log_origin: config.grant_log_origin.clone(),
                key_file, model: RwLock::new(model), spicedb: None,
                model_revision: AtomicU64::new(model_revision), refresh: Mutex::new(()),
            },
            secrets: None, requests: None, mcp_requests: None, network: None, joins: None, roles: None,
            provisioning: None, certificates: None, runtime: None, service_accounts: None, reviews: None,
            teams: None, budgets: None, policies: None, stops: None, goals: None,
            configuration: Mutex::new(Box::new(crate::configuration_store::ConfigurationStore::open(&path.join("organisation"), Arc::clone(&key))?)),
            cord: Mutex::new(crate::cord_store::CordStore::open(&path.join("cord"), Arc::clone(&key))?),
            canvas: crate::canvas_store::CanvasStore::beside(&config.log_dir),
            apps: Mutex::new(apps),
            grant_tokens: Mutex::new(crate::grant_token_store::Tokens::open(path.join("grant-tokens.json"))?),
            agent_passes: Arc::new(Mutex::new(crate::agent_pass_store::Passes::open(path.join("agent-passes.json"))?)),
            kept_responsibilities: crate::kept_responsibilities::Kept::load(&path.join("kept-responsibilities.json"))?,
            benches: crate::apps_bench::Benches::new(path.join("benches"), &*say)?,
            sign_in_providers: None, agent_nonces: Mutex::default(), operator_token: None,
            operator_upgrade_file: None, runners: crate::runner_client::Runners::new(Arc::clone(&key), None),
            acts: Mutex::new(crate::runner_acts::ActStore::open(&path.join("runner-acts"), key)?), say,
        };
        Ok(Self { state, fault, engine, dir: held.dir, asker: held.tom, agent: held.dana_agent, holder: held.tom_agent, actor })
    }

    fn count(&self) -> TestResult<u64> {
        Ok(FileLeafStore::open(&self.dir.path().join("grants"))?.extent())
    }

    fn headers(&self) -> TestResult<HeaderMap> {
        let cookie = self.state.sessions.begin(self.actor.clone())?;
        let mut headers = HeaderMap::new();
        headers.insert(header::COOKIE, cookie.split(';').next().ok_or("session cookie absent")?.parse()?);
        Ok(headers)
    }

    fn close(self) -> TestResult {
        self.fault.store(false, Ordering::Release);
        drop(self.state);
        Arc::try_unwrap(self.dir).map_err(|_| "fixture directory still shared")?.close()?;
        Ok(())
    }

    fn finish<T>(self, result: TestResult<T>) -> TestResult<T> {
        match (result, self.close()) {
            (Ok(value), Ok(())) => Ok(value),
            (Err(error), Ok(())) | (Ok(_), Err(error)) => Err(error),
            (Err(error), Err(cleanup)) => {
                Err(format!("fixture act failed: {error}; fixture cleanup failed: {cleanup}").into())
            }
        }
    }

    fn goal(&self) -> Item {
        Item {
            goal: Goal {
                id: "judgement".to_owned(), holder: Holder { kind: HolderKind::Agent, id: self.agent.to_string() },
                kind: Kind::Goal, words: "Read the decision".to_owned(), deadline: None, active: true,
                evidence: None, judged_by: Some("read".to_owned()), reminders: Vec::new(),
                responsible: IdentityId::Agent(self.agent).to_string(), set_by: self.asker.to_string(), at: 0,
            },
            standing: Standing::Open, marked: None, timers: Vec::new(), fired: Vec::new(), changes: Vec::new(),
        }
    }
}

fn exact(error: &ServerError) {
    assert_eq!(error.name(), "PermissionEngineUnavailable");
    assert!(matches!(error, ServerError::Grant(cause) if cause == &refusal()), "{error}");
}

#[tokio::test]
async fn goal_judgement_keeps_the_original_grant_refusal() -> TestResult {
    let table = Table::fresh().await?;
    let goal = table.goal();
    let readings = (|| -> TestResult<_> {
        let healthy = crate::goals_api::judge(&table.state, IdentityId::Person(table.asker), &goal);
        let before = table.count()?;
        table.fault.store(true, Ordering::Release);
        let failed = crate::goals_api::judge(&table.state, IdentityId::Person(table.asker), &goal);
        table.fault.store(false, Ordering::Release);
        Ok((healthy, failed, before, table.count()?))
    })();
    let (healthy, failed, before, after) = table.finish(readings)?;
    healthy?;
    exact(&failed.err().ok_or("grant backend failure permitted goal judgement")?);
    assert_eq!(before, after);
    Ok(())
}

#[tokio::test]
async fn runner_question_keeps_the_original_grant_refusal() -> TestResult {
    let table = Table::fresh().await?;
    let question = lys_runner::refusals::GrantQuestion {
        attempt: "attempt".to_owned(), session: "session".to_owned(), agent: table.holder.to_string(),
        policy_version: 1, rule: "read".to_owned(),
        resource: lys_runner::judge::NamedResource { kind: "agent".to_owned(), id: table.agent.to_string() },
        action: "read".to_owned(),
    };
    let readings = (|| -> TestResult<_> {
        let healthy = crate::grants_refusals::judged(&table.state, &question);
        let before = table.count()?;
        table.fault.store(true, Ordering::Release);
        let failed = crate::grants_refusals::judged(&table.state, &question);
        table.fault.store(false, Ordering::Release);
        Ok((healthy, failed, before, table.count()?))
    })();
    let (healthy, failed, before, after) = table.finish(readings)?;
    assert!(healthy.permitted, "{}", healthy.words);
    assert!(!failed.permitted);
    assert!(failed.words.contains(&refusal().to_string()), "{}", failed.words);
    assert_eq!(failed.attempt, question.attempt);
    assert_eq!(failed.session, question.session);
    assert_eq!(failed.policy_version, question.policy_version);
    assert_eq!(failed.rule, question.rule);
    assert_eq!(before, after);
    Ok(())
}

#[tokio::test]
async fn runner_operator_keeps_the_original_grant_refusal() -> TestResult {
    let table = Table::fresh().await?;
    let agent = table.agent.to_string();
    let readings = (|| -> TestResult<_> {
        let headers = table.headers()?;
        let healthy = crate::runner_sessions::operator(&table.state, &headers, &agent, "stop");
        let before = table.count()?;
        table.fault.store(true, Ordering::Release);
        let failed = crate::runner_sessions::operator(&table.state, &headers, &agent, "stop");
        table.fault.store(false, Ordering::Release);
        let acts = table.state.acts.lock().map_err(|error| error.to_string())?.len();
        Ok((healthy, failed, before, table.count()?, acts))
    })();
    let (healthy, failed, before, after, acts) = table.finish(readings)?;
    assert!(healthy.is_ok(), "{healthy:?}");
    exact(&failed.err().ok_or("grant backend failure permitted runner operation")?);
    assert_eq!(before, after);
    assert_eq!(acts, 0);
    Ok(())
}

#[tokio::test]
async fn team_migration_keeps_the_original_grant_refusal() -> TestResult {
    let table = Table::fresh().await?;
    let added = crate::teams_state::Changed {
        operation: "legacy-add".to_owned(), team: "legacy-team".to_owned(), member: table.agent.to_string(),
        by: crate::read_views::Login { provider: table.actor.binding().issuer().to_owned(), subject: table.actor.binding().subject().to_owned() },
        at: 0,
    };
    let readings = (|| -> TestResult<_> {
        let healthy = crate::teams_migration::test_permitted(&table.state, &added);
        let before = table.count()?;
        table.fault.store(true, Ordering::Release);
        let failed = crate::teams_migration::test_permitted(&table.state, &added);
        table.fault.store(false, Ordering::Release);
        Ok((healthy, failed, before, table.count()?))
    })();
    let (healthy, failed, before, after) = table.finish(readings)?;
    assert!(healthy?, "healthy grant did not permit historical attribution");
    exact(&failed.err().ok_or("team membership answer discarded the original grant refusal")?);
    assert_eq!(before, after);
    Ok(())
}

#[tokio::test]
async fn opening_projection_failure_is_reported_once_without_another_projection() -> TestResult {
    let table = Table::fresh().await?;
    let readings = (|| -> TestResult<_> {
        let administrator = table.state.admission.administrator_login()?
            .ok_or("administrator absent from opening fixture")?;
        let root = table.state.directory.lock().map_err(|error| error.to_string())?
            .projection()?.person_for(&administrator)
            .ok_or("administrator not bound in opening fixture")?;
        let writes = Arc::new(AtomicU64::new(0));
        let fault = Arc::new(AtomicBool::new(true));
        let engine = Relationships::faulted_projection(
            MemoryRelationships::default(), Arc::clone(&fault), Arc::clone(&writes),
        );
        let revision = engine.revision()?;
        let path = table.dir.path().join("grants");
        let mut opened = Grants::open(
            Box::new(move || FileLeafStore::open(&path)),
            Ed25519Identity::load(&table.state.grant_setup.key_file)?,
            engine, table.state.grant_setup.model()?, root,
        )?;
        let before = writes.load(Ordering::Relaxed);
        let reports = Mutex::new(Vec::new());
        let say = |words: &str| {
            match reports.lock() {
                Ok(mut reports) => reports.push(words.to_owned()),
                Err(error) => panic!("opening_report_fixture_lock_poisoned: {error}"),
            }
        };
        crate::grants::report_startup(&mut opened, &say);
        crate::grants::report_startup(&mut opened, &say);
        let consumed = opened.take_startup_degradation().is_none();
        let after = writes.load(Ordering::Relaxed);
        let reports = reports.into_inner().map_err(|error| error.to_string())?;
        fault.store(false, Ordering::Release);
        drop(opened);
        Ok((reports, before, after, consumed, revision))
    })();
    let (reports, before, after, consumed, revision) = table.finish(readings)?;
    assert_eq!(before, 1);
    assert_eq!(after, before);
    assert!(consumed);
    assert_eq!(reports, [format!(
        "grant projection degraded at revision {revision}: {}", refusal(),
    )]);
    Ok(())
}

#[tokio::test]
async fn serialized_grant_answers_share_the_selected_degradation_and_omit_private_causes() -> TestResult {
    let table = Table::fresh().await?;
    let readings = (|| -> TestResult<_> {
        let healthy = crate::grants::with_grants(&table.state, |judged| {
            let frame = judged.grants.frame(judged.directory, None)?;
            let request = lys_identity::grants::ExerciseRequest {
                caller: IdentityId::Person(table.asker), route: Route::Api,
                resource: Resource::new("agent", &table.agent.to_string())?,
                action: Action::new("read")?,
            };
            let permit = judged.grants.explain_in(&frame, &request, crate::session::now())?;
            wire(crate::grant_contract::PermitView::from(&permit))
        })?;
        table.projection_pending()?;
        let degraded = crate::grants::with_grants(&table.state, |mut judged| {
            let frame = judged.grants.frame(judged.directory, None)?;
            let request = lys_identity::grants::ExerciseRequest {
                caller: IdentityId::Person(table.asker), route: Route::Api,
                resource: Resource::new("agent", &table.agent.to_string())?,
                action: Action::new("read")?,
            };
            let permit = judged.grants.explain_in(&frame, &request, crate::session::now())?;
            let shared = permit.degraded.as_ref().zip(frame.degradation())
                .is_some_and(|(permit, frame)| Arc::ptr_eq(permit, frame));
            let marker = permit.degraded.as_deref().ok_or(ServerError::RequestMalformed {
                reason: "degraded permit lost its marker".to_owned(),
            })?;
            let view = crate::grants::DegradedView::from(marker);
            let permit_json = wire(crate::grant_contract::PermitView::from(&permit))?;
            let who = wire(crate::grant_contract::WhoPage {
                holders: Vec::new(), revision: frame.revision(), complete: true, next: None,
                degraded: Some(view.clone()),
            })?;
            let reach = wire(crate::grants_reach::ReachAnswer {
                revision: frame.revision(), resources: Vec::new(), degraded: Some(view.clone()),
            })?;
            let which = wire(crate::grants_batch::WhichPage {
                ids: Vec::new(), next: None, revision: frame.revision(), degraded: Some(view),
            })?;
            let check = crate::grants_batch::one(&mut judged, None, &crate::grants_batch::CheckWire {
                subject: table.asker.to_string(), kind: "agent".to_owned(),
                id: table.agent.to_string(), action: "read".to_owned(),
            }, crate::session::now(), None);
            let batch = wire(crate::grants_batch::BatchAnswer {
                revision: frame.revision(), degraded: check.degraded.clone(), results: vec![check],
            })?;
            Ok((shared, frame.revision(), [permit_json, who, reach, which, batch]))
        })?;
        table.fault.store(false, Ordering::Release);
        let recovered = crate::grants::with_grants(&table.state, |judged| {
            let frame = judged.grants.frame(judged.directory, None)?;
            Ok(frame.degradation().is_none())
        })?;
        Ok((healthy, degraded, recovered, crate::openapi::document()?))
    })();
    let (healthy, (shared, revision, answers), recovered, document) = table.finish(readings)?;
    assert!(healthy.get("degraded").is_none());
    let legacy: serde_json::Map<String, serde_json::Value> = serde_json::from_value(healthy)?;
    assert_eq!(legacy["permitted"], true);
    assert_eq!(legacy.len(), 7);
    assert!(shared);
    assert!(recovered);
    for answer in answers {
        assert_eq!(answer["degraded"], json!({
            "step": "project", "refusal": "PermissionEngineUnavailable", "revision": revision,
        }));
        assert!(!answer.to_string().contains("instance-private"));
    }
    assert!(document.to_string().contains("DegradedView"));
    Ok(())
}
