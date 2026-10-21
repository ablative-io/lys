#![cfg(test)]
use std::error::Error;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use lys_identity::grants::{PassOn, Relation, Resource, RootRequest, Route, Window};
use lys_identity::{IdentityId, OperationId, PersonId};
use serde_json::json;

use crate::apps_state::{Approved, By, Client, Line, Registered};
use crate::grants::with_grants;
use crate::routes::AppState;
use crate::spicedb::ReadHook;

async fn fixture(count: usize) -> Result<(tempfile::TempDir, AppState, PersonId), Box<dyn Error>> {
    let (dir, state, _, person, _, _) = crate::grant_settlement_tests::person_fixture().await?;
    with_grants(&state, |judged| {
        judged.apps.keep(Line::Registered(Registered {
            operation: "batch-register".to_owned(), app: "notes".to_owned(), name: "Notes".to_owned(),
            redirects: vec![], service_account: None, by: By::Start, at: 1,
            schema: json!({"kinds":{"notes.doc":{"actions":["read"],"relations":{"viewer":["read"]}}}}),
        }))?;
        judged.apps.keep(Line::Approved(Approved {
            operation: "batch-approve".to_owned(),
            app: "notes".to_owned(),
            client: Client {
                client_id: "notes".to_owned(),
                secret_sha256: String::new(),
            },
            binding: None,
            by: By::Start,
            at: 1,
        }))?;
        let model = judged.apps.model()?;
        judged.grants.set_kinds(model.kinds().clone());
        state.grant_setup.hold_model(model)?;
        state
            .grant_setup
            .model_revision
            .store(judged.apps.model_revision(), Ordering::Release);
        for index in 0..count {
            judged.grants.issue_root(
                judged.directory,
                &RootRequest {
                    operation: OperationId::generate()?,
                    caller: IdentityId::Person(judged.root),
                    route: Route::Api,
                    holder: person,
                    resource: Resource::new("notes.doc", &index.to_string())?,
                    relation: Relation::new("viewer")?,
                    pass_on: PassOn::UseOnly,
                    window: Window::new(0, None)?,
                },
                crate::session::now(),
            )?;
        }
        Ok(())
    })?;
    Ok((dir, state, person))
}

#[tokio::test]
async fn rights_batch_cold_pass_matches_two_hundred_resource_rights_and_paths()
-> Result<(), Box<dyn Error>> {
    let (dir, state, person) = fixture(200).await?;
    let subject = IdentityId::Person(person);
    let at = crate::session::now();
    let reference = crate::grants_reads::with_read_grants(
        &state,
        None,
        at,
        |judged| crate::grants_reads::subject_questions(judged, subject, Some("notes")),
        |mut judged| super::rights_claim::rights(&mut judged, subject, "notes", at, None, None),
    )?;
    assert_eq!(reference.rights.len(), 200);
    assert_eq!(reference.paths.len(), 200);
    let calls = Arc::new(AtomicUsize::new(0));
    let counted = Arc::clone(&calls);
    with_grants(&state, |judged| {
        judged
            .grants
            .relationships()
            .on_read(ReadHook(Box::new(move |_, _| {
                counted.fetch_add(1, Ordering::SeqCst);
                Ok(())
            })))?;
        Ok(())
    })?;
    let started = std::time::Instant::now();
    let (_, actual, _, hit) =
        super::rights_cached::read(&state, subject, "notes", at, false, None)?;
    assert!(!hit);
    assert_eq!(actual, reference);
    eprintln!("cold_pass_200 elapsed_ms={}", started.elapsed().as_millis());
    assert_eq!(
        calls.load(Ordering::SeqCst),
        1,
        "one relationship batch for the cold pass"
    );
    drop(state);
    dir.close()?;
    Ok(())
}

#[tokio::test]
async fn rights_batch_log_names_one_batch_and_truncation() -> Result<(), Box<dyn Error>> {
    let (dir, state, person) = fixture(2).await?;
    let heard: Arc<Mutex<Vec<String>>> = Arc::default();
    let captured = Arc::clone(&heard);
    let say: crate::routes::Say = Arc::new(move |line| match captured.lock() {
        Ok(mut lines) => lines.push(line.to_owned()),
        Err(error) => panic!("fixture log poisoned: {error}"),
    });
    tracing::subscriber::with_default(crate::warn_lines::WarnLines::new(say), || {
        super::rights_cached::read(
            &state,
            IdentityId::Person(person),
            "notes",
            crate::session::now(),
            false,
            None,
        )
    })?;
    let lines = heard.lock().map_err(|error| error.to_string())?;
    let line = lines
        .iter()
        .find(|line| line.contains("pass rights SpiceDB reads"))
        .ok_or("rights log missing")?;
    assert!(line.contains("resources_read=1"), "{line}");
    assert!(line.contains("batch=yes"), "{line}");
    assert!(line.contains("truncated=no"), "{line}");
    drop(lines);
    drop(state);
    dir.close()?;
    Ok(())
}

