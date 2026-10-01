//! Grant-bound header credentials and their responsible-person administration.
use crate::error::ServerError;
pub use crate::grant_token_store::Tokens;
use crate::routes::AppState;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode, header::COOKIE};
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::{Json, Router};
use lys_identity::grants::{Action, GrantBook, GrantId, Resource, admission::effective};
use lys_identity::projection::Projection;
use lys_identity::{IdentityId, PersonId};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// The sole wire header for a grant-bound credential.
pub const HEADER: &str = "lys-grant-token";

/// A named grant-token refusal, preserving underlying operational refusals.
#[derive(Debug, thiserror::Error)]
pub enum TokenError {
    /// The credential or administration id is unknown.
    #[error("GrantTokenUnknown")]
    Unknown,
    /// The credential's own expiry has passed.
    #[error("GrantTokenExpired")]
    Expired,
    /// The credential was revoked.
    #[error("GrantTokenRevoked")]
    Revoked,
    /// The requested resource/action lies outside its one bound grant.
    #[error("GrantTokenScopeMismatch")]
    Scope,
    /// Only the grant's responsible person may administer it.
    #[error("GrantTokenResponsibleRequired")]
    Responsible,
    /// A token cannot borrow a personal cookie.
    #[error("GrantTokenCookieConflict")]
    CookieConflict(String),
    /// An expiry must be future and at most 24 hours from issue.
    #[error("GrantTokenExpiryInvalid")]
    Expiry,
    /// Live and retained records have reached the install limit.
    #[error("GrantTokenStoreFull")]
    Full,
    /// Durable state is unavailable; admission fails closed.
    #[error("GrantTokenUnavailable: {0}")]
    Unavailable(String),
    /// An underlying authority or operational refusal.
    #[error(transparent)]
    Authority(#[from] ServerError),
}

impl IntoResponse for TokenError {
    fn into_response(self) -> Response {
        if let Self::Authority(error) = self {
            return error.into_response();
        }
        let name = match &self {
            Self::Unknown => "GrantTokenUnknown",
            Self::Expired => "GrantTokenExpired",
            Self::Revoked => "GrantTokenRevoked",
            Self::Scope => "GrantTokenScopeMismatch",
            Self::Responsible => "GrantTokenResponsibleRequired",
            Self::CookieConflict(_) => "GrantTokenCookieConflict",
            Self::Expiry => "GrantTokenExpiryInvalid",
            Self::Full => "GrantTokenStoreFull",
            Self::Unavailable(_) => "GrantTokenUnavailable",
            Self::Authority(_) => "GrantTokenAuthorityRefused",
        };
        let status = match &self {
            Self::Unavailable(_) => StatusCode::SERVICE_UNAVAILABLE,
            Self::Full => StatusCode::CONFLICT,
            Self::Expiry | Self::CookieConflict(_) => StatusCode::BAD_REQUEST,
            Self::Responsible | Self::Scope => StatusCode::FORBIDDEN,
            _ => StatusCode::UNAUTHORIZED,
        };
        (
            status,
            Json(serde_json::json!({"refusal":name,"reason":self.to_string(),"fields":{}})),
        )
            .into_response()
    }
}

/// Required, explicitly bounded expiry; no default lifetime.
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct IssueBody {
    /// Seconds since the Unix epoch.
    pub expires_at: u64,
}

/// The secret is returned once; only its digest is stored.
#[derive(Serialize, utoipa::ToSchema)]
pub struct Issued {
    /// Public id used to revoke this one credential.
    pub id: String,
    /// The opaque header value; never written to the token table.
    pub token: String,
    /// Seconds since the Unix epoch.
    pub expires_at: u64,
}

/// An explicit durable revocation answer.
#[derive(Serialize, utoipa::ToSchema)]
pub struct Revoked {
    /// The credential id.
    pub id: String,
    /// Revocation was durably recorded.
    pub revoked: bool,
}

/// Read exactly one token header and refuse a personal session alongside it.
pub fn header(headers: &HeaderMap) -> Result<&str, TokenError> {
    if headers.get_all(HEADER).iter().count() != 1 {
        return Err(TokenError::Unknown);
    }
    for value in headers.get_all(COOKIE) {
        let text = value.to_str().map_err(|error| {
            TokenError::CookieConflict(format!("cookie header is not text: {error}"))
        })?;
        if text.split(';').any(|part| {
            part.trim()
                .split_once('=')
                .is_some_and(|(name, _)| name.trim() == crate::session::COOKIE)
        }) {
            return Err(TokenError::CookieConflict(
                "a grant token carries a personal session cookie".to_owned(),
            ));
        }
    }
    headers
        .get(HEADER)
        .and_then(|v| v.to_str().ok())
        .ok_or(TokenError::Unknown)
}

pub(crate) fn responsible(caller: IdentityId, person: PersonId) -> Result<(), TokenError> {
    if caller == IdentityId::Person(person) {
        Ok(())
    } else {
        Err(TokenError::Responsible)
    }
}

pub(crate) fn bound(
    book: &GrantBook,
    directory: &Projection,
    id: GrantId,
    resource: &Resource,
    action: &Action,
    at: u64,
) -> Result<(), TokenError> {
    effective(book, directory, id, at).map_err(ServerError::from)?;
    let grant = book.grant(id).ok_or(TokenError::Unknown)?;
    if grant.resource() != resource || !grant.actions().contains(action) {
        return Err(TokenError::Scope);
    }
    Ok(())
}

/// Validate a token against its bound grant in current cached projections.
/// The consuming route supplies its declared scope and judges its own holding.
pub fn validate_with(
    tokens: &Tokens,
    book: &GrantBook,
    directory: &Projection,
    token: &str,
    resource: &Resource,
    action: &Action,
    at: u64,
) -> Result<GrantId, TokenError> {
    let id = tokens
        .lookup(token, at)?
        .grant
        .parse::<GrantId>()
        .map_err(|error| TokenError::Unavailable(format!("invalid cached grant id: {error}")))?;
    bound(book, directory, id, resource, action, at)?;
    Ok(id)
}

/// Validate only the token's bound grant against caller-supplied declared scope.
/// This answers grant admission; the consuming route still judges its holding.
pub fn validate(
    state: &AppState,
    token: &str,
    resource: &Resource,
    action: &Action,
) -> Result<GrantId, TokenError> {
    let at = crate::session::now();
    crate::grants::with_grants(state, |judged| {
        Ok((|| {
            let tokens = state
                .grant_tokens
                .lock()
                .map_err(|e| TokenError::Unavailable(format!("grant token lock poisoned: {e}")))?;
            judged
                .apps
                .admit_kind(None, resource.kind())
                .map_err(ServerError::from)?;
            judged
                .apps
                .admit_action(resource.kind(), action.as_str())
                .map_err(ServerError::from)?;
            validate_with(
                &tokens,
                judged.grants.book(),
                judged.directory,
                token,
                resource,
                action,
                at,
            )
        })())
    })?
}

/// Personal administration routes; token admission belongs to the consumer.
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/grants/{id}/tokens", post(issue))
        .route("/grants/{id}/tokens/{token_id}/revoke", post(revoke))
}

