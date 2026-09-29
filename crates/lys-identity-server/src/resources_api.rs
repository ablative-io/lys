//! The resources access stands on, read from the grants. A resource is
//! listed when a grant the caller may see names it, so the list is drawn
//! from the same sight as `GET /grants` and says nothing the grants do not.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Arc;

use axum::extract::State;
use axum::http::HeaderMap;
use axum::routing::get;
use axum::{Json, Router};
use serde::Serialize;

use crate::error::ServerError;
use crate::grant_sight::sees_with;
use crate::grants::with_grants;
use crate::reviews_api::stands;
use crate::routes::AppState;
use crate::session::now;

/// One resource, and the grants on it the caller may see.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct ResourceSummary {
    /// The resource's kind.
    pub kind: String,
    /// The resource's id.
    pub id: String,
    /// How many grants on it stand now: unrevoked and within their window.
    pub standing: usize,
    /// How many grants on it no longer stand.
    pub ended: usize,
    /// How many identities hold a grant on it that stands now.
    pub holders: usize,
}

/// The answer of `GET /resources`.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct ResourceList {
    /// The kinds the listed resources are of, in the order of their names.
    pub kinds: Vec<String>,
    /// Every resource a grant the caller may see names, by kind and then id.
    pub resources: Vec<ResourceSummary>,
    /// The grants' revision the answer was read at.
    pub revision: u64,
    /// When the grants were judged, in seconds since the Unix epoch.
    pub judged_at: u64,
}

/// The resource routes.
pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/resources", get(list))
}

#[derive(Default)]
struct Tally {
    standing: usize,
    ended: usize,
    holders: BTreeSet<String>,
}

async fn list(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<ResourceList>, ServerError> {
    with_grants(&state, |judged| {
        let caller = crate::service_account_grants::caller(&state, &headers, &judged)?;
        let at = now();
        let book = judged.grants.book();
        let mut known = HashMap::new();
        let mut tallies: BTreeMap<(String, String), Tally> = BTreeMap::new();
        for record in book.records() {
            if !sees_with(&judged, caller, record, &mut known) {
                continue;
            }
            let grant = record.grant();
            let resource = grant.resource();
            let tally = tallies
                .entry((resource.kind().to_owned(), resource.id().to_owned()))
                .or_default();
            if stands(book, record, at) {
                tally.standing += 1;
                tally.holders.insert(grant.holder().to_string());
            } else {
                tally.ended += 1;
            }
        }
        let kinds: BTreeSet<String> = tallies.keys().map(|(kind, _)| kind.clone()).collect();
        Ok(Json(ResourceList {
            kinds: kinds.into_iter().collect(),
            resources: tallies
                .into_iter()
                .map(|((kind, id), tally)| ResourceSummary {
                    kind,
                    id,
                    standing: tally.standing,
                    ended: tally.ended,
                    holders: tally.holders.len(),
                })
                .collect(),
            revision: judged.grants.revision(),
            judged_at: at,
        }))
    })
}
