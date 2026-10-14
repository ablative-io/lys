//! Approval failures say whether the held request or its browser session changed.

use axum::http::{StatusCode, header};
use axum::response::{Html, IntoResponse, Response};

use super::{Apps, Asking, held};
use crate::error::ServerError;
use crate::session::{SessionEntry, now};

pub(super) enum Approval {
    Ready(Asking),
    Refused(Response),
}

pub(super) fn take(
    apps: &Apps,
    key: &str,
    session: &SessionEntry,
) -> Result<Approval, ServerError> {
    let mut waiting = held(&apps.asking)?;
    let cause = match waiting.get(key) {
        None => Some(("asking_missing", String::new())),
        Some(asking) if asking.expires_at <= now() => Some((
            "asking_expired",
            asking.session_id.chars().take(8).collect(),
        )),
        Some(asking) if asking.session_id != session.id => Some((
            "session_mismatch",
            asking.session_id.chars().take(8).collect(),
        )),
        Some(_) => None,
    };
    if let Some((cause, prior)) = cause {
        if cause == "asking_expired" {
            waiting.remove(key);
        }
        drop(waiting);
        let person = crate::routes::with_directory(&apps.state, |directory| {
            crate::read_api::own_person(directory.projection()?, &session.actor)
        })?;
        let current: String = session.id.chars().take(8).collect();
        tracing::warn!(refusal = "CodeUnknown", cause, person = %person, asking_session = prior, consent_session = current, "CLI or app approval refused");
        let words = match cause {
            "session_mismatch" => {
                "The sign-in changed while this approval was open. Start sign-in again from your terminal, then approve its new page."
            }
            "asking_expired" => {
                "This approval expired. Start sign-in again from your terminal, then approve its new page."
            }
            _ => {
                "This approval is no longer held by Lys. The service may have restarted or the request was already answered. Start sign-in again from your terminal, then approve its new page."
            }
        };
        return Ok(Approval::Refused((StatusCode::BAD_REQUEST,
            [(header::CACHE_CONTROL, "no-store"), (header::CONTENT_SECURITY_POLICY, "default-src 'none'")],
            Html(format!("<!doctype html><html lang=\"en\"><title>Sign-in needs a new approval</title><h1>Sign-in needs a new approval</h1><p>{words}</p><p>Run <code>lys sign-in</code> again.</p><details><summary>Error details</summary><p>CodeUnknown: {cause}</p></details></html>"))
        ).into_response()));
    }
    waiting
        .remove(key)
        .map(Approval::Ready)
        .ok_or_else(|| ServerError::ConfigInvalid {
            reason: "the admitted approval disappeared while its lock was held".to_owned(),
        })
}