async fn issue(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<IssueBody>,
) -> Result<Json<Issued>, TokenError> {
    if headers.contains_key(HEADER) {
        return Err(TokenError::Responsible);
    }
    let actor = state
        .sessions
        .actor(crate::routes::cookie_header(&headers))?;
    let id = crate::grant_contract::grant_id(&id)?;
    let at = crate::session::now();
    crate::grants::with_grants(&state, |judged| {
        Ok((|| {
            let caller = crate::caller_admission::active_caller(judged.directory, &actor)?;
            let grant = judged.grants.book().grant(id).ok_or(TokenError::Unknown)?;
            responsible(caller, grant.responsible())?;
            let lineage = effective(judged.grants.book(), judged.directory, id, at)
                .map_err(ServerError::from)?;
            if lineage.ends.is_some_and(|(end, _)| body.expires_at > end) {
                return Err(TokenError::Expiry);
            }
            let mut tokens = state
                .grant_tokens
                .lock()
                .map_err(|e| TokenError::Unavailable(format!("grant token lock poisoned: {e}")))?;
            tokens.issue(id, body.expires_at, at).map(Json)
        })())
    })?
}

async fn revoke(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((id, token_id)): Path<(String, String)>,
) -> Result<Json<Revoked>, TokenError> {
    if headers.contains_key(HEADER) {
        return Err(TokenError::Responsible);
    }
    let actor = state
        .sessions
        .actor(crate::routes::cookie_header(&headers))?;
    let id = crate::grant_contract::grant_id(&id)?;
    crate::grants::with_grants(&state, |judged| {
        Ok((|| {
            let caller = crate::caller_admission::active_caller(judged.directory, &actor)?;
            let grant = judged.grants.book().grant(id).ok_or(TokenError::Unknown)?;
            responsible(caller, grant.responsible())?;
            let mut tokens = state
                .grant_tokens
                .lock()
                .map_err(|e| TokenError::Unavailable(format!("grant token lock poisoned: {e}")))?;
            tokens.revoke(id, &token_id)?;
            Ok(Json(Revoked {
                id: token_id,
                revoked: true,
            }))
        })())
    })?
}
