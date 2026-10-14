#![cfg(test)]

use std::collections::BTreeSet;
use std::error::Error;
use std::sync::{Arc, Mutex};

use axum::{
    Json,
    extract::State,
    http::{StatusCode, Uri},
};
use lys_identity::IdentityId;
use lys_identity::grants::{
    Action, ExerciseRequest, GrantError, Model, ObjectRef, ReadStamp, Relation, Relationship,
    RelationshipStore, Resource, Route,
};
use serde_json::{Value, json};

use super::{Prepared, ReachRead, ReachSnapshot, preparation::Asked, prepare_reach};
use crate::spicedb::context::Permission;
use crate::spicedb::snapshot::Snapshot;
use crate::spicedb::{Relationships, SpiceDb, SpiceDbConnection, SpiceDbSettings};

#[derive(Default)]
struct Captured {
    schema: String,
    calls: Vec<(String, Value)>,
}

#[derive(Clone)]
struct Gateway {
    revision: u64,
    captured: Arc<Mutex<Captured>>,
}

fn gateway_error(reason: impl Into<String>) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, reason.into())
}

async fn answer(
    State(gateway): State<Gateway>,
    uri: Uri,
    Json(body): Json<Value>,
) -> Result<String, (StatusCode, String)> {
    let mut captured = gateway
        .captured
        .lock()
        .map_err(|error| gateway_error(error.to_string()))?;
    captured.calls.push((uri.path().to_owned(), body.clone()));
    let marker = json!({
        "resource":{"objectType":"lys_mirror","objectId":"reach_fixture"},
        "relation":"revision",
        "subject":{"object":{"objectType":"lys_revision","objectId":gateway.revision.to_string()}},
    });
    let response = match uri.path() {
        "/v1/schema/read" => json!({"schemaText":captured.schema}),
        "/v1/schema/write" => {
            captured.schema = body
                .get("schema")
                .and_then(Value::as_str)
                .ok_or_else(|| gateway_error("schema write carries no schema"))?
                .to_owned();
            json!({})
        }
        "/v1/relationships/read" => json!({"result":{
            "relationship":marker,"readAt":{"token":"reach-snapshot"},
        }}),
        "/v1/relationships/exportbulk" => json!({"result":{"relationships":[marker]}}),
        "/v1/permissions/checkbulk" => {
            let items = body
                .get("items")
                .and_then(Value::as_array)
                .ok_or_else(|| gateway_error("permission batch carries no items"))?;
            let mut pairs = Vec::with_capacity(items.len());
            for item in items {
                let verdict = match item.get("permission").and_then(Value::as_str) {
                    Some("read") => "PERMISSIONSHIP_HAS_PERMISSION",
                    Some("write") => "PERMISSIONSHIP_NO_PERMISSION",
                    _ => return Err(gateway_error("permission batch carries an unknown action")),
                };
                let mut echoed = item.clone();
                echoed
                    .get_mut("subject")
                    .and_then(Value::as_object_mut)
                    .ok_or_else(|| gateway_error("permission batch carries no subject"))?
                    .insert("optionalRelation".to_owned(), json!(""));
                pairs.push(json!({"request":echoed,"item":{"permissionship":verdict}}));
            }
            json!({"checkedAt":{"token":"reach-snapshot"},"pairs":pairs})
        }
        path => {
            return Err(gateway_error(format!(
                "unexpected reach gateway path: {path}"
            )));
        }
    };
    Ok(response.to_string())
}

fn asked(holders: usize, resources: usize) -> Result<Asked, Box<dyn Error>> {
    let mut asked = Asked::new();
    for holder in 1..=holders {
        let subject = IdentityId::Person(format!("person-{holder:032x}").parse()?);
        for index in 0..resources {
            let resource = Resource::new("person", &format!("resource-{index}"))?;
            for name in ["read", "write"] {
                let action = Action::new(name)?;
                let request = ExerciseRequest {
                    caller: subject,
                    route: Route::Api,
                    resource: resource.clone(),
                    action: action.clone(),
                };
                asked.insert(
                    (subject, resource.clone(), action),
                    (request, vec![resource.clone()]),
                );
            }
        }
    }
    Ok(asked)
}

