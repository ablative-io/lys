#![cfg(test)]
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::sync::{Arc, Mutex};

use axum::{Json, extract::State, http::Uri};
use lys_identity::IdentityId;
use lys_identity::grants::{Action, GrantError, RelationshipStore, Resource};
use serde_json::{Value, json};

use super::super::SpiceDb;

#[derive(Clone, Copy)]
enum Reply {
    Complete,
    Missing,
    Wrong,
    Error,
    BrokenExport,
}

#[derive(Clone)]
struct Gateway {
    heard: Arc<Mutex<Vec<(String, Value)>>>,
    reply: Reply,
    scope: Option<String>,
}

async fn answer(State(gateway): State<Gateway>, uri: Uri, Json(body): Json<Value>) -> String {
    gateway
        .heard
        .lock()
        .unwrap()
        .push((uri.path().to_owned(), body.clone()));
    let prefix = gateway
        .scope
        .as_ref()
        .map_or_else(String::new, |scope| format!("{scope}/"));
    let marker = json!({"resource":{"objectType":format!("{prefix}lys_mirror"),"objectId":"mirror"},
        "relation":"revision","subject":{"object":{"objectType":format!("{prefix}lys_revision"),"objectId":"9"}}});
    match uri.path() {
        "/v1/relationships/read" => {
            json!({"result":{"relationship":marker,"readAt":{"token":"snapshot-fixture"}}})
                .to_string()
        }
        "/v1/schema/read" => {
            json!({"schemaText":format!("definition {prefix}notes/doc {{}}")}).to_string()
        }
        "/v1/relationships/exportbulk" => {
            if matches!(gateway.reply, Reply::BrokenExport) {
                return json!({"error":{"code":13}}).to_string();
            }
            let edge = json!({"resource":{"objectType":format!("{prefix}notes/doc"),"objectId":"one"},
                "relation":"viewer","subject":{"object":{"objectType":format!("{prefix}grant"),"objectId":"grant-one"},"optionalRelation":"holder"}});
            let foreign = json!({"resource":{"objectType":"lys/bforeign/notes/doc","objectId":"foreign"},
                "relation":"viewer","subject":{"object":{"objectType":"lys/bforeign/grant","objectId":"foreign"},"optionalRelation":"holder"}});
            format!(
                "{}\n{}\n",
                json!({"result":{"relationships":[marker],"afterResultCursor":{"token":"page-one"}}}),
                json!({"result":{"relationships":[edge,foreign],"afterResultCursor":{"token":"page-two"}}})
            )
        }
        "/v1/permissions/checkbulk" => {
            let mut pairs: Vec<Value> = body["items"]
                .as_array()
                .unwrap()
                .iter()
                .map(|item| {
                    let verdict = if item["permission"] == "read" {
                        "PERMISSIONSHIP_HAS_PERMISSION"
                    } else {
                        "PERMISSIONSHIP_CONDITIONAL_PERMISSION"
                    };
                    json!({"request":item,"item":{"permissionship":verdict}})
                })
                .collect();
            match gateway.reply {
                Reply::Missing => {
                    pairs.pop();
                }
                Reply::Wrong => pairs[0]["request"]["permission"] = json!("other"),
                Reply::Error => {
                    pairs[0].as_object_mut().unwrap().remove("item");
                    pairs[0]["error"] = json!({"code":13});
                }
                Reply::Complete | Reply::BrokenExport => {}
            }
            json!({"checkedAt":{"token":"snapshot-fixture"},"pairs":pairs}).to_string()
        }
        _ => panic!("unexpected permission gateway path"),
    }
}

async fn exercise(reply: Reply, scope: Option<&str>) -> Result<(), Box<dyn Error>> {
    exercise_given(reply, scope, false).await
}

