#![cfg(test)]

use std::error::Error;
use std::os::unix::fs::PermissionsExt;
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
use lys_identity::{
    Actor, AgentId, AuthMethod, Directory, IdentityId, LoginBinding, OperationId, PersonId,
    Profile, Provenance, Transition,
};
use lys_log_store::{FileLeafStore, LeafStore};
use serde_json::json;

use crate::error::ServerError;
use crate::goals_state::{Goal, Holder, HolderKind, Item, Kind, Standing};
use crate::grants::GrantSetup;
use crate::routes::AppState;
use crate::spicedb::Relationships;

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

include!("grant_settlement_fixture_tests.rs");

pub(crate) async fn person_fixture() -> TestResult<(
    tempfile::TempDir,
    AppState,
    Actor,
    PersonId,
    AgentId,
    AgentId,
)> {
    let dir = tempfile::tempdir()?;
    let ready = Table::prepare(dir.path()).await?;
    Ok((
        dir,
        ready.state,
        ready.actor,
        ready.asker,
        ready.holder,
        ready.agent,
    ))
}

fn exact(error: &ServerError) {
    assert_eq!(error.name(), "PermissionEngineUnavailable");
    assert!(
        matches!(error, ServerError::Grant(cause) if cause == &refusal()),
        "{error}"
    );
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
    exact(
        &failed
            .err()
            .ok_or("grant backend failure permitted goal judgement")?,
    );
    assert_eq!(before, after);
    Ok(())
}

#[tokio::test]
async fn runner_question_keeps_the_original_grant_refusal() -> TestResult {
    let table = Table::fresh().await?;
    let question = lys_runner::refusals::GrantQuestion {
        attempt: "attempt".to_owned(),
        session: "session".to_owned(),
        agent: table.holder.to_string(),
        policy_version: 1,
        rule: "read".to_owned(),
        resource: lys_runner::judge::NamedResource {
            kind: "agent".to_owned(),
            id: table.agent.to_string(),
        },
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
    assert!(
        failed.words.contains(&refusal().to_string()),
        "{}",
        failed.words
    );
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
        let acts = table
            .state
            .acts
            .lock()
            .map_err(|error| error.to_string())?
            .len();
        Ok((healthy, failed, before, table.count()?, acts))
    })();
    let (healthy, failed, before, after, acts) = table.finish(readings)?;
    assert!(healthy.is_ok(), "{healthy:?}");
    exact(
        &failed
            .err()
            .ok_or("grant backend failure permitted runner operation")?,
    );
    assert_eq!(before, after);
    assert_eq!(acts, 0);
    Ok(())
}

