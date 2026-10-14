use std::collections::BTreeMap;
use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use axum::extract::State;
use axum::http::{StatusCode, Uri};
use axum::{Json, Router};
use serde_json::{Value, json};

struct Held {
    schema: String,
    relationships: Vec<Value>,
    snapshot: u64,
    broken: Arc<AtomicBool>,
    refusing: Arc<AtomicBool>,
}

pub struct Engine {
    pub endpoint: String,
    shutdown: Option<tokio::sync::oneshot::Sender<()>>,
    worker: Option<std::thread::JoinHandle<io::Result<()>>>,
}

impl Engine {
    pub fn start(broken: Arc<AtomicBool>, refusing: Arc<AtomicBool>) -> io::Result<Self> {
        let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
        let endpoint = listener.local_addr()?.to_string();
        listener.set_nonblocking(true)?;
        let (shutdown, stopped) = tokio::sync::oneshot::channel();
        let (ready, started) = std::sync::mpsc::channel();
        let worker = std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?;
            runtime.block_on(async move {
                let listener = tokio::net::TcpListener::from_std(listener)?;
                let app = Router::new()
                    .fallback(exchange)
                    .with_state(Arc::new(Mutex::new(Held {
                        schema: lys_identity::grants::SCHEMA.to_owned(),
                        relationships: Vec::new(),
                        snapshot: 0,
                        broken,
                        refusing,
                    })));
                ready.send(()).map_err(io::Error::other)?;
                axum::serve(listener, app)
                    .with_graceful_shutdown(async move {
                        if let Err(error) = stopped.await {
                            eprintln!("grant_engine_shutdown_signal_failed: {error}");
                        }
                    })
                    .await
            })
        });
        let mut engine = Self {
            endpoint,
            shutdown: Some(shutdown),
            worker: Some(worker),
        };
        if let Err(error) = started.recv() {
            let closed = engine.stop();
            return Err(io::Error::other(format!(
                "grant_engine_start_failed: {error}; shutdown: {closed:?}"
            )));
        }
        Ok(engine)
    }

    pub fn stop(&mut self) -> io::Result<()> {
        let mut failures = Vec::new();
        if let Some(shutdown) = self.shutdown.take()
            && shutdown.send(()).is_err()
        {
            failures.push("grant_engine_shutdown_signal_failed".to_owned());
        }
        if let Some(worker) = self.worker.take() {
            match worker.join() {
                Ok(Ok(())) => {}
                Ok(Err(error)) => failures.push(format!("grant_engine_worker_failed: {error}")),
                Err(error) => failures.push(format!("grant_engine_panicked: {error:?}")),
            }
        }
        if failures.is_empty() {
            Ok(())
        } else {
            Err(io::Error::other(failures.join("; ")))
        }
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        if let Err(error) = self.stop() {
            eprintln!("grant_engine_shutdown_failed: {error}");
        }
    }
}

fn definitions(schema: &str) -> BTreeMap<&str, &str> {
    schema
        .split("definition ")
        .skip(1)
        .filter_map(|block| {
            let (kind, body) = block.split_once('{')?;
            Some((kind.trim(), body.split_once('}')?.0))
        })
        .collect()
}

fn valid(held: &Held, relationship: &Value) -> bool {
    let Some(kind) = relationship["resource"]["objectType"].as_str() else {
        return false;
    };
    let Some(relation) = relationship["relation"].as_str() else {
        return false;
    };
    definitions(&held.schema).get(kind).is_some_and(|body| {
        body.lines().any(|line| {
            line.trim_start()
                .starts_with(&format!("relation {relation}:"))
        })
    })
}

fn matches(relationship: &Value, filter: &Value) -> bool {
    relationship["resource"]["objectType"] == filter["resourceType"]
        && filter["optionalResourceId"]
            .as_str()
            .is_none_or(|id| relationship["resource"]["objectId"] == id)
        && filter["optionalRelation"]
            .as_str()
            .is_none_or(|relation| relationship["relation"] == relation)
        && filter["optionalSubjectFilter"]
            .as_object()
            .is_none_or(|subject| {
                subject.get("subjectType") == relationship.pointer("/subject/object/objectType")
                    && subject
                        .get("optionalSubjectId")
                        .and_then(Value::as_str)
                        .is_none_or(|id| relationship["subject"]["object"]["objectId"] == id)
            })
}

fn permitted(held: &Held, request: &Value) -> Result<bool, &'static str> {
    let permission = request["permission"]
        .as_str()
        .filter(|permission| !permission.is_empty())
        .ok_or("fixture_permission_missing")?;
    let kind = request["resource"]["objectType"]
        .as_str()
        .ok_or("fixture_resource_kind_missing")?;
    for member in [
        "/resource/objectId",
        "/subject/object/objectType",
        "/subject/object/objectId",
    ] {
        if request
            .pointer(member)
            .and_then(Value::as_str)
            .is_none_or(str::is_empty)
        {
            return Err("fixture_permission_object_missing");
        }
    }
    if let Some(relation) = request.pointer("/subject/optionalRelation")
        && relation.as_str() != Some("")
    {
        return Err("fixture_permission_subject_relation_unsupported");
    }
    let kinds = definitions(&held.schema);
    let body = kinds.get(kind).ok_or("fixture_resource_kind_unknown")?;
    let prefix = format!("permission {permission} = ");
    let relations = body
        .lines()
        .find_map(|line| line.trim().strip_prefix(&prefix))
        .ok_or("fixture_permission_unknown")?
        .split(" + ")
        .collect::<Vec<_>>();
    Ok(held.relationships.iter().any(|resource| {
        resource["resource"] == request["resource"]
            && resource["relation"]
                .as_str()
                .is_some_and(|relation| relations.contains(&relation))
            && held.relationships.iter().any(|holder| {
                holder["resource"] == resource["subject"]["object"]
                    && holder["relation"] == "holder"
                    && holder["subject"]["object"] == request["subject"]["object"]
            })
    }))
}

