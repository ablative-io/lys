//! Bounded provider refusals carry contact context only to the initiating browser.

use super::failed;
use crate::error::ServerError;
use serde_json::Value;

pub(super) struct ProviderRefusal {
    pub(super) error: ServerError,
    pub(super) email: Option<String>,
}

impl From<ServerError> for ProviderRefusal {
    fn from(error: ServerError) -> Self {
        Self { error, email: None }
    }
}

fn checked_email(text: &str) -> Option<String> {
    crate::accounts::check_email(text).ok().map(str::to_owned)
}

fn admission_refusal(body: &[u8]) -> Option<ProviderRefusal> {
    let value: Value = serde_json::from_slice(body).ok()?;
    let error = value.get("error")?.as_str()?;
    let message = value.get("message").and_then(Value::as_str);
    let unlinked = message
        .and_then(|text| text.strip_prefix("User with email '"))
        .and_then(|text| text.strip_suffix("' already exists but is not linked to this provider."));
    let name = match (error, message) {
        ("Disabled", Some("User is not enabled")) => "AccountDisabled",
        ("NotFound", Some("User not found")) | ("BadRequest", Some("Invalid E-Mail")) => {
            "NotAdmittedYet"
        }
        ("Forbidden", _) if unlinked.is_some() => "NotAdmittedYet",
        _ => return None,
    };
    let email = value
        .get("email")
        .and_then(Value::as_str)
        .and_then(checked_email)
        .or_else(|| unlinked.and_then(checked_email));
    Some(ProviderRefusal {
        error: ServerError::Withheld {
            refusal: name.to_owned(),
        },
        email,
    })
}

/// Only recognized refusal words enter the log; upstream bodies can carry
/// credentials, personal details and text that would forge another log line.
fn callback_summary(body: &[u8]) -> String {
    let Ok(value) = serde_json::from_slice::<Value>(body) else {
        return format!("body is not a JSON refusal ({} bytes)", body.len());
    };
    let error = match value.get("error").and_then(Value::as_str) {
        Some(word)
            if matches!(
                word,
                "BadRequest"
                    | "Blocked"
                    | "Connection"
                    | "CSRFTokenError"
                    | "Database"
                    | "DatabaseIo"
                    | "Disabled"
                    | "Encryption"
                    | "Forbidden"
                    | "Internal"
                    | "invalid_grant"
                    | "invalid_target"
                    | "JwtToken"
                    | "JoseError"
                    | "MfaRequired"
                    | "NoSession"
                    | "NotFound"
                    | "PasswordExpired"
                    | "PasswordRefresh"
                    | "PreconditionRequired"
                    | "Scim"
                    | "SessionExpired"
                    | "SessionTimeout"
                    | "Timeout"
                    | "Unauthorized"
                    | "NotAccepted"
            ) =>
        {
            word
        }
        _ => "unrecognized",
    };
    let message = match value.get("message").and_then(Value::as_str) {
        Some(text)
            if text.starts_with("User with email '")
                && text.ends_with("' already exists but is not linked to this provider.") =>
        {
            "existing account is not linked to this provider"
        }
        Some(
            text @ ("User not found"
            | "Invalid value for the Upstream User ID"
            | "Cannot find any user id in the response"
            | "bad provider_id in link cookie"
            | "bad user_id in link cookie"
            | "Invalid E-Mail"
            | "No `email` in ID token claims. This is a mandatory claim"
            | "Callback Code not found - timeout reached?"
            | "Neither `access_token` nor `id_token` existed"),
        ) => text,
        Some(text)
            if text.starts_with("HTTP ")
                && text.contains(" during POST ")
                && text.contains(" for upstream auth provider '") =>
        {
            "upstream token endpoint refused"
        }
        Some(_) => "unrecognized message redacted",
        None => "no string message",
    };
    format!(
        "body error={error}; message={message} ({} bytes)",
        body.len()
    )
}

/// A callback refusal keeps its status even when its body cannot be read.
pub(super) async fn callback_refusal(mut answer: reqwest::Response) -> ProviderRefusal {
    const MOST: usize = 4096;
    let status = answer.status().as_u16();
    let mut body = Vec::new();
    loop {
        match answer.chunk().await {
            Ok(Some(chunk)) => {
                if chunk.len() > MOST - body.len() {
                    return failed(format!(
                        "the provider callback answered {status}; body exceeds {MOST} bytes"
                    ))
                    .into();
                }
                body.extend_from_slice(&chunk);
            }
            Ok(None) => break,
            Err(error) => {
                return failed(format!(
                    "the provider callback answered {status}; body read failed: {}",
                    super::super::unreached(error)
                ))
                .into();
            }
        }
    }
    if let Some(refused) = admission_refusal(&body) {
        return refused;
    }
    failed(format!(
        "the provider callback answered {status}; {}",
        callback_summary(&body)
    ))
    .into()
}