fn exercise(
    settings: &SpiceDbSettings,
    captured: &Mutex<Captured>,
    stamp: &ReadStamp,
    asked: Asked,
    holders: usize,
    resources: usize,
) -> Result<(), String> {
    let read = Action::new("read").map_err(|error| error.to_string())?;
    let write = Action::new("write").map_err(|error| error.to_string())?;
    let model = Model::new(
        1,
        [
            (
                Relation::new("reader").map_err(|error| error.to_string())?,
                BTreeSet::from([read]),
            ),
            (
                Relation::new("writer").map_err(|error| error.to_string())?,
                BTreeSet::from([write]),
            ),
        ],
    )
    .map_err(|error| error.to_string())?;
    let connection = SpiceDbConnection::load(settings).map_err(|error| error.to_string())?;
    let engine = Relationships::SpiceDb(
        SpiceDb::open_connected(&connection, &model).map_err(|error| error.to_string())?,
    );
    let start = captured
        .lock()
        .map_err(|error| error.to_string())?
        .calls
        .len();
    let expected: BTreeSet<_> = asked.keys().cloned().collect();
    let measured = ReachRead::new(resources);
    let data =
        prepare_reach(&engine, stamp, asked, 100, &measured).map_err(|error| error.to_string())?;
    verify_data(&data, &measured, &expected, holders, resources)?;
    let captured = captured.lock().map_err(|error| error.to_string())?;
    verify_calls(&captured.calls[start..], &measured, expected.len())
}

fn verify_data(
    data: &Prepared,
    measured: &ReachRead,
    expected: &BTreeSet<Permission>,
    holders: usize,
    resources: usize,
) -> Result<(), String> {
    data.validation.as_ref().map_err(ToString::to_string)?;
    assert_eq!(measured.holders.get(), holders);
    assert_eq!(measured.questions.get(), holders * resources * 2);
    assert_eq!(data.checks.len(), expected.len());
    assert_eq!(data.readings.len(), usize::from(!expected.is_empty()));
    for (key, result) in &data.checks {
        assert!(expected.contains(key));
        assert_eq!(result, &Ok(key.2.as_str() == "read"));
    }
    for reading in data.readings.values() {
        let reading = reading.as_ref().map_err(ToString::to_string)?;
        assert_eq!(reading.relationships().len(), 1);
        assert!(
            reading.relationships().iter().any(|row| {
                row.resource.kind == "lys_mirror"
                    && row.resource.id == "reach_fixture"
                    && row.relation == "revision"
                    && row.subject.kind == "lys_revision"
                    && row.subject.id == data.revision.to_string()
            }),
            "the one whole reading retains the certified snapshot marker"
        );
    }
    if expected.is_empty() {
        assert!(!data.batch);
        assert_eq!(data.resources_read, 0);
        assert_eq!(measured.batches.get(), 0);
        assert_eq!(
            measured.reason.borrow().as_deref(),
            Some("no carried actions on visible grants")
        );
        return Ok(());
    }
    assert!(data.batch);
    assert_eq!(data.resources_read, 1);
    assert_eq!(measured.batches.get(), 1);
    assert!(measured.reason.borrow().is_none());
    Ok(())
}

fn verify_calls(
    calls: &[(String, Value)],
    measured: &ReachRead,
    questions: usize,
) -> Result<(), String> {
    let count = |path: &str| calls.iter().filter(|(heard, _)| heard == path).count();
    assert_eq!(count("/v1/permissions/check"), 0);
    assert_eq!(
        count("/v1/relationships/read"),
        1,
        "only the settled marker is read"
    );
    if questions == 0 {
        assert_eq!(count("/v1/permissions/checkbulk"), 0);
        assert_eq!(count("/v1/relationships/exportbulk"), 0);
        assert_eq!(calls.len(), 1);
        return Ok(());
    }
    assert_eq!(count("/v1/permissions/checkbulk"), measured.batches.get());
    assert_eq!(
        count("/v1/relationships/exportbulk"),
        measured.batches.get()
    );
    assert_eq!(
        calls.len(),
        4,
        "one marker, export, schema read and permission batch"
    );
    let bulk = calls
        .iter()
        .find(|(path, _)| path == "/v1/permissions/checkbulk")
        .ok_or_else(|| "permission batch was not sent".to_owned())?;
    let items = bulk
        .1
        .get("items")
        .and_then(Value::as_array)
        .ok_or_else(|| "permission batch sent no items".to_owned())?;
    let unique: BTreeSet<_> = items.iter().map(Value::to_string).collect();
    assert_eq!(items.len(), questions);
    assert_eq!(
        unique.len(),
        questions,
        "every permission question is sent once"
    );
    for (path, body) in calls {
        if path == "/v1/relationships/exportbulk" || path == "/v1/permissions/checkbulk" {
            assert_eq!(
                body["consistency"],
                json!({"atExactSnapshot":{"token":"reach-snapshot"}})
            );
        }
    }
    Ok(())
}

