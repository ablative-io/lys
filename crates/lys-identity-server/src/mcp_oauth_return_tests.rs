use super::return_page::{Return, page};
use axum::http::StatusCode;
use std::error::Error;

#[tokio::test]
async fn finishing_page_carries_one_address_and_no_script_or_redirect() -> Result<(), Box<dyn Error>>
{
    for (native, name) in [(false, "Claude Code"), (true, "lys-cli")] {
        for same_host in [false, true] {
            let response = page(Return {
                redirect: "http://127.0.0.1:23456/lys/sign-in",
                pairs: &[("code", "fixture"), ("state", "kept")],
                name,
                native,
                same_host,
                expires_at: Some(157),
                at: 100,
            })?;
            assert_eq!(response.status(), StatusCode::OK);
            assert!(!response.headers().contains_key("location"));
            assert_eq!(response.headers()["cache-control"], "no-store");
            assert_eq!(response.headers()["x-frame-options"], "DENY");
            let csp = response.headers()["content-security-policy"].to_str()?;
            assert!(!csp.contains("script-src"));
            let body = String::from_utf8(
                axum::body::to_bytes(response.into_body(), usize::MAX)
                    .await?
                    .to_vec(),
            )?;
            assert!(body.contains("readonly"));
            assert!(body.contains(
                "value=\"http://127.0.0.1:23456/lys/sign-in?code=fixture&amp;state=kept\""
            ));
            assert!(body.contains(
                "href=\"http://127.0.0.1:23456/lys/sign-in?code=fixture&amp;state=kept\""
            ));
            assert_eq!(body.contains("http-equiv=\"refresh\""), same_host);
            assert!(!body.contains("<script"));
            assert!(body.contains("once"));
            assert!(body.contains("57 seconds"));
            assert!(body.contains(if native {
                "lys sign-in --finish"
            } else {
                "complete_authentication"
            }));
        }
    }
    let response = page(Return {
        redirect: "https://app.test/back",
        pairs: &[("error", "access_denied"), ("state", "kept")],
        name: "App",
        native: false,
        same_host: false,
        expires_at: None,
        at: 100,
    })?;
    let body = String::from_utf8(
        axum::body::to_bytes(response.into_body(), usize::MAX)
            .await?
            .to_vec(),
    )?;
    assert!(!body.contains("<input"));
    assert!(body.contains("error=access_denied&amp;state=kept"));
    Ok(())
}

#[tokio::test]
async fn registered_and_native_clients_return_only_to_the_same_host_and_decide_once()
-> Result<(), Box<dyn Error>> {
    let (service, cookie) = super::tests::service().await?;
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .no_proxy()
        .build()?;
    let elsewhere = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .no_proxy()
        .local_address("127.0.0.2".parse::<std::net::IpAddr>()?)
        .build()?;
    let app:serde_json::Value=client.post(format!("{}/oauth/mcp/register",service.base)).json(&serde_json::json!({"client_name":"Claude Code","redirect_uris":["http://127.0.0.1:23456/lys/sign-in"]})).send().await?.json().await?;
    for id in [app["client_id"].as_str().ok_or("client absent")?, "lys-cli"] {
        for same in [true, false] {
            let response = client
                .get(format!("{}/oauth/mcp/authorize", service.base))
                .header("cookie", &cookie)
                .query(&[
                    ("response_type", "code"),
                    ("client_id", id),
                    ("redirect_uri", "http://127.0.0.1:23456/lys/sign-in"),
                    ("code_challenge", &"a".repeat(43)),
                    ("code_challenge_method", "S256"),
                    ("state", "kept"),
                ])
                .send()
                .await?;
            let body = response.text().await?;
            let asking = body
                .split("name=\"asking\" value=\"")
                .nth(1)
                .and_then(|s| s.split('"').next())
                .ok_or("asking absent")?;
            let browser = if same { &client } else { &elsewhere };
            let answer = browser
                .post(format!("{}/oauth/mcp/consent", service.base))
                .header("cookie", &cookie)
                .form(&[("asking", asking), ("decision", "approve")])
                .send()
                .await?;
            assert_eq!(answer.status(), 200);
            assert!(!answer.headers().contains_key("location"));
            let body = answer.text().await?;
            assert_eq!(body.contains("http-equiv=\"refresh\""), same);
            let again = browser
                .post(format!("{}/oauth/mcp/consent", service.base))
                .header("cookie", &cookie)
                .form(&[("asking", asking), ("decision", "approve")])
                .send()
                .await?;
            assert_eq!(again.status(), 400);
            assert!(again.text().await?.contains("CodeUnknown"));
        }
    }
    Ok(())
}

#[test]
fn elsewhere_warning_names_only_the_client_once() -> Result<(), Box<dyn Error>> {
    let lines = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let heard = std::sync::Arc::clone(&lines);
    tracing::subscriber::with_default(
        crate::warn_lines::WarnLines::new(std::sync::Arc::new(move |s| {
            heard.lock().expect("capture poisoned").push(s.to_owned());
        })),
        || {
            super::return_page::elsewhere("Claude Code", false);
            super::return_page::elsewhere("Claude Code", true);
        },
    );
    let lines = lines.lock().map_err(|e| e.to_string())?;
    assert_eq!(lines.len(), 1);
    assert!(lines[0].contains("WARN"));
    assert!(lines[0].contains("Claude Code"));
    assert!(lines[0].contains("elsewhere"));
    for secret in ["code=", "state=", "http:", "127.0.0.1", "host="] {
        assert!(!lines[0].contains(secret));
    }
    Ok(())
}

#[tokio::test]
async fn forwarded_hosts_are_compared_only_from_a_configured_proxy() -> Result<(), Box<dyn Error>> {
    for trusted in [false, true] {
        let (service, cookie) = super::tests::service_with_trusted(if trusted {
            vec!["127.0.0.1".parse()?]
        } else {
            Vec::new()
        })
        .await?;
        let client = reqwest::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .build()?;
        let app: serde_json::Value = client
            .post(format!("{}/oauth/mcp/register", service.base))
            .header("x-forwarded-for", "192.0.2.1")
            .json(
                &serde_json::json!({"client_name":"App","redirect_uris":["https://app.test/back"]}),
            )
            .send()
            .await?
            .json()
            .await?;
        for host in ["192.0.2.1", "192.0.2.2"] {
            let page = client
                .get(format!("{}/oauth/mcp/authorize", service.base))
                .header("cookie", &cookie)
                .query(&[
                    (
                        "client_id",
                        app["client_id"].as_str().ok_or("client absent")?,
                    ),
                    ("redirect_uri", "https://app.test/back"),
                    ("response_type", "code"),
                    ("code_challenge", &"a".repeat(43)),
                    ("code_challenge_method", "S256"),
                ])
                .send()
                .await?
                .text()
                .await?;
            let asking = page
                .split("name=\"asking\" value=\"")
                .nth(1)
                .and_then(|s| s.split('"').next())
                .ok_or("asking absent")?;
            let answer = client
                .post(format!("{}/oauth/mcp/consent", service.base))
                .header("cookie", &cookie)
                .header("x-forwarded-for", host)
                .form(&[("asking", asking), ("decision", "approve")])
                .send()
                .await?;
            assert_eq!(answer.status(), 200);
            assert_eq!(
                answer.text().await?.contains("http-equiv=\"refresh\""),
                !trusted || host == "192.0.2.1"
            );
        }
    }
    Ok(())
}
