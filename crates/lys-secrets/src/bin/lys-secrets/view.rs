//! Read-only routes a screen reads: the sealed entries without their values,
//! the grants, and the checked audit log. None of them carries a secret
//! byte, a handle or a digest. Each is asked by a handle's holder or by a
//! trusted screen service for a person (see `callers`), and answers only
//! with what that identity may discover; a secret outside its scope is left
//! out, as one not sealed. The secrets list is a person's view, answered
//! through the broker's access seam, one scope at a time or none.

use std::sync::Arc;

use axum::Json;
use axum::extract::{Query, Request, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use lys_secrets::{Broker, SigningPurpose};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::callers::{self, refused};
use crate::serve::{Shared, on_broker};
use crate::spice::Grants;

type Answer = Result<Json<Value>, (StatusCode, String)>;

fn failed(error: &lys_secrets::SecretsError) -> (StatusCode, String) {
    refused(error)
}

/// The identity the request's caller speaks for. A screen route carries no
/// body, so the signature covers an empty one.
async fn caller(shared: &Arc<Shared>, request: Request) -> Result<String, (StatusCode, String)> {
    let (parts, _body) = request.into_parts();
    callers::caller(shared, &parts, &[])
        .await
        .map(|who| who.identity)
}

/// The query of the secrets list: at most one scope, percent-decoded.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ListAsked {
    #[serde(default)]
    scope: Option<String>,
}

/// The secrets list, a person's view: with `scope` one of `organisation`,
/// `team` and `mine`, the secrets of that scope the caller may see, applied
/// here; with none, every secret the caller may see. Any other scope is
/// refused `unknown_scope` and an agent `agent_uses_virtual_credentials`,
/// each with no list.
pub async fn secrets(State(shared): State<Arc<Shared>>, request: Request) -> Response {
    let (parts, _body) = request.into_parts();
    let asked = match Query::<ListAsked>::try_from_uri(&parts.uri) {
        Ok(Query(asked)) => asked,
        Err(error) => {
            return failed(&lys_secrets::SecretsError::Encoding {
                context: "query",
                reason: error.body_text(),
            })
            .into_response();
        }
    };
    let who = match callers::caller(&shared, &parts, &[]).await {
        Ok(who) => who,
        Err(refusal) => return refusal.into_response(),
    };
    let routes = match shared.layout.routes() {
        Ok(routes) => routes,
        Err(error) => return failed(&error).into_response(),
    };
    let answered = on_broker(&shared, move |broker| {
        let broker: &Broker<Grants> = broker;
        let listed = match broker.secret_list(&who.asker(), asked.scope.as_deref()) {
            Ok(listed) => listed,
            Err(refusal) => return crate::manage::refused_json(refusal.status(), refusal.body()),
        };
        let entries: Vec<Value> = listed
            .into_iter()
            .map(|entry| {
                let route = routes.get(&entry.name);
                let scope = broker
                    .store()
                    .scope(&entry.name)
                    .map(|scope| scope.target());
                json!({
                    "name": entry.name,
                    "class": entry.class,
                    "owner": entry.owner,
                    "sequence": entry.sequence,
                    "scope": scope,
                    "upstream": route.map(|route| route.upstream.clone()),
                    "header": route.map(|route| route.header.clone()),
                    "purpose": entry.purpose.map(SigningPurpose::label),
                    "public_key": entry.public_key,
                })
            })
            .collect();
        Json(json!({ "scope": asked.scope, "secrets": entries })).into_response()
    })
    .await;
    match answered {
        Ok(answer) => answer,
        Err(error) => failed(&error).into_response(),
    }
}

/// The query of a handles read: the holder, percent-decoded.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct HeldAsked {
    holder: String,
}

