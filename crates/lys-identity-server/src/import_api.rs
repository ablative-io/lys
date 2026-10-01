//! One import walks existing mutation owners in order. It is not an atomic
//! transaction: a refusal names the entry and the completed prefix. A repeat
//! uses content-derived operation ids and reads the original receipts.
//! Every entry authenticates and checks its authority again. Root grants
//! remain administrator-only; a loader never becomes its owner to issue one.

use std::collections::BTreeMap;
use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Response};
use lys_identity::IdentityId;
use lys_identity::import_document::{Entry, Kind};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::apps_state::By;
use crate::error::ServerError;
use crate::routes::AppState;

/// Each entry is checked against the existing mutation contract, with its
/// operation supplied by the importer. Grants also have a document-local name.
#[derive(Deserialize, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ImportDocument {
    /// `RegisterBody` entries without operation ids.
    #[serde(default)]
    #[schema(value_type = Vec<Object>)]
    apps: Vec<Value>,
    /// Agent entries with `display_name`.
    #[serde(default)]
    #[schema(value_type = Vec<Object>)]
    agents: Vec<Value>,
    /// Named `RootBody` entries; root authority is still required.
    #[serde(default)]
    #[schema(value_type = Vec<Object>)]
    root_grants: Vec<Value>,
    /// Named `DelegateBody` entries without operation ids.
    #[serde(default)]
    #[schema(value_type = Vec<Object>)]
    delegations: Vec<Value>,
}

/// The actual account and each existing owner's receipt, in document order.
#[derive(Serialize, utoipa::ToSchema)]
pub(crate) struct ImportAnswer {
    /// The independently authenticated account, always kind `service_account`.
    #[schema(value_type = Object)]
    by: By,
    /// Entry name, operation id and the mutation owner's original result.
    #[schema(value_type = Vec<Object>)]
    completed: Vec<Value>,
}

fn malformed(reason: impl Into<String>) -> ServerError {
    ServerError::RequestMalformed {
        reason: reason.into(),
    }
}

fn decode<T: DeserializeOwned>(body: Value) -> Result<T, ServerError> {
    serde_json::from_value(body)
        .map_err(|_error| malformed("import entry does not match its route's request body"))
}

fn value<T: Serialize>(answer: T) -> Result<Value, ServerError> {
    serde_json::to_value(answer).map_err(|_error| malformed("import receipt could not be encoded"))
}

fn validate(entry: &Entry) -> Result<(), ServerError> {
    match entry.kind {
        Kind::App => {
            decode::<crate::apps_api::RegisterBody>(entry.body.clone())?;
        }
        Kind::Agent => {
            decode::<crate::routes::AgentRegistration>(entry.body.clone())?;
        }
        Kind::Root => {
            decode::<crate::grant_contract::RootBody>(entry.body.clone())?;
        }
        Kind::Delegation => {
            decode::<crate::grant_contract::DelegateBody>(entry.body.clone())?;
        }
    }
    Ok(())
}

fn refusal(entry: Option<&Entry>, completed: &[Value], error: &ServerError) -> Response {
    (
        error.status(),
        Json(json!({
            "refusal": error.name(), "reason": error.to_string(), "fields": error.fields(),
            "entry": entry.map(|entry| format!("{}/{}", entry.kind.section(), entry.name)),
            "operation": entry.map(|entry| entry.operation.to_string()),
            "completed": completed,
        })),
    )
        .into_response()
}

fn recorded_refusal(
    state: &AppState,
    account: &str,
    entry: &Entry,
    completed: &[Value],
    error: &ServerError,
) -> Response {
    let record = crate::service_accounts_state::ImportRefused {
        account: account.to_owned(),
        operation: entry.operation.to_string(),
        entry: format!("{}/{}", entry.kind.section(), entry.name),
        refusal: error.name(),
        at: crate::session::now(),
    };
    let recorded = state
        .service_accounts
        .as_ref()
        .ok_or_else(|| ServerError::ServiceAccountsUnavailable {
            reason: "service account log is not configured".to_owned(),
        })
        .and_then(|store| {
            store
                .lock()
                .map_err(|error| ServerError::ServiceAccountsUnavailable {
                    reason: format!("the service accounts lock is poisoned: {error}"),
                })?
                .record_import_refusal(record)
        });
    let (audit, audit_failure, status) = match recorded {
        Ok(record) => (Some(record), None, error.status()),
        Err(failure) => (
            None,
            Some(failure.name()),
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
        ),
    };
    (status, Json(json!({"refusal":error.name(), "reason":error.to_string(), "fields":error.fields(),
        "entry":format!("{}/{}",entry.kind.section(),entry.name), "operation":entry.operation.to_string(),
        "completed":completed, "audit":audit, "audit_failure":audit_failure}))).into_response()
}

