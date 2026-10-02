use std::collections::BTreeMap;
use std::io;
use std::sync::{Arc, Mutex};

use axum::extract::State;
use axum::http::{StatusCode, Uri};
use axum::{Json, Router};
use serde_json::{Value, json};

#[derive(Default)]
struct Held {
    schema: String,
    relationships: Vec<Value>,
}

pub struct Engine {
    pub endpoint: String,
    shutdown: Option<tokio::sync::oneshot::Sender<()>>,
    worker: Option<std::thread::JoinHandle<io::Result<()>>>,
}

impl Engine {
    pub fn start() -> io::Result<Self> {
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
        if let Some(shutdown) = self.shutdown.take() {
            match shutdown.send(()) {
                Ok(()) | Err(()) => {}
            }
        }
        match self.worker.take() {
            Some(worker) => worker
                .join()
                .map_err(|error| io::Error::other(format!("grant_engine_panicked: {error:?}")))?,
            None => Ok(()),
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
}

fn permitted(held: &Held, request: &Value) -> bool {
    let permission = request["permission"].as_str().unwrap_or_default();
    let Some(kind) = request["resource"]["objectType"].as_str() else {
        return false;
    };
    let kinds = definitions(&held.schema);
    let Some(body) = kinds.get(kind) else {
        return false;
    };
    let prefix = format!("permission {permission} = ");
    let relations = body
        .lines()
        .find_map(|line| line.trim().strip_prefix(&prefix))
        .map(|terms| terms.split(" + ").collect::<Vec<_>>())
        .unwrap_or_default();
    held.relationships.iter().any(|resource| {
        resource["resource"] == request["resource"]
            && relations.contains(&resource["relation"].as_str().unwrap_or_default())
            && held.relationships.iter().any(|holder| {
                holder["resource"] == resource["subject"]["object"]
                    && holder["relation"] == "holder"
                    && holder["subject"]["object"] == request["subject"]["object"]
            })
    })
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
        "/v1/relationships/read" => held
            .relationships
            .iter()
            .filter(|relationship| matches(relationship, &request["relationshipFilter"]))
            .map(|relationship| json!({"result": {"relationship": relationship}}).to_string())
            .collect::<Vec<_>>()
            .join("\n"),
        "/v1/relationships/write" => {
            let Some(updates) = request["updates"].as_array() else {
                return (
                    StatusCode::BAD_REQUEST,
                    "fixture_updates_missing".to_owned(),
                );
            };
            for update in updates {
                if !valid(&held, &update["relationship"]) {
                    return (
                        StatusCode::BAD_REQUEST,
                        json!({"message": "resource relation absent from schema", "relationship": update["relationship"]}).to_string(),
                    );
                }
            }
            for update in updates {
                let relationship = &update["relationship"];
                held.relationships.retain(|kept| kept != relationship);
                if update["operation"] == "OPERATION_TOUCH" {
                    held.relationships.push(relationship.clone());
                }
            }
            "{}".to_owned()
        }
        "/v1/permissions/check" => json!({
            "permissionship": if permitted(&held, &request) {
                "PERMISSIONSHIP_HAS_PERMISSION"
            } else {
                "PERMISSIONSHIP_NO_PERMISSION"
            }
        })
        .to_string(),
        _ => return (StatusCode::NOT_FOUND, "fixture_route_unknown".to_owned()),
    };
    (StatusCode::OK, answer)
}
