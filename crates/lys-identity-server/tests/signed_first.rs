#![cfg(test)]
//! Every route the table names with no public door answers a caller that
//! brings no session, no credential and no signature that it is not signed
//! in, even when the body, query or path is malformed: such a caller is
//! refused before its request is read, so it is never told how the request
//! failed to parse. The walk is made against the routes as served bare, as
//! served under `/api` beside the screens, and by HEAD on every GET route.

use identity_contract::apps::{Auth, TestResult, send};
use identity_contract::harness::{GRANT_MODEL, Service};
use lys_identity_server::openapi::api;
use lys_openapi::Method;

/// `path` with each parameter filled.
fn filled(path: &str) -> String {
    path.split('/')
        .map(|segment| {
            if segment.starts_with('{') {
                "x"
            } else {
                segment
            }
        })
        .collect::<Vec<_>>()
        .join("/")
}

/// The screens' page, as the walk under `/api` serves it.
const PAGE: &str = "<title>Lys</title>";

/// Every route without a public door, asked anonymously under `prefix` with
/// a malformed body. The provider's `/oauth/userinfo` route stays at the
/// origin root beside the screens, as its discovery document advertises;
/// each one that does not answer `NotSignedIn`, named.
async fn walk(
    service: &Service,
    prefix: &str,
) -> Result<(usize, Vec<String>), Box<dyn std::error::Error>> {
    let malformed = serde_json::json!([["not", "a", "body"]]);
    let mut wrong = Vec::new();
    let mut asked = 0;
    for route in api().routes() {
        if route.auth.contains(&lys_openapi::Auth::Public) {
            continue;
        }
        let method = match route.method {
            Method::Get => reqwest::Method::GET,
            Method::Post => reqwest::Method::POST,
            Method::Put => reqwest::Method::PUT,
        };
        let body = (method != reqwest::Method::GET).then_some(&malformed);
        let mount = if route.path == "/oauth/userinfo" {
            ""
        } else {
            prefix
        };
        let path = format!("{mount}{}", filled(route.path));
        let (status, answer) = send(service, method, &path, Auth::Nobody, body).await?;
        if status != 401 || answer["refusal"] != "NotSignedIn" {
            wrong.push(format!("{} {path}: {status} {answer}", route.method.word()));
        }
        asked += 1;
    }
    Ok((asked, wrong))
}

#[tokio::test]
async fn every_signed_in_route_refuses_an_anonymous_malformed_request_as_not_signed_in()
-> TestResult {
    let service = Service::start().await?;
    let (asked, wrong) = walk(&service, "").await?;
    assert!(asked > 100, "the walk reached {asked} signed-in routes");
    assert!(
        wrong.is_empty(),
        "these answer an anonymous caller before judging who it is:\n{}",
        wrong.join("\n")
    );
    Ok(())
}

#[tokio::test]
async fn under_the_screens_every_signed_in_route_refuses_an_anonymous_caller_too() -> TestResult {
    let screens = tempfile::TempDir::new()?;
    std::fs::write(screens.path().join("index.html"), PAGE)?;
    let dir = screens.path().to_path_buf();
    let (service, ()) = Service::start_adjusted(
        GRANT_MODEL,
        None,
        None,
        None,
        |config| config.surface_dir = Some(dir),
        |_| Ok(()),
    )
    .await?;
    let (asked, wrong) = walk(&service, "/api").await?;
    assert!(asked > 100, "the walk reached {asked} signed-in routes");
    assert!(
        wrong.is_empty(),
        "these answer an anonymous caller under /api before judging who it is:\n{}",
        wrong.join("\n")
    );
    Ok(())
}

#[tokio::test]
async fn a_head_on_every_signed_in_get_route_is_refused_before_its_query_is_read() -> TestResult {
    let service = Service::start().await?;
    let client = reqwest::Client::new();
    let mut wrong = Vec::new();
    let mut asked = 0;
    for route in api().routes() {
        if route.method != Method::Get || route.auth.contains(&lys_openapi::Auth::Public) {
            continue;
        }
        let path = filled(route.path);
        let response = client
            .head(format!("{}{path}?%zz", service.base))
            .send()
            .await?;
        let status = response.status().as_u16();
        let body = response.bytes().await?;
        if status != 401 || !body.is_empty() {
            wrong.push(format!("HEAD {path}: {status}, {} body bytes", body.len()));
        }
        asked += 1;
    }
    assert!(asked > 40, "the walk reached {asked} signed-in GET routes");
    assert!(
        wrong.is_empty(),
        "these answer an anonymous HEAD before judging who asks:\n{}",
        wrong.join("\n")
    );
    Ok(())
}

#[tokio::test]
async fn nested_api_authentication_precedes_json_extraction_while_static_screen_stays_public()
-> TestResult {
    let screens = tempfile::TempDir::new()?;
    std::fs::write(screens.path().join("index.html"), PAGE)?;
    let dir = screens.path().to_path_buf();
    let (service, ()) = Service::start_adjusted(
        GRANT_MODEL,
        None,
        None,
        None,
        |config| config.surface_dir = Some(dir),
        |_| Ok(()),
    )
    .await?;
    let client = reqwest::Client::new();
    let public = client.get(format!("{}/", service.base)).send().await?;
    assert_eq!(public.status(), 200);
    assert_eq!(public.text().await?, PAGE);
    let refused = client
        .post(format!("{}/api/people", service.base))
        .header("content-type", "application/json")
        .body("{")
        .send()
        .await?;
    assert_eq!(refused.status(), 401);
    let answer: serde_json::Value = serde_json::from_str(&refused.text().await?)?;
    assert_eq!(answer["refusal"], "NotSignedIn");
    let (status, receipt) = service.get("/api/receipts/0", None).await?;
    assert_eq!(status, 404, "the refused request wrote a leaf: {receipt}");
    Ok(())
}
