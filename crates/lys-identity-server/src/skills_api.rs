//! The skills Lys keeps. The administrator keeps a skill's text under its
//! name; each text is kept by the SHA-256 of its bytes and never changed, so
//! a profile version pins the text it was recorded with and a later text
//! under the same name reaches only versions recorded after it.

use std::sync::Arc;

use axum::extract::State;
use axum::extract::rejection::JsonRejection;
use axum::http::HeaderMap;
use axum::routing::get;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::ServerError;
use crate::provisioning_api::with_provisioning;
use crate::provisioning_store::SkillText;
use crate::routes::{AppState, hex, signed_in};

/// The longest skill text kept, in bytes.
const TEXT_MAX: usize = 256 * 1024;
/// The longest skill name.
const NAME_MAX: usize = 64;

/// A skill's text to keep under its name.
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct SkillBody {
    name: String,
    text: String,
}

/// One kept skill text, without the text.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct SkillLine {
    /// Its name.
    pub name: String,
    /// The length of its text in bytes.
    pub len: u64,
    /// The SHA-256 of its text, as lowercase hex.
    pub sha256: String,
}

/// Every skill text Lys keeps, in the order kept.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct SkillsView {
    /// The kept texts.
    pub skills: Vec<SkillLine>,
}

/// The skills routes.
pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/skills", get(list).post(keep))
}

fn malformed(reason: impl Into<String>) -> ServerError {
    ServerError::RequestMalformed {
        reason: reason.into(),
    }
}

/// `name` when it is one visible path component a harness reads a skill by.
fn name(given: &str) -> Result<String, ServerError> {
    let visible = !given.is_empty()
        && given.len() <= NAME_MAX
        && !given.starts_with(['.', '-'])
        && given
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '-' | '_' | '.'));
    if !visible {
        return Err(malformed(format!(
            "a skill name is 1 to {NAME_MAX} of a-z, 0-9, -, _ and ., not beginning with . or -"
        )));
    }
    Ok(given.to_owned())
}

fn view(skills: &[SkillText]) -> SkillsView {
    SkillsView {
        skills: skills
            .iter()
            .map(|kept| SkillLine {
                name: kept.name.clone(),
                len: kept.len,
                sha256: kept.sha256.clone(),
            })
            .collect(),
    }
}

async fn list(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<SkillsView>, ServerError> {
    signed_in(&state, &headers)?;
    with_provisioning(&state, |store| Ok(Json(view(store.skills()))))
}

async fn keep(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Result<Json<SkillBody>, JsonRejection>,
) -> Result<Json<SkillsView>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    state.admission.administrator(&actor)?;
    let Json(body) = body.map_err(|refused| malformed(refused.body_text()))?;
    let name = name(&body.name)?;
    if body.text.trim().is_empty() || body.text.len() > TEXT_MAX || body.text.contains('\0') {
        return Err(malformed(format!(
            "a skill's text is not empty, holds no NUL and is at most {TEXT_MAX} bytes"
        )));
    }
    let skill = SkillText {
        name,
        len: u64::try_from(body.text.len()).map_err(|_long| malformed("the text is too long"))?,
        sha256: hex(&Sha256::digest(body.text.as_bytes())),
        text: body.text,
    };
    with_provisioning(&state, |store| {
        store.keep_skill(skill)?;
        Ok(Json(view(store.skills())))
    })
}
