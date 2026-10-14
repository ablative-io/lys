//! Approval belongs to the person and client, across that person's browser sessions.

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
    let person = crate::routes::with_directory(&apps.state, |directory| {
        crate::read_api::own_person(directory.projection()?, &session.actor)
    })?;
    let mut waiting = held(&apps.asking)?;
    let cause = match waiting.get(key) {
        None => Some(("asking_missing", String::new())),
        Some(asking) if asking.expires_at <= now() => {
            Some(("asking_expired", asking.person.to_string()))
        }
        Some(asking) if asking.person != person => {
            Some(("person_mismatch", asking.person.to_string()))
        }
        Some(_) => None,
    };
    if let Some((cause, prior)) = cause {
        if cause == "asking_expired" {
            waiting.remove(key);
        }
        drop(waiting);
        tracing::warn!(refusal = "CodeUnknown", cause, person = %person, asking_person = prior, "CLI or app approval refused");
        let words = match cause {
            "person_mismatch" => {
                "This approval belongs to another person. Sign in as the person who opened it, or start sign-in again from your terminal."
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