async fn exercise_given(
    reply: Reply,
    scope: Option<&str>,
    relationships_only: bool,
) -> Result<(), Box<dyn Error>> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let heard = Arc::new(Mutex::new(Vec::new()));
    let gateway = Gateway {
        heard: Arc::clone(&heard),
        reply,
        scope: scope.map(str::to_owned),
    };
    let router = axum::Router::new()
        .fallback(axum::routing::post(answer))
        .with_state(gateway);
    let (stop, stopped) = tokio::sync::oneshot::channel();
    let worker = tokio::spawn(async move {
        axum::serve(listener, router)
            .with_graceful_shutdown(async move {
                stopped.await.unwrap();
            })
            .await
    });
    let engine = SpiceDb {
        endpoint: address.to_string(),
        key: Arc::from("fixture-engine-key"),
        mirror: "mirror".to_owned(),
        relations: BTreeMap::new(),
        app_kinds: Mutex::default(),
        scope: scope.map(str::to_owned),
        client: Arc::default(),
        post_json: super::super::plain_post_json,
        tokens: Arc::default(),
        confirmed: Arc::default(),
    };
    let subject = IdentityId::Person("person-11111111111111111111111111111111".parse()?);
    let resource = Resource::new("notes.doc", "one")?;
    let requested = resource.clone();
    let read = Action::new("read")?;
    let write = Action::new("write")?;
    let checks = if relationships_only {
        BTreeSet::new()
    } else {
        BTreeSet::from([
            (subject, resource.clone(), read.clone()),
            (subject, resource.clone(), write.clone()),
        ])
    };
    let result = tokio::task::spawn_blocking(move || {
        super::super::Relationships::SpiceDb(engine).read_batch(
            9,
            BTreeSet::from([(subject, requested)]),
            &checks,
            100,
        )
    })
    .await;
    stop.send(())
        .map_err(|()| "gateway ended before shutdown")?;
    worker.await??;
    let batch = result?;
    if !relationships_only && matches!(reply, Reply::Missing | Reply::Wrong | Reply::BrokenExport) {
        assert!(matches!(
            batch,
            Err(GrantError::PermissionEngineUnavailable { .. })
        ));
        return Ok(());
    }
    let batch = batch?;
    let reading = batch.snapshot.read()?;
    assert!(
        reading
            .iter()
            .any(|row| row.resource.kind == "notes.doc" && row.resource.id == "one")
    );
    assert!(!reading.iter().any(|row| row.resource.id == "foreign"));
    if relationships_only {
        assert!(batch.checks.is_empty());
        let heard = heard.lock().map_err(|error| error.to_string())?;
        assert_eq!(
            heard.len(),
            2,
            "one certification and one export; reach needs no permission or schema reads"
        );
        assert!(
            heard
                .iter()
                .all(|(path, _)| path.starts_with("/v1/relationships/"))
        );
        return Ok(());
    }
    if matches!(reply, Reply::Error) {
        assert!(matches!(
            batch.checks[&(subject, resource.clone(), read.clone())],
            Err(GrantError::PermissionEngineUnavailable { .. })
        ));
    } else {
        assert_eq!(batch.checks[&(subject, resource.clone(), read)], Ok(true));
    }
    assert_eq!(batch.checks[&(subject, resource, write)], Ok(false));
    assert_snapshot_calls(&heard)
}

fn assert_snapshot_calls(heard: &Mutex<Vec<(String, Value)>>) -> Result<(), Box<dyn Error>> {
    let heard = heard.lock().map_err(|error| error.to_string())?;
    for path in ["/v1/relationships/exportbulk", "/v1/permissions/checkbulk"] {
        let calls: Vec<_> = heard.iter().filter(|(asked, _)| asked == path).collect();
        assert_eq!(calls.len(), 1);
        assert_eq!(
            calls[0].1["consistency"],
            json!({"atExactSnapshot":{"token":"snapshot-fixture"}})
        );
    }
    assert_eq!(
        heard.len(),
        4,
        "one marker certification, one export, one schema read, one bulk check"
    );
    Ok(())
}

#[tokio::test]
async fn relationship_only_reach_never_asks_permission_checks_that_cannot_affect_its_explanation()
-> Result<(), Box<dyn Error>> {
    exercise_given(Reply::Wrong, None, true).await
}

#[tokio::test]
async fn rights_batch_gateway_streams_all_pages_at_one_snapshot_and_excludes_other_scopes()
-> Result<(), Box<dyn Error>> {
    exercise(Reply::Complete, None).await?;
    exercise(Reply::Complete, Some("lys/bbatch")).await
}

#[tokio::test]
async fn rights_batch_incomplete_or_mismatched_answers_are_named() -> Result<(), Box<dyn Error>> {
    for reply in [
        Reply::Missing,
        Reply::Wrong,
        Reply::Error,
        Reply::BrokenExport,
    ] {
        exercise(reply, None).await?;
    }
    Ok(())
}

#[test]
fn the_engines_echo_with_an_empty_optional_relation_is_the_question_sent_and_a_moved_one_is_not() {
    let sent = serde_json::json!({"resource":{"objectType":"ablative/workspace","objectId":"ablative"},
        "permission":"read","subject":{"object":{"objectType":"ablative/identity","objectId":"person-cfbcf33e3d08c87be44abf5831308247"}},
        "context":{"now":1_791_945_371}});
    // The engine's own echo of 14 Oct 2026, HTTP door 127.0.0.1:58443: `optionalRelation` added, empty.
    let echoed = serde_json::json!({"resource":{"objectType":"ablative/workspace","objectId":"ablative"},
        "permission":"read","subject":{"object":{"objectType":"ablative/identity","objectId":"person-cfbcf33e3d08c87be44abf5831308247"},"optionalRelation":""},
        "context":{"now":1_791_945_371}});
    assert!(super::same_question(&sent, &echoed));
    let mut other = echoed.clone();
    other["resource"]["objectId"] = serde_json::json!("other");
    assert!(!super::same_question(&sent, &other));
    let mut related = echoed.clone();
    related["subject"]["optionalRelation"] = serde_json::json!("member");
    assert!(!super::same_question(&sent, &related));
    let mut moved = echoed;
    moved["permission"] = serde_json::json!("write");
    assert!(!super::same_question(&sent, &moved));
}
