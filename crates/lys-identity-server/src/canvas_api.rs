//! A person's canvas: where they put each agent's window on the Running
//! page, the boxes, notes and lines they drew there, and every layout they
//! saved by name (Tom, 4 October 2026: "This is on a per user thing…
//! every layout has to be saveable"). It is kept for the person, so it is
//! the same on any computer they sign in at, and it is theirs alone: each
//! route reads and changes only the caller's own.
//!
//! - `GET /canvas`: the caller's arrangement and saved layouts
//! - `PUT /canvas`: keep the arrangement being worked in
//! - `POST /canvas/layouts`: save an arrangement under a name, over any
//!   layout of that name
//! - `POST /canvas/layouts/remove`: remove the layout of a name

use std::sync::Arc;

use axum::extract::State;
use axum::extract::rejection::JsonRejection;
use axum::http::HeaderMap;
use axum::routing::{get, post};
use axum::{Json, Router};
use lys_identity::PersonId;
use serde::{Deserialize, Serialize};

use crate::canvas_store::{Arrangement, Canvas, Layout, layout_name};
use crate::error::ServerError;
use crate::error_canvas::CanvasError;
use crate::routes::{AppState, signed_in, with_directory};
use crate::session::now;

/// The arrangement to keep.
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = CanvasKeepBody)]
pub(crate) struct KeepBody {
    arrangement: Arrangement,
}

/// A layout to save by name.
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = CanvasSaveBody)]
pub(crate) struct SaveBody {
    name: String,
    arrangement: Arrangement,
}

/// The layout to remove.
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = CanvasRemoveBody)]
pub(crate) struct RemoveBody {
    name: String,
}

/// The caller's saved layouts after a change to them.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[schema(as = CanvasLayoutsView)]
pub struct LayoutsView {
    /// Every layout saved by name, in the order first saved.
    pub layouts: Vec<Layout>,
}

/// The canvas routes.
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/canvas", get(read).put(keep))
        .route("/canvas/layouts", post(save))
        .route("/canvas/layouts/remove", post(remove))
}

/// The canvas routes' schemas.
pub(crate) fn types(api: &mut lys_openapi::Api) -> Vec<crate::openapi_types::Entry> {
    let canvas = api.schema::<Canvas>();
    let layouts = api.schema::<LayoutsView>();
    vec![
        (
            crate::openapi_table::GET,
            "/canvas",
            None,
            Some(canvas.clone()),
        ),
        (
            crate::openapi_table::PUT,
            "/canvas",
            Some(api.schema::<KeepBody>()),
            Some(canvas),
        ),
        (
            crate::openapi_table::POST,
            "/canvas/layouts",
            Some(api.schema::<SaveBody>()),
            Some(layouts.clone()),
        ),
        (
            crate::openapi_table::POST,
            "/canvas/layouts/remove",
            Some(api.schema::<RemoveBody>()),
            Some(layouts),
        ),
    ]
}

/// The person whose canvas the caller reads and changes: their own. An
/// agent or a service account has none.
fn owner(state: &AppState, headers: &HeaderMap) -> Result<PersonId, ServerError> {
    let actor = signed_in(state, headers)?;
    with_directory(state, |directory| {
        crate::caller_admission::own_account_person(directory.projection()?, &actor)
    })
}

fn body<T>(given: Result<Json<T>, JsonRejection>) -> Result<T, ServerError> {
    given
        .map(|Json(body)| body)
        .map_err(|refused| ServerError::RequestMalformed {
            reason: refused.body_text(),
        })
}

async fn read(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Canvas>, ServerError> {
    let person = owner(&state, &headers)?;
    Ok(Json(state.canvas.read(person)?))
}

async fn keep(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    given: Result<Json<KeepBody>, JsonRejection>,
) -> Result<Json<Canvas>, ServerError> {
    let given = body(given)?;
    let person = owner(&state, &headers)?;
    let kept = state.canvas.change(person, |canvas| {
        canvas.arrangement = Some(given.arrangement);
        Ok(())
    })?;
    Ok(Json(kept))
}

async fn save(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    given: Result<Json<SaveBody>, JsonRejection>,
) -> Result<Json<LayoutsView>, ServerError> {
    let given = body(given)?;
    let name = layout_name(&given.name)?;
    let person = owner(&state, &headers)?;
    let kept = state.canvas.change(person, |canvas| {
        let layout = Layout {
            name,
            saved_at: now(),
            arrangement: given.arrangement,
        };
        // Saved over, a layout keeps its place in the list; a new one goes last.
        match canvas
            .layouts
            .iter_mut()
            .find(|held| held.name == layout.name)
        {
            Some(held) => *held = layout,
            None => canvas.layouts.push(layout),
        }
        Ok(())
    })?;
    Ok(Json(LayoutsView {
        layouts: kept.layouts,
    }))
}

async fn remove(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    given: Result<Json<RemoveBody>, JsonRejection>,
) -> Result<Json<LayoutsView>, ServerError> {
    let given = body(given)?;
    let name = layout_name(&given.name)?;
    let person = owner(&state, &headers)?;
    let kept = state.canvas.change(person, |canvas| {
        let held = canvas.layouts.len();
        canvas.layouts.retain(|layout| layout.name != name);
        if canvas.layouts.len() == held {
            return Err(ServerError::Canvas(CanvasError::Refused {
                words: format!("no layout is saved as `{name}`"),
            }));
        }
        Ok(())
    })?;
    Ok(Json(LayoutsView {
        layouts: kept.layouts,
    }))
}
