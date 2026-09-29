#![cfg(test)]
//! Every route the table names with no public door answers an anonymous
//! caller that it is not signed in, even when the body is malformed: the
//! caller's authentication is judged before its body is read, so no
//! anonymous caller is ever told how its body failed to parse.

use identity_contract::apps::{Auth, TestResult, send};
use identity_contract::harness::Service;
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

#[tokio::test]
async fn every_signed_in_route_refuses_an_anonymous_malformed_request_as_not_signed_in()
-> TestResult {
    let service = Service::start().await?;
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
        let path = filled(route.path);
        let (status, answer) = send(&service, method, &path, Auth::Nobody, body).await?;
        if status != 401 || answer["refusal"] != "NotSignedIn" {
            wrong.push(format!(
                "{} {}: {status} {answer}",
                route.method.word(),
                route.path
            ));
        }
        asked += 1;
    }
    assert!(asked > 100, "the walk reached {asked} signed-in routes");
    assert!(
        wrong.is_empty(),
        "these answer an anonymous caller before judging who it is:\n{}",
        wrong.join("\n")
    );
    Ok(())
}