fn resolve(body: &mut Value, names: &BTreeMap<String, String>) -> Result<(), ServerError> {
    for field in ["holder", "source", "recipient", "responsible"] {
        let Some(reference) = body
            .get(field)
            .and_then(Value::as_str)
            .filter(|name| name.starts_with('@'))
        else {
            continue;
        };
        let actual = names
            .get(reference)
            .ok_or_else(|| malformed(format!("unresolved import reference in {field}")))?;
        body[field] = Value::String(actual.clone());
    }
    Ok(())
}

async fn apply(
    state: &Arc<AppState>,
    headers: &HeaderMap,
    entry: &Entry,
    body: Value,
) -> Result<Value, ServerError> {
    match entry.kind {
        Kind::App => value(
            crate::apps_api::register(
                State(Arc::clone(state)),
                headers.clone(),
                Ok(Json(decode(body)?)),
            )
            .await?
            .0,
        ),
        Kind::Agent => value(
            crate::routes::register_agent(
                State(Arc::clone(state)),
                headers.clone(),
                Json(decode(body)?),
            )
            .await?
            .0,
        ),
        Kind::Root => value(
            crate::grants::issue_root(
                State(Arc::clone(state)),
                headers.clone(),
                Json(decode(body)?),
            )
            .await?
            .0,
        ),
        Kind::Delegation => value(
            crate::grants::delegate(
                State(Arc::clone(state)),
                headers.clone(),
                Json(decode(body)?),
            )
            .await?
            .0,
        ),
    }
}

/// Run a document with the account the bearer credential authenticates.
pub(crate) async fn import(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Result<Json<ImportDocument>, axum::extract::rejection::JsonRejection>,
) -> Response {
    let Ok(Json(document)) = body else {
        return refusal(
            None,
            &[],
            &malformed("expected the import document's arrays and no unknown members"),
        );
    };
    let caller = crate::grants::with_grants(&state, |judged| {
        let caller = crate::service_account_grants::caller(&state, &headers, &judged)?;
        let IdentityId::ServiceAccount(account) = caller else {
            return Err(ServerError::NotAdmitted {
                reason: "an import uses its own service-account credential",
            });
        };
        let owner = judged
            .directory
            .record(caller)
            .and_then(lys_identity::projection::Record::responsible)
            .ok_or(ServerError::ServiceAccountUnknown)?;
        Ok((account.to_string(), owner.to_string()))
    });
    let (account, owner) = match caller {
        Ok(caller) => caller,
        Err(error) => return refusal(None, &[], &error),
    };
    let Ok(encoded) = serde_json::to_vec(&document) else {
        return refusal(None, &[], &malformed("cannot encode import document"));
    };
    let entries = match lys_identity::import_document::parse(&encoded, &account) {
        Ok(entries) => entries,
        Err(error) => return refusal(None, &[], &malformed(error.to_string())),
    };
    for entry in &entries {
        if let Err(error) = validate(entry) {
            return refusal(Some(entry), &[], &error);
        }
    }
    let mut names = BTreeMap::from([
        ("@owner".to_owned(), owner),
        ("@self".to_owned(), account.clone()),
    ]);
    let mut completed = Vec::new();
    for entry in &entries {
        let mut body = entry.body.clone();
        if let Err(error) = resolve(&mut body, &names) {
            return refusal(Some(entry), &completed, &error);
        }
        let answer = match apply(&state, &headers, entry, body).await {
            Ok(answer) => answer,
            Err(error) => return recorded_refusal(&state, &account, entry, &completed, &error),
        };
        let reference = match entry.kind {
            Kind::Agent => answer
                .get("agent")
                .and_then(Value::as_str)
                .map(|id| (format!("@agent/{}", entry.name), id.to_owned())),
            Kind::Root | Kind::Delegation => answer
                .get("grant")
                .and_then(Value::as_str)
                .map(|id| (format!("@grant/{}", entry.name), id.to_owned())),
            Kind::App => None,
        };
        if let Some((name, id)) = reference {
            names.insert(name, id);
        }
        completed.push(json!({"entry": format!("{}/{}", entry.kind.section(), entry.name), "operation": entry.operation.to_string(), "result": answer}));
    }
    Json(ImportAnswer {
        by: By::ServiceAccount { id: account },
        completed,
    })
    .into_response()
}
