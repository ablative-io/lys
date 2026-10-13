#![cfg(test)]

use std::error::Error;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, Instant};

use axum::{Json, extract::State};
use lys_identity::grants::{
    Action, ExerciseRequest, PassOn, Relation, Resource, RootRequest, Route, Window,
};
use lys_identity::{Actor, AuthMethod, IdentityId, OperationId, Provenance};

use super::Table;
use crate::grants_batch::{BatchBody, CheckWire};
use crate::spicedb::ReadHook;

fn asked(table: &Table, subject: IdentityId) -> Result<ExerciseRequest, Box<dyn Error>> {
    Ok(ExerciseRequest {
        caller: subject,
        route: Route::Api,
        resource: Resource::new("agent", &table.agent.to_string())?,
        action: Action::new("read")?,
    })
}

fn administrator_headers(table: &Table) -> Result<axum::http::HeaderMap, Box<dyn Error>> {
    let binding = table
        .state
        .admission
        .administrator_login()?
        .ok_or("administrator missing")?;
    let actor = Actor::new(
        binding,
        Provenance::new(AuthMethod::Oidc, crate::session::now()),
    );
    let cookie = table.state.sessions.begin(actor)?;
    let mut headers = axum::http::HeaderMap::new();
    headers.insert(
        axum::http::header::COOKIE,
        cookie.split(';').next().ok_or("cookie missing")?.parse()?,
    );
    Ok(headers)
}

fn body(table: &Table, subject: IdentityId, count: usize) -> BatchBody {
    BatchBody {
        checks: (0..count)
            .map(|_| CheckWire {
                subject: subject.to_string(),
                kind: "agent".to_owned(),
                id: table.agent.to_string(),
                action: "read".to_owned(),
            })
            .collect(),
        at_least: None,
    }
}

