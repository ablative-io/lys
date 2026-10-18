#![cfg(test)]
//! Lys reads an app's manifest itself for the approve page (DIRECTORY-102
//! R3): `GET /apps/connect/manifest?address=` answers the manifest a fixture
//! app serves, or one named reason: not answering (with the connection
//! error's words), answered with a status (the status), or not a manifest
//! (what was found). Only the administrator is answered, a redirect is
//! answered as its status and never followed, and an address that is not an
//! app's manifest address is refused before anything is read.

use axum::http::{StatusCode, header};
use axum::response::IntoResponse;
use axum::routing::get as route;
use identity_contract::apps::{Auth, BEA, get, login, ok, refused, seeded, workspace_schema};
use identity_contract::harness::{ADMINISTRATOR, Service};
use serde_json::{Value, json};

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

const MANIFEST_PATH: &str = "/identity/lys/manifest";

fn manifest(origin: &str) -> Value {
    json!({"id": "fixture_notes", "name": "Notes", "redirects": [format!("{origin}/identity/oidc/callback")],
        "post_logout_redirects": [], "profile": true, "schema": workspace_schema("fixture_notes")})
}

/// A fixture app serving its manifest at the root, a 503 under `/down`, an
/// HTML page under `/page` and a redirect to `elsewhere` under `/moved`,
/// answering its origin.
async fn fixture_app(elsewhere: String) -> Result<String> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let origin = format!("http://{}", listener.local_addr()?);
    let served = manifest(&origin);
    let router = axum::Router::new()
        .route(
            MANIFEST_PATH,
            route(move || async move { axum::Json(served) }),
        )
        .route(
            &format!("/down{MANIFEST_PATH}"),
            route(|| async { (StatusCode::SERVICE_UNAVAILABLE, "starting").into_response() }),
        )
        .route(
            &format!("/page{MANIFEST_PATH}"),
            route(|| async {
                (
                    [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
                    "<!doctype html><title>Sign in</title><p>Welcome</p>",
                )
                    .into_response()
            }),
        )
        .route(
            &format!("/moved{MANIFEST_PATH}"),
            route(move || async move {
                (StatusCode::FOUND, [(header::LOCATION, elsewhere)]).into_response()
            }),
        );
    tokio::spawn(async move { axum::serve(listener, router).await });
    Ok(origin)
}

async fn read(service: &Service, auth: Auth<'_>, address: &str) -> Result<(u16, Value)> {
    let query = reqwest::Url::parse_with_params("http://query.invalid/", [("address", address)])?;
    let query = query.query().ok_or("the address made no query")?;
    get(service, &format!("/apps/connect/manifest?{query}"), auth).await
}

#[tokio::test]
async fn lys_reads_the_manifest_and_names_each_reason_it_could_not() -> Result {
    let (service, _) = seeded().await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    let app = fixture_app(format!("{}/apps", service.base)).await?;
    let mut cases = 0;

    let answer = ok(read(
        &service,
        Auth::Cookie(&admin),
        &format!("{app}{MANIFEST_PATH}"),
    )
    .await?)?;
    assert_eq!(answer["manifest"], manifest(&app), "{answer}");
    assert_eq!(answer["origin"], app.as_str(), "{answer}");
    cases += 1;

    let down = read(
        &service,
        Auth::Cookie(&admin),
        &format!("{app}/down{MANIFEST_PATH}"),
    )
    .await?;
    refused(&down, 502, "manifest_answered_status")?;
    assert_eq!(down.1["status"], 503, "{down:?}");
    assert!(
        down.1["reason"]
            .as_str()
            .is_some_and(|reason| reason.contains("503")),
        "{down:?}"
    );
    cases += 1;

    let page = read(
        &service,
        Auth::Cookie(&admin),
        &format!("{app}/page{MANIFEST_PATH}"),
    )
    .await?;
    refused(&page, 502, "manifest_not_a_manifest")?;
    let found = page.1["reason"].as_str().ok_or("no reason")?;
    assert!(found.contains("text/html"), "{found}");
    // What was found is described, never repeated: the page's words stay with the app.
    assert!(!found.contains("Welcome"), "{found}");
    cases += 1;

    let gone = {
        let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
        format!("http://{}", listener.local_addr()?)
    };
    let silent = read(
        &service,
        Auth::Cookie(&admin),
        &format!("{gone}{MANIFEST_PATH}"),
    )
    .await?;
    refused(&silent, 502, "manifest_not_answering")?;
    let words = silent.1["reason"].as_str().ok_or("no reason")?;
    assert!(words.contains(&gone), "{words}");
    assert!(words.contains("Connection refused"), "{words}");
    cases += 1;

    // A redirect, here to Lys's own loopback address, is answered as its status and never followed.
    let moved = read(
        &service,
        Auth::Cookie(&admin),
        &format!("{app}/moved{MANIFEST_PATH}"),
    )
    .await?;
    refused(&moved, 502, "manifest_answered_status")?;
    assert_eq!(moved.1["status"], 302, "{moved:?}");
    cases += 1;

    assert_eq!(cases, 5);
    Ok(())
}

#[tokio::test]
async fn only_the_administrator_is_answered_and_only_for_a_manifest_address() -> Result {
    let (service, _) = seeded().await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    let bea = service.sign_in(login(BEA)).await?;
    let app = fixture_app(format!("{}/apps", service.base)).await?;
    let address = format!("{app}{MANIFEST_PATH}");
    refused(
        &read(&service, Auth::Cookie(&bea), &address).await?,
        403,
        "NotAdmitted",
    )?;
    let anonymous = read(&service, Auth::Nobody, &address).await?;
    assert_eq!(anonymous.0, 401, "{anonymous:?}");
    let mut refusals = 0;
    for address in [
        format!("{app}/identity/lys/connect"),
        format!("{}/apps", service.base),
        "ftp://notes.example.test/identity/lys/manifest".to_owned(),
        "not an address".to_owned(),
    ] {
        refused(
            &read(&service, Auth::Cookie(&admin), &address).await?,
            400,
            "manifest_address_refused",
        )?;
        refusals += 1;
    }
    assert_eq!(refusals, 4);
    Ok(())
}