async fn gateway_read(
    stamp: ReadStamp,
    holders: usize,
    resources: usize,
) -> Result<(), Box<dyn Error>> {
    let asked = asked(holders, resources)?;
    let dir = tempfile::tempdir()?;
    let key_file = dir.path().join("synthetic-engine-key");
    std::fs::write(&key_file, "reach-test-only")?;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let endpoint = listener.local_addr()?.to_string();
    let captured = Arc::new(Mutex::new(Captured::default()));
    let router = axum::Router::new()
        .fallback(axum::routing::post(answer))
        .with_state(Gateway {
            revision: stamp.revision(),
            captured: Arc::clone(&captured),
        });
    let (stop, stopped) = tokio::sync::oneshot::channel();
    let worker = tokio::spawn(async move {
        axum::serve(listener, router)
            .with_graceful_shutdown(async move {
                assert!(
                    stopped.await.is_ok(),
                    "reach gateway shutdown signal was dropped"
                );
            })
            .await
    });
    let result = tokio::task::spawn_blocking(move || {
        exercise(
            &SpiceDbSettings {
                endpoint,
                key_file,
                mirror: "reach_fixture".to_owned(),
            },
            &captured,
            &stamp,
            asked,
            holders,
            resources,
        )
    })
    .await;
    let signalled = stop
        .send(())
        .map_err(|()| "reach gateway ended before shutdown");
    let joined = worker.await;
    let cleaned = dir.close();
    signalled?;
    joined??;
    cleaned?;
    result??;
    Ok(())
}

#[tokio::test]
async fn reach_gateway_batches_all_unique_holder_resource_actions_into_one_engine_question_call()
-> Result<(), Box<dyn Error>> {
    let (dir, state, _, _, _, _) = crate::grant_settlement_tests::person_fixture().await?;
    let stamp = crate::grants::with_grants(&state, |judged| Ok(judged.grants.read_stamp(None)?))?;
    let result = async {
        gateway_read(stamp.clone(), 1, 1).await?;
        gateway_read(stamp, 68, 20).await
    }
    .await;
    drop(state);
    let cleaned = dir.close();
    result?;
    cleaned?;
    Ok(())
}

#[tokio::test]
async fn reach_gateway_with_no_carried_actions_names_why_no_batch_was_formed()
-> Result<(), Box<dyn Error>> {
    let (dir, state, _, _, _, _) = crate::grant_settlement_tests::person_fixture().await?;
    let stamp = crate::grants::with_grants(&state, |judged| Ok(judged.grants.read_stamp(None)?))?;
    let result = gateway_read(stamp, 0, 20).await;
    drop(state);
    let cleaned = dir.close();
    result?;
    cleaned?;
    Ok(())
}

#[test]
fn reach_whole_snapshot_keeps_coverage_revision_and_immutability_refusals()
-> Result<(), Box<dyn Error>> {
    let subject = IdentityId::Person("person-11111111111111111111111111111111".parse()?);
    let other = IdentityId::Person("person-22222222222222222222222222222222".parse()?);
    let resource = Resource::new("person", "covered")?;
    let outside = Resource::new("person", "outside")?;
    let rows = BTreeSet::from([Relationship {
        resource: ObjectRef {
            kind: "lys_mirror".to_owned(),
            id: "fixture".to_owned(),
        },
        relation: "revision".to_owned(),
        subject: ObjectRef {
            kind: "lys_revision".to_owned(),
            id: "9".to_owned(),
        },
        subject_relation: None,
        ends_at: None,
    }]);
    let mut snapshot = ReachSnapshot(Snapshot::new(
        9,
        BTreeSet::from([(subject, resource.clone())]),
        rows.clone(),
    ));
    assert_eq!(snapshot.revision()?, 9);
    assert_eq!(snapshot.read_about_at(subject, &resource, 9)?, rows);
    assert_eq!(snapshot.read()?, rows);
    assert_eq!(
        snapshot.read_about_at(subject, &resource, 10),
        Err(GrantError::StaleDecision {
            required: 10,
            projected: 9,
        })
    );
    for (holder, asked) in [(other, &resource), (subject, &outside)] {
        assert_eq!(
            snapshot.read_about_at(holder, asked, 9),
            Err(GrantError::PermissionEngineUnavailable {
                reason: "the resource is outside the reach batch".to_owned(),
            },)
        );
    }
    assert_eq!(
        snapshot.write(10, &[], &[]),
        Err(GrantError::PermissionEngineUnavailable {
            reason: "a relationship snapshot is immutable".to_owned(),
        })
    );
    assert_eq!(snapshot.revision()?, 9);
    assert_eq!(snapshot.read()?, rows);
    Ok(())
}
