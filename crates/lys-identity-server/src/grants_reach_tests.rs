#![cfg(test)]

use std::collections::BTreeMap;
use std::error::Error;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use axum::Json;
use axum::extract::State;
use lys_identity::IdentityId;
use lys_identity::grants::{GrantError, Route};
use tracing::instrument::WithSubscriber;

use super::{ReachAnswer, ReachBody};
use crate::error::ServerError;
use crate::grants::with_grants;
use crate::spicedb::ReadHook;

#[path = "grants_reach_fixture_tests.rs"]
mod fixture;
use fixture::Fixture;

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

async fn read(
    fixture: &Fixture,
    body: ReachBody,
) -> TestResult<(Result<Json<ReachAnswer>, ServerError>, Vec<String>)> {
    let heard: Arc<Mutex<Vec<String>>> = Arc::default();
    let captured = Arc::clone(&heard);
    let say: crate::routes::Say = Arc::new(move |line| match captured.lock() {
        Ok(mut lines) => lines.push(line.to_owned()),
        Err(error) => panic!("reach fixture log poisoned: {error}"),
    });
    let result = super::reach(
        State(Arc::clone(&fixture.state)),
        fixture.headers.clone(),
        Json(body),
    )
    .with_subscriber(crate::warn_lines::WarnLines::new(say))
    .await;
    let lines = heard.lock().map_err(|error| error.to_string())?.clone();
    Ok((result, lines))
}

fn log(lines: &[String]) -> TestResult<&str> {
    let mut logs = lines
        .iter()
        .filter(|line| line.contains("reach grant reads"));
    let line = logs.next().ok_or("reach INFO line absent")?;
    assert!(
        logs.next().is_none(),
        "one INFO line per reach call: {lines:?}"
    );
    assert!(
        line.starts_with("INFO lys_identity_server::passfast:"),
        "{line}"
    );
    assert!(line.contains("elapsed_ms="), "{line}");
    Ok(line)
}

fn field(line: &str, name: &str) -> TestResult<usize> {
    let prefix = format!("{name}=");
    Ok(line
        .split_whitespace()
        .find_map(|part| part.strip_prefix(&prefix))
        .ok_or_else(|| format!("reach log field absent: {name}: {line}"))?
        .parse()?)
}

#[tokio::test]
async fn reach_sixty_eight_agents_twenty_resources_use_one_engine_batch_and_keep_every_answer()
-> TestResult {
    let fixture = Fixture::new(68, 20).await?;
    let body = fixture.body();
    let expected = fixture.expected(&body);
    let calls = Arc::new(AtomicUsize::new(0));
    let counted = Arc::clone(&calls);
    fixture.on_read(ReadHook(Box::new(move |_, _| {
        counted.fetch_add(1, Ordering::SeqCst);
        Ok(())
    })))?;
    let (answer, lines) = read(&fixture, body).await?;
    let Json(answer) = answer?;
    assert_eq!(
        answer.resources, expected,
        "all holders, actions, modes and duplicate resource order"
    );
    assert!(answer.degraded.is_none(), "{answer:?}");
    let line = log(&lines)?;
    assert_eq!(field(line, "resources")?, 21);
    assert_eq!(field(line, "holders")?, 69);
    assert_eq!(field(line, "questions")?, 2080);
    assert_eq!(field(line, "batches")?, 1);
    assert_eq!(
        calls.load(Ordering::SeqCst),
        1,
        "the actual relationship engine was read once"
    );
    assert!(calls.load(Ordering::SeqCst) <= field(line, "batches")?);
    fixture.close()
}