#[tokio::test]
async fn rights_batch_binding_still_refuses_when_both_snapshots_move() -> Result<(), Box<dyn Error>>
{
    let (dir, state, person) = fixture(2).await?;
    let state = Arc::new(state);
    let weak = Arc::downgrade(&state);
    let moves = Arc::new(AtomicUsize::new(0));
    let counted = Arc::clone(&moves);
    // The root the hook issues reads the engine itself (the caller's
    // organisation.grant, ACCESS-009 R2): those reads are not the batch's.
    let moving = Arc::new(AtomicBool::new(false));
    with_grants(&state, |judged| {
        judged
            .grants
            .relationships()
            .on_read(ReadHook(Box::new(move |_, _| {
                if moving.load(Ordering::SeqCst) {
                    return Ok(());
                }
                let state = weak.upgrade().ok_or_else(|| {
                    lys_identity::grants::GrantError::LogUnavailable {
                        reason: "fixture ended".to_owned(),
                    }
                })?;
                assert!(
                    state.grants.try_lock().is_ok(),
                    "the batch must release the authority hold"
                );
                let index = counted.fetch_add(1, Ordering::SeqCst);
                moving.store(true, Ordering::SeqCst);
                let issued = with_grants(&state, |judged| {
                    judged.grants.issue_root(
                        judged.directory,
                        &RootRequest {
                            operation: OperationId::generate()?,
                            caller: IdentityId::Person(judged.root),
                            route: Route::Api,
                            holder: person,
                            resource: Resource::new("notes.doc", &format!("moved-{index}"))?,
                            relation: Relation::new("viewer")?,
                            pass_on: PassOn::UseOnly,
                            window: Window::new(0, None)?,
                        },
                        crate::session::now(),
                    )?;
                    Ok(())
                });
                moving.store(false, Ordering::SeqCst);
                issued.map_err(|error| lys_identity::grants::GrantError::LogUnavailable {
                    reason: error.to_string(),
                })
            })))?;
        Ok(())
    })?;
    let answer = super::rights_cached::read(
        &state,
        IdentityId::Person(person),
        "notes",
        crate::session::now(),
        true,
        None,
    );
    assert_eq!(
        answer.err().ok_or("moved binding was accepted")?.name(),
        "grant_binding_revision_moved"
    );
    assert_eq!(moves.load(Ordering::SeqCst), 2);
    drop(state);
    dir.close()?;
    Ok(())
}

#[tokio::test]
async fn rights_batch_a_certain_bound_skips_resource_reads_but_not_binding()
-> Result<(), Box<dyn Error>> {
    let (dir, state, person) = fixture(2).await?;
    let subject = IdentityId::Person(person);
    let heard: Arc<Mutex<Vec<String>>> = Arc::default();
    let captured = Arc::clone(&heard);
    let say: crate::routes::Say = Arc::new(move |line| match captured.lock() {
        Ok(mut lines) => lines.push(line.to_owned()),
        Err(error) => panic!("fixture log poisoned: {error}"),
    });
    let (_, read, _, hit) =
        tracing::subscriber::with_default(crate::warn_lines::WarnLines::new(say), || {
            super::rights_cached::read(
                &state,
                subject,
                "notes",
                crate::session::now(),
                false,
                Some(1),
            )
        })?;
    assert!(!hit);
    assert!(read.rights.is_empty());
    let lines = heard.lock().map_err(|error| error.to_string())?;
    let line = lines
        .iter()
        .find(|line| line.contains("pass rights SpiceDB reads"))
        .ok_or("rights log missing")?;
    assert!(
        line.contains("resources_read=0")
            && line.contains("batch=no")
            && line.contains("truncated=yes"),
        "{line}"
    );
    drop(lines);
    let (_, bound, revision, _) = super::rights_cached::read(
        &state,
        subject,
        "notes",
        crate::session::now(),
        true,
        Some(1),
    )?;
    assert_eq!(bound.rights.len(), 2);
    assert!(revision.is_some());
    drop(state);
    dir.close()?;
    Ok(())
}