fn permissionship(allowed: bool) -> &'static str {
    if allowed {
        "PERMISSIONSHIP_HAS_PERMISSION"
    } else {
        "PERMISSIONSHIP_NO_PERMISSION"
    }
}

async fn exchange(
    State(state): State<Arc<Mutex<Held>>>,
    uri: Uri,
    Json(request): Json<Value>,
) -> (StatusCode, String) {
    let Ok(mut held) = state.lock() else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "fixture_lock_poisoned".to_owned(),
        );
    };
    let answer = match uri.path() {
        "/v1/schema/read" => json!({"schemaText": held.schema}).to_string(),
        "/v1/schema/write" => {
            let Some(schema) = request["schema"].as_str() else {
                return (StatusCode::BAD_REQUEST, "fixture_schema_missing".to_owned());
            };
            schema.clone_into(&mut held.schema);
            "{}".to_owned()
        }
        "/v1/relationships/read"
            if request["relationshipFilter"]["resourceType"] != "lys_mirror"
                && held.broken.load(Ordering::SeqCst) =>
        {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                json!({"message": "the engine is down"}).to_string(),
            );
        }
        "/v1/relationships/read" => held
            .relationships
            .iter()
            .filter(|relationship| matches(relationship, &request["relationshipFilter"]))
            .map(|relationship| {
                json!({"result": {"relationship": relationship,
                    "readAt": {"token": format!("fixture-snapshot-{}", held.snapshot)}}})
                .to_string()
            })
            .collect::<Vec<_>>()
            .join("\n"),
        "/v1/relationships/exportbulk" if held.broken.load(Ordering::SeqCst) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                json!({"message": "the engine is down"}).to_string(),
            );
        }
        "/v1/relationships/exportbulk" => {
            json!({"result": {"relationships": held.relationships}}).to_string()
        }
        "/v1/relationships/write" if held.refusing.load(Ordering::SeqCst) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                json!({"message": "the engine takes no writes"}).to_string(),
            );
        }
        "/v1/relationships/write" => {
            let Some(updates) = request["updates"].as_array() else {
                return (
                    StatusCode::BAD_REQUEST,
                    "fixture_updates_missing".to_owned(),
                );
            };
            let preconditions: &[Value] = match request.get("optionalPreconditions") {
                None => &[],
                Some(Value::Array(preconditions)) => preconditions,
                Some(_) => {
                    return (
                        StatusCode::BAD_REQUEST,
                        "fixture_preconditions_not_an_array".to_owned(),
                    );
                }
            };
            for precondition in preconditions {
                let found = held
                    .relationships
                    .iter()
                    .any(|row| matches(row, &precondition["filter"]));
                let satisfied = match precondition["operation"].as_str() {
                    Some("OPERATION_MUST_MATCH") => found,
                    Some("OPERATION_MUST_NOT_MATCH") => !found,
                    _ => {
                        return (
                            StatusCode::BAD_REQUEST,
                            "fixture_precondition_unknown".to_owned(),
                        );
                    }
                };
                if !satisfied {
                    return (StatusCode::BAD_REQUEST, "PRECONDITION_FAILED".to_owned());
                }
            }
            for update in updates {
                if !valid(&held, &update["relationship"]) {
                    return (
                        StatusCode::BAD_REQUEST,
                        json!({"message": "resource relation absent from schema", "relationship": update["relationship"]}).to_string(),
                    );
                }
                if !matches!(
                    update["operation"].as_str(),
                    Some("OPERATION_TOUCH" | "OPERATION_DELETE")
                ) {
                    return (StatusCode::BAD_REQUEST, "fixture_update_unknown".to_owned());
                }
            }
            let Some(snapshot) = held.snapshot.checked_add(1) else {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "fixture_snapshot_exhausted".to_owned(),
                );
            };
            for update in updates {
                let relationship = &update["relationship"];
                held.relationships.retain(|kept| kept != relationship);
                if update["operation"] == "OPERATION_TOUCH" {
                    held.relationships.push(relationship.clone());
                }
            }
            held.snapshot = snapshot;
            json!({"writtenAt": {"token": format!("fixture-snapshot-{snapshot}")}}).to_string()
        }
        "/v1/permissions/check" => match permitted(&held, &request) {
            Ok(allowed) => json!({"permissionship": permissionship(allowed)}).to_string(),
            Err(reason) => return (StatusCode::BAD_REQUEST, reason.to_owned()),
        },
        "/v1/permissions/checkbulk" => {
            let Some(items) = request["items"].as_array() else {
                return (StatusCode::BAD_REQUEST, "fixture_items_missing".to_owned());
            };
            let mut pairs = Vec::with_capacity(items.len());
            for item in items {
                let allowed = match permitted(&held, item) {
                    Ok(allowed) => allowed,
                    Err(reason) => return (StatusCode::BAD_REQUEST, reason.to_owned()),
                };
                let mut echoed = item.clone();
                echoed["subject"]["optionalRelation"] = json!("");
                pairs.push(json!({"request": echoed,
                    "item": {"permissionship": permissionship(allowed)}}));
            }
            json!({"checkedAt": {"token": format!("fixture-snapshot-{}", held.snapshot)},
                "pairs": pairs})
            .to_string()
        }
        _ => return (StatusCode::NOT_FOUND, "fixture_route_unknown".to_owned()),
    };
    (StatusCode::OK, answer)
}