#[tokio::test]
async fn reach_memo_explains_each_carried_question_once_across_duplicate_resources_actions_and_grants()
-> TestResult {
    let fixture = Fixture::new(4, 3).await?;
    let body = fixture.body();
    let mut questions = BTreeMap::new();
    let answer = with_grants(&fixture.state, |judged| {
        let frame = judged.grants.frame(judged.directory, None)?;
        let asked = super::admit_resources(&judged, &body.resources)?;
        super::views(
            &judged,
            IdentityId::Person(judged.root),
            asked,
            Route::Browser,
            |request| {
                *questions
                    .entry((
                        request.caller,
                        request.resource.clone(),
                        request.action.clone(),
                    ))
                    .or_insert(0) += 1;
                judged
                    .grants
                    .explain_in(&frame, request, crate::session::now())
            },
        )
    })?;
    assert_eq!(answer, fixture.expected(&body));
    assert_eq!(questions.len(), fixture.questions());
    assert!(questions.values().all(|count| *count == 1), "{questions:?}");
    for ((holder, _, action), _) in questions {
        assert_ne!(action.as_str(), "unused");
        if let Some(index) = fixture
            .agents
            .iter()
            .position(|agent| holder == IdentityId::Agent(*agent))
        {
            assert!(
                index % 2 == 1 || action.as_str() == "read",
                "a read-only holder was asked {action}"
            );
        }
    }
    fixture.close()
}

#[tokio::test]
async fn reach_unavailable_relationship_snapshot_refuses_without_serial_engine_fan_out()
-> TestResult {
    let fixture = Fixture::new(4, 3).await?;
    let calls = Arc::new(AtomicUsize::new(0));
    let counted = Arc::clone(&calls);
    fixture.on_read(ReadHook(Box::new(move |_, _| {
        counted.fetch_add(1, Ordering::SeqCst);
        Err(GrantError::PermissionEngineUnavailable {
            reason: "reach fixture batch unavailable".to_owned(),
        })
    })))?;
    let (answer, lines) = read(&fixture, fixture.body()).await?;
    let error = answer
        .err()
        .ok_or("uncertified reach was answered as complete")?;
    assert!(
        error
            .to_string()
            .contains("reach fixture batch unavailable")
    );
    assert_eq!(
        calls.load(Ordering::SeqCst),
        1,
        "one snapshot request; no serial repair or permission fan-out"
    );
    let line = log(&lines)?;
    assert_eq!(field(line, "batches")?, 0);
    assert_eq!(field(line, "questions")?, fixture.questions());
    assert!(line.contains("reach fixture batch unavailable"), "{line}");
    fixture.close()
}

#[tokio::test]
async fn reach_unavailable_batch_and_direct_reads_return_the_original_engine_failure() -> TestResult
{
    let fixture = Fixture::new(2, 2).await?;
    fixture.on_read(ReadHook(Box::new(|_, _| {
        Err(GrantError::PermissionEngineUnavailable {
            reason: "reach fixture engine unavailable".to_owned(),
        })
    })))?;
    let (answer, lines) = read(&fixture, fixture.body()).await?;
    let error = answer.err().ok_or("unavailable engine answered reach")?;
    assert_eq!(error.name(), "PermissionEngineUnavailable");
    assert!(
        matches!(error, ServerError::Grant(GrantError::PermissionEngineUnavailable { ref reason })
        if reason == "reach fixture engine unavailable"),
        "{error}"
    );
    let line = log(&lines)?;
    assert_eq!(field(line, "batches")?, 0);
    assert!(line.contains("reach fixture engine unavailable"), "{line}");
    fixture.close()
}

#[tokio::test]
async fn reach_uncarried_action_asks_nothing_and_names_why_no_batch_was_formed() -> TestResult {
    let fixture = Fixture::new(2, 2).await?;
    let calls = Arc::new(AtomicUsize::new(0));
    let counted = Arc::clone(&calls);
    fixture.on_read(ReadHook(Box::new(move |_, _| {
        counted.fetch_add(1, Ordering::SeqCst);
        Ok(())
    })))?;
    let mut body = fixture.body();
    for resource in &mut body.resources {
        resource.actions = vec!["unused".to_owned()];
    }
    let (answer, lines) = read(&fixture, body).await?;
    let Json(answer) = answer?;
    assert_eq!(answer.resources.len(), 3);
    assert!(
        answer
            .resources
            .iter()
            .all(|resource| resource.holders.is_empty())
    );
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    let line = log(&lines)?;
    assert_eq!(field(line, "holders")?, 0);
    assert_eq!(field(line, "questions")?, 0);
    assert_eq!(field(line, "batches")?, 0);
    assert!(
        line.contains("no carried actions on visible grants"),
        "{line}"
    );
    fixture.close()
}
