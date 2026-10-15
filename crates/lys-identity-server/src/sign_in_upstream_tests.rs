//! Provider continuations and bounded issuer refusals.

use std::error::Error;

use axum::http::{StatusCode, header};

use super::{Continuation, callback_refusal, continuation_target, to_sign_in};
use crate::error::ServerError;

/// Where `response` sends the browser.
fn location(response: &axum::response::Response) -> Result<String, Box<dyn Error>> {
    Ok(response
        .headers()
        .get(header::LOCATION)
        .ok_or("the refusal names no location")?
        .to_str()?
        .to_owned())
}

#[test]
fn a_refused_provider_sign_in_keeps_its_continuation() -> Result<(), Box<dyn Error>> {
    let target = "/oauth/authorize?client_id=notes&redirect_uri=https%3A%2F%2Fapp.example.test%2Fcallback&state=s1";
    let Some(kept) = continuation_target(Some(target.to_owned()))? else {
        return Err("an authorize request on this origin is a continuation".into());
    };
    let response = to_sign_in(&ServerError::SignInStateUnknown, Some(&kept));
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    let sent = location(&response)?;
    let carried = sent
        .strip_prefix("/#/sign-in?refused=SignInStateUnknown&continue=")
        .ok_or_else(|| format!("the refusal keeps no continuation: {sent}"))?;
    assert!(
        !carried.contains(['&', '#', '?', '/']),
        "the continuation is one encoded value: {carried}"
    );
    let read_back = reqwest::Url::parse(&format!("http://lys.test/?continue={carried}"))?;
    let decoded = read_back
        .query_pairs()
        .find(|(name, _)| name == "continue")
        .map(|(_, value)| value.into_owned());
    assert_eq!(decoded.as_deref(), Some(target));
    let without = to_sign_in(&ServerError::SignInStateUnknown, None);
    assert_eq!(location(&without)?, "/#/sign-in?refused=SignInStateUnknown");
    Ok(())
}

#[test]
fn an_unacceptable_continuation_is_refused_by_name_and_never_carried() -> Result<(), Box<dyn Error>>
{
    let long = format!("/oauth/authorize?{}", "a".repeat(8192));
    for target in [
        "https://elsewhere.example/oauth/authorize?client_id=notes",
        "//elsewhere.example/oauth/authorize?client_id=notes",
        "/oauth/authorize",
        "/#/me",
        "/oauth/authorize?client_id=notes#elsewhere",
        "/oauth/authorize?client_id=notes\\elsewhere",
        "/oauth/authorize?client_id=no tes",
        "/oauth/authorize?client_id=notes\r\nLocation:%20https://elsewhere.example",
        "/oauth/authorize?client_id=n\u{f6}tes",
        long.as_str(),
    ] {
        let Err(error) = continuation_target(Some(target.to_owned())) else {
            return Err(format!("{target:?} was accepted").into());
        };
        assert_eq!(error.name(), "SignInFailed", "{target:?}");
        let response = to_sign_in(&error, None);
        let location = location(&response)?;
        assert_eq!(location, "/#/sign-in?refused=SignInFailed", "{target:?}");
    }
    assert!(continuation_target(None)?.is_none());
    let Continuation(kept) = Continuation::accepted("/oauth/mcp/authorize?client_id=a".to_owned())?;
    assert_eq!(kept, "/oauth/mcp/authorize?client_id=a");
    Ok(())
}

fn answer(status: u16, body: String) -> Result<reqwest::Response, axum::http::Error> {
    Ok(reqwest::Response::from(
        axum::http::Response::builder()
            .status(status)
            .body(reqwest::Body::from(body))?,
    ))
}

#[tokio::test]
async fn an_existing_unlinked_account_is_named_without_logging_personal_details()
-> Result<(), Box<dyn Error>> {
    let body = serde_json::json!({
        "error": "Forbidden",
        "message": "User with email 'private-address' already exists but is not linked to this provider.",
        "access_token": "private-token",
    }).to_string();
    let error = callback_refusal(answer(403, body)?).await;
    assert_eq!(error.error.name(), "NotAdmittedYet");
    let words = error.error.to_string();
    assert!(error.email.is_none());
    assert!(!words.contains("private-address"), "{words}");
    assert!(!words.contains("private-token"), "{words}");
    Ok(())
}

#[tokio::test]
async fn admission_refusals_name_the_next_act_and_carry_only_the_email()
-> Result<(), Box<dyn Error>> {
    for (kind, message, name) in [
        ("NotFound", "User not found", "NotAdmittedYet"),
        ("Disabled", "User is not enabled", "AccountDisabled"),
        ("BadRequest", "Invalid E-Mail", "NotAdmittedYet"),
    ] {
        let body = serde_json::json!({"error": kind, "message": message,
            "email": "member@example.test", "access_token": "private-token"})
        .to_string();
        let refused = callback_refusal(answer(403, body)?).await;
        assert_eq!(refused.error.name(), name);
        assert_eq!(refused.email.as_deref(), Some("member@example.test"));
        assert!(!refused.error.to_string().contains("member@example.test"));
        let target = super::refused_to_sign_in(&refused.error, None, refused.email.as_deref());
        assert_eq!(
            location(&target)?,
            format!("/#/sign-in?refused={name}&email=member%40example.test")
        );
    }
    Ok(())
}

#[tokio::test]
async fn unknown_messages_and_error_values_never_reach_the_log() -> Result<(), Box<dyn Error>> {
    for body in [
        r#"{"error":"private-token","message":"private-token\nforged log line"}"#,
        r#"{"error":"Forbidden","message":"private-token"}"#,
        r#"{"error":{"Forbidden":"private-token"},"message":"private-token"}"#,
        "private-token\nforged log line",
    ] {
        let error = callback_refusal(answer(400, body.to_owned())?).await;
        let words = error.error.to_string();
        assert!(words.contains("400"), "{words}");
        assert!(!words.contains("private-token"), "{words}");
        assert!(!words.contains('\n'), "{words}");
    }
    Ok(())
}

#[tokio::test]
async fn each_callback_refusal_keeps_its_actual_status() -> Result<(), Box<dyn Error>> {
    for status in [400, 401, 403, 404, 409, 422] {
        let error = callback_refusal(answer(
            status,
            r#"{"error":"Forbidden","message":"User not found"}"#.to_owned(),
        )?)
        .await;
        let words = error.error.to_string();
        assert_eq!(error.error.name(), "SignInFailed");
        assert!(words.contains(&status.to_string()), "{words}");
        assert!(words.contains("User not found"), "{words}");
    }
    Ok(())
}

#[tokio::test]
async fn oversized_callback_bodies_are_named_without_logging_their_bytes()
-> Result<(), Box<dyn Error>> {
    let error = callback_refusal(answer(403, "private-token".repeat(1024))?).await;
    let words = error.error.to_string();
    assert!(words.contains("403"), "{words}");
    assert!(words.contains("body exceeds 4096 bytes"), "{words}");
    assert!(!words.contains("private-token"), "{words}");
    Ok(())
}
