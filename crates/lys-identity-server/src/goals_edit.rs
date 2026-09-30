//! A standing aim changes only under the same judgement authority as a mark.

use std::str::FromStr;
use std::sync::Arc;

use axum::Json;
use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, Uri};
use lys_identity::OperationId;
use serde::Deserialize;

use crate::error::ServerError;
use crate::goals_api::{asker, checked_words, goals, judge};
use crate::goals_types::{Change, Changed, GoalError};
use crate::goals_views::ItemView;
use crate::routes::AppState;
use crate::session::now;

/// Suspend or resume reminders without making a judgement.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[schema(as = GoalActiveBody)]
#[serde(deny_unknown_fields)]
pub struct ActiveBody {
    operation: String,
    active: bool,
}

/// Replace the words of a standing aim.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[schema(as = GoalWordsBody)]
#[serde(deny_unknown_fields)]
pub struct WordsBody {
    operation: String,
    words: String,
}

pub(crate) async fn active(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(goal): Path<String>,
    uri: Uri,
    bytes: Bytes,
) -> Result<Json<ItemView>, ServerError> {
    let body: ActiveBody = crate::signed_json::read(&state, &headers, &bytes)?;
    edit(
        &state,
        &headers,
        &uri,
        &bytes,
        (
            body.operation,
            goal,
            Change::Active {
                active: body.active,
            },
        ),
    )
}

pub(crate) async fn words(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(goal): Path<String>,
    uri: Uri,
    bytes: Bytes,
) -> Result<Json<ItemView>, ServerError> {
    let body: WordsBody = crate::signed_json::read(&state, &headers, &bytes)?;
    let words = checked_words(&body.words)?;
    edit(
        &state,
        &headers,
        &uri,
        &bytes,
        (body.operation, goal, Change::Words { words }),
    )
}

fn edit(
    state: &Arc<AppState>,
    headers: &HeaderMap,
    uri: &Uri,
    bytes: &[u8],
    (operation, goal, change): (String, String, Change),
) -> Result<Json<ItemView>, ServerError> {
    let operation = OperationId::from_str(&operation)?.to_string();
    let asker = asker(state, headers, uri, bytes)?;
    let goals = goals(state)?;
    let item = goals.with(|store| {
        store
            .item(&goal)
            .cloned()
            .ok_or_else(|| GoalError::Unknown.into())
    })?;
    judge(state, asker, &item)?;
    let changed = Changed {
        operation,
        goal,
        change,
        by: asker.to_string(),
        at: now(),
    };
    let item = goals.with(|store| store.change(changed))?;
    goals.changed.notify_one();
    Ok(Json(item.into()))
}