struct Release(mpsc::Sender<()>);
impl Drop for Release {
    fn drop(&mut self) {
        let result = self.0.send(());
        assert!(result.is_ok(), "delayed fixture disappeared before release");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_slow_decision_releases_the_locks_for_an_unrelated_decision_and_refresh()
-> Result<(), Box<dyn Error>> {
    let mut table = Table::fresh().await?;
    crate::provider::tests::prepare_refresh(
        Arc::get_mut(&mut table.state).ok_or("shared fixture state")?,
        table.actor.clone(),
        table.asker,
    )?;
    let (entered, entered_rx) = mpsc::channel();
    let (release, release_rx) = mpsc::channel();
    let release_rx = Mutex::new(release_rx);
    let calls = Arc::new(AtomicUsize::new(0));
    let counted = Arc::clone(&calls);
    let subject = IdentityId::Person(table.asker);
    table
        .state
        .grants
        .lock()
        .map_err(|e| e.to_string())?
        .as_mut()
        .ok_or("grants missing")?
        .relationships()
        .on_read(ReadHook(Box::new(move |caller, _| {
            if caller == subject && counted.fetch_add(1, Ordering::SeqCst) == 0 {
                entered.send(()).map_err(|e| {
                    lys_identity::grants::GrantError::PermissionEngineUnavailable {
                        reason: e.to_string(),
                    }
                })?;
                release_rx
                    .lock()
                    .map_err(
                        |e| lys_identity::grants::GrantError::PermissionEngineUnavailable {
                            reason: e.to_string(),
                        },
                    )?
                    .recv()
                    .map_err(
                        |e| lys_identity::grants::GrantError::PermissionEngineUnavailable {
                            reason: e.to_string(),
                        },
                    )?;
            }
            Ok(())
        })))?;
    let table = Arc::new(table);
    let held = Arc::clone(&table);
    let runtime = tokio::runtime::Handle::current();
    let headers = administrator_headers(&table)?;
    let first = std::thread::spawn(move || {
        runtime.block_on(crate::grants_batch::batch(
            State(Arc::clone(&held.state)),
            headers,
            Ok(Json(body(&held, subject, 1))),
        ))
    });
    let release = Release(release);
    entered_rx.recv()?;
    assert!(
        table.state.directory.try_lock().is_ok(),
        "the slow read holds the directory lock"
    );
    assert!(
        table.state.grants.try_lock().is_ok(),
        "the slow read holds the grants lock"
    );
    let started = Instant::now();
    let request = asked(&table, IdentityId::Agent(table.holder))?;
    crate::grants_reads::with_read_grants(
        &table.state,
        None,
        3,
        |_| Ok(vec![request.clone()]),
        |mut judged| {
            crate::grants::decide(
                &mut judged,
                &request,
                3,
                None,
                crate::grants::Decision::Explain,
            )
            .map(drop)
            .map_err(Into::into)
        },
    )?;
    let refresh = crate::provider::tests::refresh(&table.state)?;
    assert!(refresh.get("access_token").is_some());
    let elapsed = started.elapsed();
    eprintln!(
        "unrelated decision and refresh completed in {elapsed:?} while the first read remained held"
    );
    assert!(elapsed < Duration::from_secs(2));
    drop(release);
    let answer = first
        .join()
        .map_err(|p| format!("decision panicked: {p:?}"))??;
    assert_eq!(answer.results.len(), 1);
    assert!(answer.results[0].allowed);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_batch_reuses_one_subject_resource_read() -> Result<(), Box<dyn Error>> {
    let table = Table::fresh().await?;
    let calls = Arc::new(AtomicUsize::new(0));
    let counted = Arc::clone(&calls);
    table
        .state
        .grants
        .lock()
        .map_err(|e| e.to_string())?
        .as_ref()
        .ok_or("grants missing")?
        .relationships()
        .on_read(ReadHook(Box::new(move |_, _| {
            counted.fetch_add(1, Ordering::SeqCst);
            Ok(())
        })))?;
    let answer = crate::grants_batch::batch(
        State(Arc::clone(&table.state)),
        administrator_headers(&table)?,
        Ok(Json(body(&table, IdentityId::Person(table.asker), 12))),
    )
    .await?;
    assert_eq!(answer.results.len(), 12);
    assert!(answer.results.iter().all(|answer| answer.allowed));
    assert_eq!(
        calls.load(Ordering::SeqCst),
        1,
        "one read_about for twelve questions about one resource"
    );
    table.close()?;
    Ok(())
}

async fn moves(count: usize) -> Result<(), Box<dyn Error>> {
    let table = Arc::new(Table::fresh().await?);
    let weak = Arc::downgrade(&table);
    let calls = Arc::new(AtomicUsize::new(0));
    let counted = Arc::clone(&calls);
    table
        .state
        .grants
        .lock()
        .map_err(|e| e.to_string())?
        .as_ref()
        .ok_or("grants missing")?
        .relationships()
        .on_read(ReadHook(Box::new(move |_, _| {
            if counted.fetch_add(1, Ordering::SeqCst) < count {
                let table = weak.upgrade().ok_or_else(|| {
                    lys_identity::grants::GrantError::LogUnavailable {
                        reason: "fixture ended".to_owned(),
                    }
                })?;
                if table.state.grants.try_lock().is_err() {
                    return Err(lys_identity::grants::GrantError::LogUnavailable {
                        reason: "the read still holds the grants lock".to_owned(),
                    });
                }
                crate::grants::with_grants(&table.state, |judged| {
                    judged.grants.issue_root(
                        judged.directory,
                        &RootRequest {
                            operation: OperationId::generate()?,
                            caller: IdentityId::Person(judged.root),
                            route: Route::Api,
                            holder: table.asker,
                            resource: Resource::new("agent", &table.agent.to_string())?,
                            relation: Relation::new("reader")?,
                            pass_on: PassOn::UseOnly,
                            window: Window::new(0, None)?,
                        },
                        crate::session::now(),
                    )?;
                    Ok(())
                })
                .map_err(|error| {
                    lys_identity::grants::GrantError::LogUnavailable {
                        reason: error.to_string(),
                    }
                })?;
            }
            Ok(())
        })))?;
    let answer = crate::grants_batch::batch(
        State(Arc::clone(&table.state)),
        administrator_headers(&table)?,
        Ok(Json(body(&table, IdentityId::Person(table.asker), 1))),
    )
    .await;
    if count == 1 {
        let answer = answer?;
        assert_eq!(answer.revision, 3);
        assert!(answer.results[0].allowed);
    } else {
        let error = answer.expect_err("a second revision move must refuse");
        assert_eq!(error.name(), "grant_binding_revision_moved");
    }
    assert_eq!(
        calls.load(Ordering::SeqCst),
        2,
        "read once, then re-read once"
    );
    Arc::try_unwrap(table)
        .map_err(|_| "fixture state still shared")?
        .close()?;
    Ok(())
}

#[tokio::test]
async fn a_write_between_read_and_judgement_is_reread_once() -> Result<(), Box<dyn Error>> {
    moves(1).await
}

#[tokio::test]
async fn a_second_write_between_read_and_judgement_refuses_binding_revision_moved()
-> Result<(), Box<dyn Error>> {
    moves(2).await
}

#[tokio::test]
async fn distinct_resources_read_at_most_once_each() -> Result<(), Box<dyn Error>> {
    let table = Table::fresh().await?;
    let calls = Arc::new(AtomicUsize::new(0));
    let counted = Arc::clone(&calls);
    table
        .state
        .grants
        .lock()
        .map_err(|e| e.to_string())?
        .as_ref()
        .ok_or("grants missing")?
        .relationships()
        .on_read(ReadHook(Box::new(move |_, _| {
            counted.fetch_add(1, Ordering::SeqCst);
            Ok(())
        })))?;
    let mut request = body(&table, IdentityId::Person(table.asker), 2);
    request.checks[1].id = table.holder.to_string();
    let answer = crate::grants_batch::batch(
        State(Arc::clone(&table.state)),
        administrator_headers(&table)?,
        Ok(Json(request)),
    )
    .await?;
    assert_eq!(answer.results.len(), 2);
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    table.close()?;
    Ok(())
}
