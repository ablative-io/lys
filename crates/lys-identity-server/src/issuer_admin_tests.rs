#![cfg(test)]

use super::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn native_authority_headers_are_preserved_without_lys_or_proxy_authority() -> TestResult {
    let mut headers = HeaderMap::new();
    headers.insert(header::AUTHORIZATION, HeaderValue::from_static("Bearer native-admin"));
    headers.insert(header::COOKIE, HeaderValue::from_static("issuer_session=held; lys_directory_session=private; issuer_csrf=proof"));
    headers.insert(header::CONNECTION, HeaderValue::from_static("x-hop"));
    for name in ["x-hop", "x-lys-operator", "x-forwarded-host", "forwarded", "x-api-key"] {
        headers.insert(name, HeaderValue::from_static("untrusted"));
    }
    let carried = request_headers(&headers)?;
    assert_eq!(carried[header::AUTHORIZATION], "Bearer native-admin");
    assert_eq!(carried[header::COOKIE], "issuer_session=held; issuer_csrf=proof");
    for name in ["x-hop", "x-lys-operator", "x-forwarded-host", "forwarded", "x-api-key", "connection"] {
        assert!(!carried.contains_key(name), "{name}");
    }
    Ok(())
}

#[test]
fn native_target_keeps_the_complete_path_and_query_on_the_private_origin() -> TestResult {
    let upstream = reqwest::Url::parse("http://127.0.0.1:18080/auth/v1")?;
    let uri = "/auth/v1/oidc/callback?code=opaque%2Bcode&state=held".parse()?;
    assert_eq!(target(&upstream, &uri)?.as_str(), "http://127.0.0.1:18080/auth/v1/oidc/callback?code=opaque%2Bcode&state=held");
    assert_eq!(target(&reqwest::Url::parse("http://127.0.0.1:18080")?, &uri)?.as_str(), "http://127.0.0.1:18080/oidc/callback?code=opaque%2Bcode&state=held");
    for path in ["/api/people", "/auth/v1/providers/callback", "/auth/v1/providers/callback/", "/auth/v1/%70roviders/callback", "/auth/v1/providers%2fcallback", "/auth/v1/%2e%2e/users"] {
        assert!(target(&upstream, &path.parse()?).is_err(), "{path}");
    }
    Ok(())
}

#[test]
fn native_oidc_redirects_return_to_the_public_origin_and_keep_state() -> TestResult {
    let upstream = reqwest::Url::parse("http://127.0.0.1:18080/auth/v1")?;
    let public = reqwest::Url::parse("https://identity.example.test")?;
    let mut headers = HeaderMap::new();
    headers.insert(header::LOCATION, HeaderValue::from_static("http://127.0.0.1:18080/auth/v1/oidc/callback?code=opaque%2Bcode&state=held"));
    let carried = response_headers(&headers, &upstream, &public)?;
    assert_eq!(carried[header::LOCATION], "https://identity.example.test/auth/v1/oidc/callback?code=opaque%2Bcode&state=held");
    for external in ["https://provider.example.test/authorize?state=held", "/auth/v1/admin"] {
        headers.insert(header::LOCATION, HeaderValue::from_str(external)?);
        assert_eq!(response_headers(&headers, &upstream, &public)?[header::LOCATION], external);
    }
    Ok(())
}

#[test]
fn multiple_native_cookies_and_mfa_refusals_keep_their_headers() -> TestResult {
    let mut headers = HeaderMap::new();
    headers.append(header::SET_COOKIE, HeaderValue::from_static("issuer_session=challenge; HttpOnly"));
    headers.append(header::SET_COOKIE, HeaderValue::from_static("issuer_csrf=proof"));
    headers.insert(header::CONNECTION, HeaderValue::from_static("x-hop"));
    headers.insert("x-hop", HeaderValue::from_static("private"));
    let carried = end_to_end(&headers)?;
    assert_eq!(carried.get_all(header::SET_COOKIE).iter().count(), 2);
    assert!(!carried.contains_key("x-hop"));
    Ok(())
}
