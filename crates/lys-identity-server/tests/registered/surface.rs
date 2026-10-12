//! Registered lifecycle regression coverage.

use std::error::Error;

use identity_contract::harness::{ADMINISTRATOR, GRANT_MODEL, Service};
use lys_identity::{Actor, AuthMethod, LoginBinding, OperationId, Profile, Provenance};
use lys_identity_server::routes::open_directory;
use lys_identity_server::session::Sessions;

type TestResult = Result<(), Box<dyn Error>>;

#[tokio::test]
async fn public_screens_do_not_exempt_unknown_or_wildcard_shaped_api_requests() -> TestResult {
    let (service, cookie) = Service::start_adjusted(
        GRANT_MODEL,
        None,
        None,
        None,
        |config| config.surface_dir = Some(config.log_dir.with_file_name("surface")),
        |config| {
            let mut directory = open_directory(config)?;
            let admin = Actor::new(
                LoginBinding::new(&config.issuer, ADMINISTRATOR)?,
                Provenance::new(AuthMethod::Oidc, 1),
            );
            let binding = LoginBinding::new(&config.issuer, "registered-screen-reader")?;
            let (person, _) = directory.register_person(
                admin.clone(),
                OperationId::generate()?,
                Profile::new("Registered screen reader")?,
                1,
            )?;
            directory.bind_login(admin, OperationId::generate()?, person, binding.clone(), 2)?;
            let sessions = Sessions::open(
                config.sessions_file.clone().ok_or("session file")?,
                config.session_seconds,
                config.secure_cookie,
            )?;
            let cookie =
                sessions.begin(Actor::new(binding, Provenance::new(AuthMethod::Oidc, 3)))?;
            let surface = config.surface_dir.as_ref().ok_or("surface")?;
            std::fs::create_dir_all(surface.join("assets"))?;
            std::fs::write(surface.join("index.html"), "public fixture screen")?;
            std::fs::write(surface.join("assets/app.js"), "public fixture asset")?;
            // If an API path falls through the screen wildcard, this file exposes the mistake.
            std::fs::create_dir_all(surface.join("api/assets"))?;
            std::fs::write(
                surface.join("api/assets/app.js"),
                "API wildcard must not serve this",
            )?;
            Ok(cookie)
        },
    )
    .await?;
    let client = reqwest::Client::new();
    for (path, expected) in [
        ("/", "public fixture screen"),
        ("/assets/app.js", "public fixture asset"),
    ] {
        let response = client
            .get(format!("{}{path}", service.base))
            .header(reqwest::header::COOKIE, &cookie)
            .send()
            .await?;
        assert_eq!(response.status().as_u16(), 200, "{path}");
        assert_eq!(response.text().await?, expected);
    }
    for path in [
        "/api/identities",
        "/api/assets/app.js",
        "/api/no-such-route",
    ] {
        let response = client
            .get(format!("{}{path}", service.base))
            .header(reqwest::header::COOKIE, &cookie)
            .send()
            .await?;
        let status = response.status().as_u16();
        let body = response.text().await?;
        assert_eq!(status, 403, "{path}: {body}");
        let refusal: serde_json::Value = serde_json::from_str(&body)?;
        assert_eq!(refusal["refusal"], "inactive", "{path}: {body}");
    }
    let response = client
        .get(format!("{}/api/me", service.base))
        .header(reqwest::header::COOKIE, &cookie)
        .send()
        .await?;
    assert_eq!(
        response.status().as_u16(),
        200,
        "own-account read through nested API"
    );
    let sessions = client
        .get(format!("{}/api/sessions", service.base))
        .header(reqwest::header::COOKIE, &cookie)
        .send()
        .await?;
    assert_eq!(sessions.status().as_u16(), 200);
    let sessions: serde_json::Value = serde_json::from_str(&sessions.text().await?)?;
    let id = sessions["sessions"][0]["id"]
        .as_str()
        .ok_or("nested own session id")?;
    let ended = client
        .post(format!("{}/api/sessions/{id}/end", service.base))
        .header(reqwest::header::COOKIE, &cookie)
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body("{}")
        .send()
        .await?;
    assert_eq!(
        ended.status().as_u16(),
        200,
        "own session end through nested API"
    );
    assert!(ended.headers().contains_key(reqwest::header::SET_COOKIE));
    let ended: serde_json::Value = serde_json::from_str(&ended.text().await?)?;
    assert_eq!(ended["ended"], id);
    let after = client
        .get(format!("{}/api/me", service.base))
        .header(reqwest::header::COOKIE, &cookie)
        .send()
        .await?;
    assert_eq!(
        after.status().as_u16(),
        401,
        "ended nested session is no longer authenticated"
    );
    Ok(())
}

/// With the screens mounted, the API answers the key set at `/api/oauth/jwks` exactly as the
/// origin answers `/oauth/jwks`: lys-pass joins `oauth/jwks` onto the API address a product
/// holds, and Cambium's key fetch met the API's 404 until it did (11 Oct 2026).
#[tokio::test]
async fn the_api_answers_the_key_set_the_origin_publishes() -> TestResult {
    let (service, _cookie) = Service::start_adjusted(
        GRANT_MODEL,
        None,
        None,
        None,
        |config| config.surface_dir = Some(config.log_dir.with_file_name("surface")),
        |config| {
            let surface = config.surface_dir.as_ref().ok_or("surface")?;
            std::fs::create_dir_all(surface)?;
            std::fs::write(surface.join("index.html"), "public fixture screen")?;
            Ok(String::new())
        },
    )
    .await?;
    let origin = reqwest::get(format!("{}/oauth/jwks", service.base)).await?;
    let (origin_status, origin_body) = (origin.status(), origin.text().await?);
    let api = reqwest::get(format!("{}/api/oauth/jwks", service.base)).await?;
    let (api_status, api_body) = (api.status(), api.text().await?);
    assert_eq!(api_status.as_u16(), 503, "{api_body}");
    let refused: serde_json::Value = serde_json::from_str(&api_body)?;
    assert_eq!(refused["refusal"], "ProviderUnavailable");
    assert_eq!(
        api_status, origin_status,
        "origin={origin_body}; api={api_body}"
    );
    assert_eq!(api_body, origin_body);
    assert!(!api_body.contains("not an API route"), "{api_body}");
    Ok(())
}
