#![cfg(test)]

use super::{pass_credentials, pass_method};
use axum::http::{HeaderMap, Method, header};
use std::error::Error;

fn refuses(name: axum::http::HeaderName) -> Result<(), Box<dyn Error>> {
    let mut headers = HeaderMap::new();
    headers.insert(crate::agent_pass::HEADER, "a".repeat(43).parse()?);
    headers.insert(name, "other".parse()?);
    let error = pass_credentials(&headers)
        .err()
        .ok_or("mixed credentials accepted")?;
    assert!(matches!(
        error,
        crate::error::ServerError::AgentPassRefused { .. }
    ));
    assert_eq!(error.name(), "AgentPassRefused");
    Ok(())
}

#[test]
fn a_pass_refuses_a_cookie_at_ingress() -> Result<(), Box<dyn Error>> {
    refuses(header::COOKIE)
}
#[test]
fn a_pass_refuses_a_signature_at_ingress() -> Result<(), Box<dyn Error>> {
    refuses(axum::http::HeaderName::from_static(
        crate::agent_signature::HEADER,
    ))
}
#[test]
fn a_pass_refuses_authorization_at_ingress() -> Result<(), Box<dyn Error>> {
    refuses(header::AUTHORIZATION)
}
#[test]
fn a_pass_head_is_judged_as_get() {
    assert_eq!(pass_method(&Method::HEAD), "GET");
    assert_eq!(pass_method(&Method::GET), "GET");
    assert_eq!(pass_method(&Method::PUT), "PUT");
    assert!(pass_credentials(&HeaderMap::new()).is_ok());
}
