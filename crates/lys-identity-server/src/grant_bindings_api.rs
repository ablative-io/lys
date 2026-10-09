//! A Lys-judged binding for a pass Lys did not issue (DIRECTORY-089 R2):
//! `POST /grants/bindings`.
//!
//! A registry-owned producer issues its own pass bytes and acquires their
//! binding here: it names the exact pass (the SHA-256 of its bytes), its
//! issuer, holder, audience and lifetime, and the grants its rights rest on.
//! Lys judges every named grant itself, under the grants' one hold, at the
//! revision the grants stand at once the log is settled: each must be held
//! by the named holder, on a kind the audience app owns, and stand through
//! its whole ancestry now, with the permission projection complete through
//! that revision. Any grant that fails refuses the whole binding by its own
//! name; nothing is partially bound and no right is guessed from a holder
//! string. The binding is signed by Lys's issuer key after the hold, in the
//! same shape as an issued pass's (`lys_pass::binding`), so one verifier
//! reads both. The pass's bytes are never seen or changed.
//!
//! It is open to the administrator, and to an app, through its credential,
//! for passes whose audience it is: binding grants no authority.

use std::collections::BTreeSet;
use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::extract::rejection::JsonRejection;
use axum::http::HeaderMap;
use lys_identity::grants::admission::effective;
use lys_identity::grants::{GrantError, GrantId, owner_of};
use lys_pass::GrantLog;
use lys_pass::binding::{BINDING_VERSION, BindingClaims, Dependency};
use serde::{Deserialize, Serialize};

use crate::channel_membership::served_log;
use crate::error::ServerError;
use crate::error_grant_stream::GrantStreamError;
use crate::grants::with_grants;
use crate::grants_batch::asker;
use crate::routes::{AppState, identity_id};
use crate::session::now;

/// The pass a registry-owned producer asks Lys to bind.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = GrantBindingRequest)]
pub struct BindRequest {
    /// The binding version asked for.
    pub binding: u32,
    /// The SHA-256, lowercase hex, of the pass's exact bytes.
    pub pass: String,
    /// The pass's issuer.
    pub iss: String,
    /// The pass's holder, a directory identity id.
    pub sub: String,
    /// The pass's audience app.
    pub aud: String,
    /// The pass's issue instant.
    pub iat: u64,
    /// The pass's first invalid instant.
    pub exp: u64,
    /// The grants the pass's rights rest on.
    pub grants: Vec<String>,
}

/// The signed binding and the revision it requires.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[schema(as = GrantBindingAnswer)]
pub struct BindAnswer {
    /// The signed binding, a compact JWS typed `lys-grant-binding+jwt`.
    pub grant_binding: String,
    /// The revision a consumer's grant stream must reach before admitting.
    pub revision: u64,
}

fn malformed(reason: &str) -> ServerError {
    ServerError::RequestMalformed {
        reason: reason.to_owned(),
    }
}

/// Refuse a request this route cannot judge, before the grants are held.
fn shaped(request: &BindRequest, acting_for: Option<&str>) -> Result<(), ServerError> {
    if request.binding != BINDING_VERSION {
        return Err(GrantStreamError::BindingUnsupported {
            asked: request.binding.to_string(),
            served: BINDING_VERSION,
        }
        .into());
    }
    let digest = request.pass.len() == 64
        && request
            .pass
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte));
    if !digest {
        return Err(malformed(
            "the pass is named by the lowercase hex SHA-256 of its bytes",
        ));
    }
    if request.iss.is_empty() || request.aud.is_empty() || request.exp <= request.iat {
        return Err(malformed(
            "the pass names its issuer, audience and a lifetime",
        ));
    }
    let mut seen = BTreeSet::new();
    if request.grants.iter().any(|grant| !seen.insert(grant)) {
        return Err(malformed("a grant is named twice"));
    }
    if let Some(app) = acting_for
        && app != request.aud
    {
        return Err(ServerError::NotAdmitted {
            reason: "an app binds only the passes whose audience it is",
        });
    }
    Ok(())
}

/// Bind a pass Lys did not issue to the grants its rights rest on.
pub async fn bind(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Result<Json<BindRequest>, JsonRejection>,
) -> Result<Json<BindAnswer>, ServerError> {
    let Json(request) = body.map_err(|refused| ServerError::RequestMalformed {
        reason: refused.body_text(),
    })?;
    let acting_for = asker(&state, &headers)?;
    shaped(&request, acting_for.as_deref())?;
    let holder = identity_id(&request.sub)?;
    let log: GrantLog = served_log(&state)?;
    let at = now();
    let (revision, dependencies) = with_grants(&state, |mut judged| {
        // Settled and projected through the revision taken, or refused.
        let frame = judged.grants.frame(judged.directory, None)?;
        if let Some(degraded) = frame.degradation() {
            return Err(GrantStreamError::BindingDegraded {
                refusal: ServerError::Grant(degraded.error().clone()).name(),
            }
            .into());
        }
        let revision = judged.grants.revision();
        judged.grants.frame(judged.directory, Some(revision))?;
        if let Some(held) = judged.grants.ledger().uncertain() {
            return Err(GrantError::OperationUnresolved {
                operation: held.operation.to_string(),
                grant: held.grant.to_string(),
            }
            .into());
        }
        let book = judged.grants.book();
        let mut dependencies = Vec::with_capacity(request.grants.len());
        for text in &request.grants {
            let id: GrantId = text.parse()?;
            let grant = book.grant(id).ok_or_else(|| GrantError::GrantUnknown {
                grant: text.clone(),
            })?;
            if grant.holder() != holder {
                return Err(GrantError::NotHolder {
                    caller: request.sub.clone(),
                    grant: text.clone(),
                }
                .into());
            }
            if owner_of(grant.resource().kind()) != request.aud {
                return Err(ServerError::NotAdmitted {
                    reason: "a named grant is not on a kind the audience app owns",
                });
            }
            let lineage = effective(book, judged.directory, id, at)?;
            dependencies.push(Dependency {
                grant: text.clone(),
                path: lineage.path.iter().map(ToString::to_string).collect(),
            });
        }
        if judged.grants.revision() != revision {
            return Err(GrantStreamError::BindingRevisionMoved {
                decided: revision,
                now: judged.grants.revision(),
            }
            .into());
        }
        dependencies.sort_by(|a, b| a.grant.cmp(&b.grant));
        Ok((revision, dependencies))
    })?;
    let claims = BindingClaims {
        binding: BINDING_VERSION,
        iss: request.iss,
        sub: request.sub,
        aud: request.aud,
        iat: request.iat,
        exp: request.exp,
        pass: request.pass,
        log,
        revision,
        dependencies,
    };
    Ok(Json(BindAnswer {
        grant_binding: crate::provider::signed_binding(&state, &claims)?,
        revision,
    }))
}
