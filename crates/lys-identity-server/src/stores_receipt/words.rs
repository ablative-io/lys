//! The loss a refusal was caused by, said in its words (DIRECTORY-107 R4).
//!
//! While a store has a loss epoch, a refusal that loss causes carries one
//! more sentence: when Lys lost the records and what to do again. While an
//! own store is lost and not yet accepted, its routes' refusal says what the
//! receipt holds, what was found and the one command that serves what was
//! found. The sentence is added to the refusal's reason, never in place of
//! its name or status, and is absent for a store with no loss; a refusal
//! from a runner also carries `lost_at`. Nothing is read or held when no
//! store has a loss.

use std::sync::Arc;

use axum::body::Body;
use axum::extract::{Request, State};
use axum::http::header::{CONTENT_LENGTH, CONTENT_TYPE};
use axum::middleware::Next;
use axum::response::Response;
use serde_json::Value;

use super::Held;
use crate::routes::AppState;

/// The store a refusal of this name and reason was caused by, and what to
/// do again once its loss is accepted.
fn accepted_cause(refusal: &str, reason: &str) -> Option<(&'static str, &'static str)> {
    match refusal {
        "NotHeld" => Some(("grant-log", "ask for the grant again")),
        "AgentSignatureRefused" if reason.contains("the signature does not verify") => {
            Some(("certificates", "issue the agent's certificate again"))
        }
        "runner_dial_refused" if reason.contains("names no dialled runner") => {
            Some(("network", "connect this computer again"))
        }
        "MachineUnknown" => Some(("network", "connect this computer again")),
        _ => None,
    }
}

/// The own stores whose routes answer a refusal of this name while they
/// are left out.
fn own_stores(refusal: &str) -> &'static [&'static str] {
    match refusal {
        "NetworkUnavailable" | "MachineUnknown" => &["network", "network-joins"],
        "RolesUnavailable" => &["roles"],
        "ProvisioningUnavailable" => &["provisioning"],
        "MemoryUnavailable" => &["homes"],
        "RequestsUnavailable" | "McpRequestsUnavailable" => &["requests", "mcp-requests"],
        "RuntimeUnavailable" => &["runtime"],
        "ServiceAccountsUnavailable" => &["service-accounts"],
        "ReviewsUnavailable" => &["reviews"],
        "TeamsUnavailable" => &["teams"],
        "StopsUnavailable" => &["stops"],
        "BudgetsUnavailable" => &["budgets"],
        "PolicyUnavailable" => &["policies"],
        "goals_unavailable" => &["goals"],
        "words_unavailable" => &["words"],
        "variables_unavailable" => &["variables"],
        "schedules_unavailable" => &["schedules"],
        "runner_unreachable" => &[
            "runner-join",
            "runner-server-key",
            "runner-machine-key",
            "runner-owners",
        ],
        _ => &[],
    }
}

fn records(store: &str) -> &'static str {
    lys_identity::stores_receipt::store(store).map_or("records", |named| named.records)
}

/// The sentence a refusal named `refusal` with `reason` carries, and the
/// loss epoch it names, when a lost store caused it.
#[must_use]
pub fn sentence(held: &Held, refusal: &str, reason: &str) -> Option<(String, Option<String>)> {
    if let Some((store, again)) = accepted_cause(refusal, reason)
        && let Some(at) = held.lost_at(store)
    {
        let words = format!("Lys lost its {} at {at}; {again}.", records(store));
        return Some((words, Some(at.to_owned())));
    }
    let (store, loss) = own_stores(refusal)
        .iter()
        .find_map(|store| held.lost.get(*store).map(|loss| (*store, loss)))?;
    let words = format!(
        "Lys lost its {}: the receipt holds {}; Lys found {}. Run `lys identity accept-loss --store {store}` on {} to serve what was found.",
        records(store),
        loss.held,
        loss.found,
        held.computer
    );
    Some((words, None))
}

/// `answer` with the sentence its refusal's loss causes added to its
/// reason, and `lost_at` beside it; whether anything was added.
fn add(held: &Held, answer: &mut Value) -> bool {
    let refusal = answer["refusal"].as_str().unwrap_or_default().to_owned();
    let reason = answer["reason"].as_str().unwrap_or_default().to_owned();
    let Some((words, lost_at)) = sentence(held, &refusal, &reason) else {
        return false;
    };
    answer["reason"] = Value::String(format!("{reason} {words}"));
    if let Some(at) = lost_at {
        answer["lost_at"] = Value::String(at);
    }
    true
}

/// Where an answered request's refusals sit inside a success: the grant
/// batch answers each check's refusal in `results`.
const BATCH: &str = "/grants/check/batch";

/// The response `response` with the loss its refusal was caused by said in
/// its reason: the layer every route is served through. A grant batch's
/// answer carries each check's refusal inside a success, so each is said
/// there.
pub(crate) async fn said(
    State(state): State<Arc<AppState>>,
    request: Request,
    next: Next,
) -> Response {
    let batch = request.uri().path().ends_with(BATCH);
    let response = next.run(request).await;
    if state.stores.quiet()
        || (response.status().is_success() && !batch)
        || response
            .headers()
            .get(CONTENT_TYPE)
            .is_none_or(|kind| !kind.as_bytes().starts_with(b"application/json"))
    {
        return response;
    }
    let (mut parts, body) = response.into_parts();
    let Ok(bytes) = axum::body::to_bytes(body, usize::MAX).await else {
        return Response::from_parts(parts, Body::empty());
    };
    let Ok(mut answer) = serde_json::from_slice::<Value>(&bytes) else {
        return Response::from_parts(parts, Body::from(bytes));
    };
    let added = if parts.status.is_success() {
        // Every check's refusal is said, so the count runs over them all.
        answer["results"].as_array_mut().is_some_and(|results| {
            results
                .iter_mut()
                .map(|result| add(&state.stores, result))
                .filter(|said| *said)
                .count()
                > 0
        })
    } else {
        add(&state.stores, &mut answer)
    };
    if !added {
        return Response::from_parts(parts, Body::from(bytes));
    }
    let Ok(rewritten) = serde_json::to_vec(&answer) else {
        return Response::from_parts(parts, Body::from(bytes));
    };
    parts.headers.remove(CONTENT_LENGTH);
    Response::from_parts(parts, Body::from(rewritten))
}

#[path = "words_tests.rs"]
mod tests;
