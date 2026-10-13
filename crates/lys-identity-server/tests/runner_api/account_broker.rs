//! Account review and binding use the same broker declarations as the held handles.
use super::{IntoResponse, PLANTED, Request, Response, json};

pub(super) async fn broker(request: Request) -> Response {
    match (request.method().as_str(), request.uri().path()) {
        ("GET", "/_lys/secrets") => axum::Json(json!({"secrets":[{
            "name":"model account", "class":"credential",
            "upstream":"https://api.anthropic.com", "header":"authorization"
        }]}))
        .into_response(),
        ("GET", "/_lys/account/key") => "ab".repeat(32).into_response(),
        ("POST", "/_lys/account/bind") => bind(request).await,
        ("GET", "/_lys/handles") => {
            let handles = ["h-live", "h-account-one", "h-account-two"].map(|id| {
                json!({
                    "id": id, "secret": "model account", "max_uses": 10, "used": 1,
                    "not_after_ms": 0, "dropped": false, "spend_cap": null, "settled": 0,
                    "parent": null, "value": PLANTED, "token": PLANTED,
                })
            });
            axum::Json(json!({ "holder": "any", "handles": handles })).into_response()
        }
        _ => (axum::http::StatusCode::NOT_FOUND, "fixture_route_unknown").into_response(),
    }
}

async fn bind(request: Request) -> Response {
    for header in [
        "lys-service",
        "lys-on-behalf-of",
        "lys-operation",
        "lys-signed-at",
        "lys-service-signature",
    ] {
        assert!(
            request.headers().contains_key(header),
            "binding missing {header}"
        );
    }
    let body = match axum::body::to_bytes(request.into_body(), 8192).await {
        Ok(body) => body,
        Err(error) => {
            return (axum::http::StatusCode::BAD_REQUEST, error.to_string()).into_response();
        }
    };
    let binding: serde_json::Value = match serde_json::from_slice(&body) {
        Ok(binding) => binding,
        Err(error) => {
            return (axum::http::StatusCode::BAD_REQUEST, error.to_string()).into_response();
        }
    };
    assert_eq!(
        binding["accounts"],
        json!(["h-account-one", "h-account-two"])
    );
    assert_eq!(binding["provider"], "https://api.anthropic.com");
    assert!(binding["run"].as_str().is_some_and(|run| !run.is_empty()));
    assert!(
        binding["agent"]
            .as_str()
            .is_some_and(|agent| !agent.is_empty())
    );
    axum::Json(json!({"bound":true})).into_response()
}
