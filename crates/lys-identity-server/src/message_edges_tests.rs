//! The message bridge preserves caller identity, visibility, credentials and explicit addressing.

use super::{Binding, EdgeError, MessageEdge, Settings, UpstreamPage, cookie, map_page};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use std::error::Error;

const PERSON: &str = "person-11111111111111111111111111111111";
const AGENT: &str = "agent-22222222222222222222222222222222";

fn settings() -> Settings {
    Settings {
        url: "http://localhost:6010/".to_owned(),
        cookie: "cambium_session".to_owned(),
        bindings: vec![
            Binding {
                participant: "alice".to_owned(),
                identity: PERSON.to_owned(),
            },
            Binding {
                participant: "scribe".to_owned(),
                identity: AGENT.to_owned(),
            },
        ],
    }
}

fn page(caller: &str) -> UpstreamPage {
    UpstreamPage {
        caller: caller.to_owned(),
        places: Vec::new(),
        roots: vec!["root".to_owned()],
        next: Some("tail".to_owned()),
        messages: vec![MessageEdge {
            message: "post-1".to_owned(),
            stream: "dm-1".to_owned(),
            source: "alice".to_owned(),
            recipients: vec!["scribe".to_owned()],
            addressing: "direct".to_owned(),
            at: 1,
        }],
    }
}

#[test]
fn maps_only_explicit_visible_identities() -> Result<(), Box<dyn Error>> {
    let bridge = settings().bridge()?;
    let visible = [PERSON.to_owned(), AGENT.to_owned()].into_iter().collect();
    let mapped = map_page(page("alice"), &bridge, PERSON, &visible)?;
    let edge = mapped.messages.first().ok_or("message absent")?;
    assert_eq!(edge.source, PERSON);
    assert_eq!(edge.recipients, [AGENT]);
    assert_eq!(edge.message, "post-1");
    assert_eq!(mapped.next.as_deref(), Some("tail"));
    assert_eq!(mapped.roots, ["root"]);
    let personal = [PERSON.to_owned()].into_iter().collect();
    assert!(
        map_page(page("alice"), &bridge, PERSON, &personal)?
            .messages
            .is_empty()
    );
    Ok(())
}

#[test]
fn mixed_logins_refuse_instead_of_borrowing_another_callers_visibility()
-> Result<(), Box<dyn Error>> {
    let bridge = settings().bridge()?;
    let visible = [PERSON.to_owned(), AGENT.to_owned()].into_iter().collect();
    assert!(matches!(
        map_page(page("scribe"), &bridge, PERSON, &visible),
        Err(EdgeError::Refused {
            status: StatusCode::FORBIDDEN,
            ..
        })
    ));
    assert!(matches!(
        map_page(page("unbound"), &bridge, PERSON, &visible),
        Err(EdgeError::Refused {
            status: StatusCode::FORBIDDEN,
            ..
        })
    ));
    Ok(())
}

#[test]
fn missing_binding_is_named_without_guessing_by_display_name() -> Result<(), Box<dyn Error>> {
    let bridge = settings().bridge()?;
    let visible = [PERSON.to_owned(), AGENT.to_owned()].into_iter().collect();
    let mut input = page("alice");
    input
        .messages
        .first_mut()
        .ok_or("message absent")?
        .recipients = vec!["not-bound".to_owned()];
    let mapped = map_page(input, &bridge, PERSON, &visible)?;
    assert!(mapped.messages.is_empty());
    assert_eq!(mapped.unmapped, ["not-bound"]);
    Ok(())
}

#[test]
fn duplicate_binding_and_untrusted_endpoint_refuse() {
    for url in [
        "http://example.com/",
        "https://user:secret@example.com/",
        "https://example.com/?key=value",
        "https://example.com/#fragment",
    ] {
        let mut candidate = settings();
        candidate.url = url.to_owned();
        assert!(candidate.validate().is_err(), "{url}");
    }
    for binding in [
        Binding {
            participant: "alice".to_owned(),
            identity: AGENT.to_owned(),
        },
        Binding {
            participant: "other".to_owned(),
            identity: PERSON.to_owned(),
        },
    ] {
        let mut candidate = settings();
        candidate.bindings.push(binding);
        assert!(candidate.validate().is_err());
    }
}

#[test]
fn only_one_cambium_cookie_is_forwarded() -> Result<(), Box<dyn Error>> {
    let mut headers = HeaderMap::new();
    headers.insert(
        header::COOKIE,
        HeaderValue::from_static(
            "lys_session=lys-secret; cambium_session=cambium-secret; unrelated=private",
        ),
    );
    assert_eq!(
        cookie(&headers, "cambium_session")?,
        "cambium_session=cambium-secret"
    );
    headers.append(
        header::COOKIE,
        HeaderValue::from_static("cambium_session=second"),
    );
    assert!(cookie(&headers, "cambium_session").is_err());
    headers.clear();
    assert!(cookie(&headers, "cambium_session").is_err());
    Ok(())
}

#[tokio::test]
async fn named_refusal_has_the_surface_contract() -> Result<(), Box<dyn Error>> {
    use axum::response::IntoResponse as _;
    let response = super::unavailable("identity bindings are not configured").into_response();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await?;
    let body: serde_json::Value = serde_json::from_slice(&bytes)?;
    assert_eq!(body["refusal"], "MessageEdgesUnavailable");
    assert_eq!(body["reason"], "identity bindings are not configured");
    assert_eq!(body["fields"], serde_json::json!({}));
    Ok(())
}

#[test]
fn openapi_names_query_members_without_a_get_body() -> Result<(), Box<dyn Error>> {
    let document = crate::openapi::document()?;
    let operation = document
        .pointer("/paths/~1runtime~1message-edges/get")
        .ok_or("operation missing")?;
    assert!(operation.get("requestBody").is_none());
    let parameters = operation["parameters"]
        .as_array()
        .ok_or("parameters absent")?;
    let names: std::collections::BTreeSet<&str> = parameters
        .iter()
        .filter_map(|parameter| parameter["name"].as_str())
        .collect();
    assert_eq!(names, ["cursor", "root", "stream"].into_iter().collect());
    assert!(
        parameters
            .iter()
            .all(|parameter| parameter["in"] == "query")
    );
    Ok(())
}