#[tokio::test]
async fn team_migration_keeps_the_original_grant_refusal() -> TestResult {
    let table = Table::fresh().await?;
    let added = crate::teams_state::Changed {
        operation: "legacy-add".to_owned(),
        team: "legacy-team".to_owned(),
        member: table.agent.to_string(),
        by: crate::read_views::Login {
            provider: table.actor.binding().issuer().to_owned(),
            subject: table.actor.binding().subject().to_owned(),
        },
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
    assert!(
        healthy?,
        "healthy grant did not permit historical attribution"
    );
    exact(
        &failed
            .err()
            .ok_or("team membership answer discarded the original grant refusal")?,
    );
    assert_eq!(before, after);
    Ok(())
}

#[tokio::test]
async fn opening_projection_failure_is_reported_once_without_another_projection() -> TestResult {
    let table = Table::fresh().await?;
    let readings = (|| -> TestResult<_> {
        let administrator = table
            .state
            .admission
            .administrator_login()?
            .ok_or("administrator absent from opening fixture")?;
        let root = table
            .state
            .directory
            .lock()
            .map_err(|error| error.to_string())?
            .projection()?
            .person_for(&administrator)
            .ok_or("administrator not bound in opening fixture")?;
        let writes = Arc::new(AtomicU64::new(0));
        let fault = Arc::new(AtomicBool::new(true));
        let engine = Relationships::faulted_projection(
            MemoryRelationships::default(),
            Arc::clone(&fault),
            Arc::clone(&writes),
        );
        let revision = engine.revision()?;
        let path = table.dir.path().join("grants");
        let mut opened = Grants::open(
            Box::new(move || FileLeafStore::open(&path)),
            Ed25519Identity::load(&table.state.grant_setup.key_file)?,
            engine,
            table.state.grant_setup.model()?,
            root,
        )?;
        let before = writes.load(Ordering::Relaxed);
        let reports = Mutex::new(Vec::new());
        let callbacks = AtomicU64::new(0);
        let report_bytes = AtomicU64::new(0);
        let callback_nanos = AtomicU64::new(0);
        let say = |words: &str| {
            let started = std::time::Instant::now();
            callbacks.fetch_add(1, Ordering::Relaxed);
            let bytes = match u64::try_from(words.len()) {
                Ok(bytes) => bytes,
                Err(error) => panic!("opening_report_fixture_bytes_overflowed: {error}"),
            };
            report_bytes.fetch_add(bytes, Ordering::Relaxed);
            match reports.lock() {
                Ok(mut reports) => reports.push(words.to_owned()),
                Err(error) => panic!("opening_report_fixture_lock_poisoned: {error}"),
            }
            let nanos = match u64::try_from(started.elapsed().as_nanos()) {
                Ok(nanos) => nanos,
                Err(error) => panic!("opening_report_fixture_duration_overflowed: {error}"),
            };
            callback_nanos.fetch_add(nanos, Ordering::Relaxed);
        };
        crate::grants::report_startup(&mut opened, &say);
        crate::grants::report_startup(&mut opened, &say);
        let consumed = opened.take_startup_degradation().is_none();
        let after = writes.load(Ordering::Relaxed);
        let reports = reports.into_inner().map_err(|error| error.to_string())?;
        fault.store(false, Ordering::Release);
        drop(opened);
        Ok((
            reports,
            before,
            after,
            consumed,
            revision,
            callbacks.load(Ordering::Relaxed),
            report_bytes.load(Ordering::Relaxed),
            callback_nanos.load(Ordering::Relaxed),
        ))
    })();
    let (reports, before, after, consumed, revision, callbacks, bytes, nanos) =
        table.finish(readings)?;
    assert_eq!(before, 1);
    assert_eq!(after, before);
    assert!(consumed);
    assert_eq!(
        reports,
        [format!(
            "grant projection degraded at revision {revision}: {}",
            refusal(),
        )]
    );
    assert_eq!(callbacks, 1);
    assert_eq!(usize::try_from(bytes)?, reports[0].len());
    eprintln!(
        "startup Say fixture: {callbacks} callback, 1 mutex, {bytes} bytes, {nanos} ns; sink I/O not measured"
    );
    Ok(())
}

#[tokio::test]
async fn serialized_grant_answers_share_the_selected_degradation_and_omit_private_causes()
-> TestResult {
    let table = Table::fresh().await?;
    let readings = (|| -> TestResult<_> {
        let healthy = healthy_permit(&table)?;
        table.projection_pending()?;
        let degraded = crate::grants::with_grants(&table.state, |mut judged| {
            let frame = judged.grants.frame(judged.directory, None)?;
            let request = lys_identity::grants::ExerciseRequest {
                caller: IdentityId::Person(table.asker),
                route: Route::Api,
                resource: Resource::new("agent", &table.agent.to_string())?,
                action: Action::new("read")?,
            };
            let permit = judged
                .grants
                .explain_in(&frame, &request, crate::session::now())?;
            let shared = permit
                .degraded
                .as_ref()
                .zip(frame.degradation())
                .is_some_and(|(permit, frame)| Arc::ptr_eq(permit, frame));
            let marker = permit
                .degraded
                .as_deref()
                .ok_or(ServerError::RequestMalformed {
                    reason: "degraded permit lost its marker".to_owned(),
                })?;
            let view = crate::grants::DegradedView::from(marker);
            let permit_json = wire(crate::grant_contract::PermitView::from(&permit))?;
            let who = wire(crate::grant_contract::WhoPage {
                holders: Vec::new(),
                revision: frame.revision(),
                complete: true,
                next: None,
                degraded: Some(view.clone()),
            })?;
            let reach = wire(crate::grants_reach::ReachAnswer {
                revision: frame.revision(),
                resources: Vec::new(),
                degraded: Some(view.clone()),
            })?;
            let which = wire(crate::grants_batch::WhichPage {
                ids: Vec::new(),
                modes: Vec::new(),
                next: None,
                revision: frame.revision(),
                degraded: Some(view),
            })?;
            let check = crate::grants_batch::one(
                &mut judged,
                None,
                &crate::grants_batch::CheckWire {
                    subject: table.asker.to_string(),
                    kind: "agent".to_owned(),
                    id: table.agent.to_string(),
                    action: "read".to_owned(),
                },
                crate::session::now(),
                None,
            );
            let batch = wire(crate::grants_batch::BatchAnswer {
                revision: frame.revision(),
                degraded: check.degraded.clone(),
                results: vec![check],
            })?;
            Ok((
                shared,
                frame.revision(),
                [permit_json, who, reach, which, batch],
            ))
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
    assert_eq!(legacy.len(), 8);
    assert!(shared);
    assert!(recovered);
    for answer in answers {
        assert_eq!(
            answer["degraded"],
            json!({
                "step": "project", "refusal": "PermissionEngineUnavailable", "revision": revision,
            })
        );
        assert!(!answer.to_string().contains("instance-private"));
    }
    assert!(document.to_string().contains("DegradedView"));
    Ok(())
}

#[tokio::test]
async fn all_four_boundaries_accept_a_proved_unrelated_degraded_grant() -> TestResult {
    let table = Table::fresh().await?;
    let readings = (|| -> TestResult<_> {
        table.projection_pending()?;
        let before = table.count()?;
        let goal =
            crate::goals_api::judge(&table.state, IdentityId::Person(table.asker), &table.goal());
        let operator = crate::runner_sessions::operator(
            &table.state,
            &table.headers()?,
            &table.agent.to_string(),
            "stop",
        );
        let question = lys_runner::refusals::GrantQuestion {
            attempt: "attempt".to_owned(),
            session: "session".to_owned(),
            agent: table.holder.to_string(),
            policy_version: 1,
            rule: "read".to_owned(),
            resource: lys_runner::judge::NamedResource {
                kind: "agent".to_owned(),
                id: table.agent.to_string(),
            },
            action: "read".to_owned(),
        };
        let answer = crate::grants_refusals::judged(&table.state, &question);
        let added = crate::teams_state::Changed {
            operation: "legacy-add".to_owned(),
            team: "legacy-team".to_owned(),
            member: table.agent.to_string(),
            at: 0,
            by: crate::read_views::Login {
                provider: table.actor.binding().issuer().to_owned(),
                subject: table.actor.binding().subject().to_owned(),
            },
        };
        let team = crate::teams_migration::test_permitted(&table.state, &added);
        Ok((goal, operator, answer, team, before, table.count()?))
    })();
    let (goal, operator, answer, team, before, after) = table.finish(readings)?;
    goal?;
    operator?;
    assert!(answer.permitted, "{}", answer.words);
    assert!(team?);
    assert_eq!(before, after);
    Ok(())
}

#[tokio::test]
async fn projection_degradation_is_private_to_its_service_instance() -> TestResult {
    let first = Table::fresh().await?;
    let second = match Table::fresh().await {
        Ok(second) => second,
        Err(error) => return first.finish(Err(error)),
    };
    let readings = (|| -> TestResult<_> {
        first.projection_pending()?;
        let degraded = crate::grants::with_grants(&first.state, |judged| {
            let frame = judged.grants.frame(judged.directory, None)?;
            Ok(frame.degradation().is_some())
        })?;
        let healthy = crate::grants::with_grants(&second.state, |judged| {
            let frame = judged.grants.frame(judged.directory, None)?;
            Ok(frame.degradation().is_none())
        })?;
        Ok((degraded, healthy, Arc::ptr_eq(&first.fault, &second.fault)))
    })();
    let readings = first.finish(readings);
    let (degraded, healthy, shared_fault) = second.finish(readings)?;
    assert!(degraded);
    assert!(healthy);
    assert!(!shared_fault);
    Ok(())
}

#[path = "grants_lock_tests.rs"]
mod lock_tests;

fn healthy_permit(table: &Table) -> Result<serde_json::Value, ServerError> {
    crate::grants::with_grants(&table.state, |judged| {
        let frame = judged.grants.frame(judged.directory, None)?;
        let request = lys_identity::grants::ExerciseRequest {
            caller: IdentityId::Person(table.asker),
            route: Route::Api,
            resource: Resource::new("agent", &table.agent.to_string())?,
            action: Action::new("read")?,
        };
        let permit = judged
            .grants
            .explain_in(&frame, &request, crate::session::now())?;
        wire(crate::grant_contract::PermitView::from(&permit))
    })
}