/// The handles an identity holds, each only when the caller may discover
/// its secret: what it stands for, its uses and its end, whether and by whom
/// it was ended, and where the provider's part of its revocation stands,
/// never the handle.
pub async fn handles(State(shared): State<Arc<Shared>>, request: Request) -> Answer {
    let (parts, _body) = request.into_parts();
    let Query(asked) = Query::<HeldAsked>::try_from_uri(&parts.uri).map_err(|error| {
        failed(&lys_secrets::SecretsError::Encoding {
            context: "query",
            reason: error.body_text(),
        })
    })?;
    if asked.holder.is_empty() {
        return Err(failed(&lys_secrets::SecretsError::Encoding {
            context: "query",
            reason: "no holder named".to_owned(),
        }));
    }
    let identity = callers::caller(&shared, &parts, &[]).await?.identity;
    on_broker(&shared, move |broker| {
        let broker: &Broker<Grants> = broker;
        let held = broker
            .permissions()
            .pinned(|| broker.held_by(&identity, &asked.holder));
        let handles: Vec<Value> = held
            .into_iter()
            .map(|held| {
                let (upstream, upstream_reason) = crate::manage::upstream_label(&held.upstream);
                let ended = held.ended.map(
                    |ended| json!({ "by": ended.by, "operation": ended.operation, "root": ended.root }),
                );
                json!({
                    "id": held.id,
                    "secret": held.secret,
                    "max_uses": held.max_uses,
                    "used": held.used,
                    "not_after_ms": held.not_after_ms,
                    "dropped": held.dropped,
                    "spend_cap": held.spend_cap,
                    "settled": held.settled,
                    "parent": held.parent,
                    "ended": ended,
                    "upstream": upstream,
                    "upstream_reason": upstream_reason,
                })
            })
            .collect();
        Json(json!({ "holder": asked.holder, "handles": handles }))
    })
    .await
    .map_err(|error| failed(&error))
}

pub async fn grants(State(shared): State<Arc<Shared>>, request: Request) -> Answer {
    let identity = caller(&shared, request).await?;
    on_broker(&shared, move |broker| grants_seen(broker, &identity))
        .await
        .map_err(|error| failed(&error))?
}

/// Every grant whose secret `identity` may discover, each secret asked of
/// the permission source once and the grants file read once.
fn grants_seen(broker: &Broker<Grants>, identity: &str) -> Answer {
    let rows = broker
        .permissions()
        .list()
        .map_err(|error| failed(&error))?;
    let grants: Vec<Value> = broker.permissions().pinned(|| {
        let mut discovery = broker.discovery(identity);
        rows.into_iter()
            .filter(|(_, secret, _, _)| discovery.discovers(secret))
            .map(|(identity, secret, relation, granted_by)| {
                json!({ "identity": identity, "secret": secret, "relation": relation, "granted_by": granted_by })
            })
            .collect()
    });
    Ok(Json(json!({ "grants": grants })))
}

/// The most lines one audit read answers.
const AUDIT_WINDOW: u64 = 200;

/// The query of an audit read: the index the window ends before. Without
/// it the window is the last of the log.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AuditAsked {
    #[serde(default)]
    before: Option<u64>,
}

/// A window of the checked audit log, oldest first: the last lines of the
/// log, or the last before `before`. It reads that window only, so a read
/// costs the same however long the log is. `size` is the length of the log,
/// `from` the first index read, and `older` the `before` that reads the
/// window before this one, null when this one starts the log.
pub async fn audit(State(shared): State<Arc<Shared>>, request: Request) -> Answer {
    let (parts, _body) = request.into_parts();
    let Query(asked) = Query::<AuditAsked>::try_from_uri(&parts.uri).map_err(|error| {
        failed(&lys_secrets::SecretsError::Encoding {
            context: "query",
            reason: error.body_text(),
        })
    })?;
    let identity = callers::caller(&shared, &parts, &[]).await?.identity;
    on_broker(&shared, move |broker| {
        audit_window(broker, &identity, asked.before)
    })
    .await
    .map_err(|error| failed(&error))?
}

/// The audit window before `before` as `identity` may read it: each line
/// only when it names a secret `identity` may discover, each secret asked
/// of the permission source once and the grants file read once.
pub(crate) fn audit_window(broker: &Broker<Grants>, identity: &str, before: Option<u64>) -> Answer {
    let size = broker.audit().len();
    let lines = broker
        .audit()
        .window(before, AUDIT_WINDOW)
        .map_err(|error| failed(&error))?;
    let from = lines
        .first()
        .map_or(size.min(before.unwrap_or(size)), |first| first.index);
    let lines: Vec<Value> = broker.permissions().pinned(|| {
        let mut discovery = broker.discovery(identity);
        lines
            .into_iter()
            .filter(|recorded| {
                recorded
                    .line
                    .secret
                    .as_deref()
                    .is_some_and(|secret| discovery.discovers(secret))
            })
            .map(|recorded| {
                let line = recorded.line;
                json!({
                    "index": recorded.index,
                    "kind": line.kind.label(),
                    "at_ms": line.at_ms,
                    "handle": line.handle,
                    "identity": line.identity,
                    "secret": line.secret,
                    "operation": line.operation,
                    "uses": line.uses,
                    "outcome": line.outcome,
                })
            })
            .collect()
    });
    Ok(Json(json!({
        "verified_by": lys_secrets::to_hex(&broker.audit().verifying_key()),
        "size": size,
        "from": from,
        "older": (from > 0).then_some(from),
        "lines": lines,
    })))
}
