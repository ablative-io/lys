//! The fixed CLI approval returns person authority through the signed HTTP routes.

use base64::Engine;
use serde_json::Value;
use sha2::{Digest, Sha256};

#[tokio::test]
async fn native_pkce_approves_the_person_refreshes_and_refuses_foreign_redirects()
-> Result<(), Box<dyn std::error::Error>> {
    let (service, cookie) = crate::mcp_oauth::tests::service().await?;
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    let verifier =
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(rand::random::<[u8; 32]>());
    let challenge = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(Sha256::digest(verifier.as_bytes()));
    let redirect = "http://127.0.0.1:23456/lys/sign-in";
    for bad in [
        "https://other.example/lys/sign-in",
        "http://localhost:23456/lys/sign-in",
    ] {
        let response = client
            .get(format!("{}/oauth/mcp/authorize", service.base))
            .query(&[
                ("response_type", "code"),
                ("client_id", "lys-cli"),
                ("redirect_uri", bad),
                ("code_challenge", challenge.as_str()),
                ("code_challenge_method", "S256"),
            ])
            .header("cookie", &cookie)
            .send()
            .await?;
        assert_eq!(response.status(), 400);
        let refusal: Value = response.json().await?;
        assert_eq!(refusal["refusal"], "RedirectUnregistered");
    }
    let response = client
        .get(format!("{}/oauth/mcp/authorize", service.base))
        .query(&[
            ("response_type", "code"),
            ("client_id", "lys-cli"),
            ("redirect_uri", redirect),
            ("code_challenge", challenge.as_str()),
            ("code_challenge_method", "S256"),
        ])
        .header("cookie", &cookie)
        .send()
        .await?;
    assert_eq!(response.status(), 200);
    let page = response.text().await?;
    assert!(page.contains("Your terminal will act as you"));
    let asking = page
        .split("name=\"asking\" value=\"")
        .nth(1)
        .and_then(|text| text.split('"').next())
        .ok_or("approval absent")?;
    let response = client
        .post(format!("{}/oauth/mcp/consent", service.base))
        .header("cookie", &cookie)
        .form(&[("asking", asking), ("decision", "approve")])
        .send()
        .await?;
    assert_eq!(response.status(), 303);
    let back = reqwest::Url::parse(
        response
            .headers()
            .get("location")
            .ok_or("redirect absent")?
            .to_str()?,
    )?;
    let code = back
        .query_pairs()
        .find(|(name, _)| name == "code")
        .ok_or("answer absent")?
        .1
        .into_owned();
    let response = client
        .post(format!("{}/oauth/mcp/token", service.base))
        .form(&[
            ("grant_type", "authorization_code"),
            ("client_id", "lys-cli"),
            ("code", code.as_str()),
            ("code_verifier", verifier.as_str()),
            ("redirect_uri", redirect),
        ])
        .send()
        .await?;
    assert_eq!(response.status(), 200);
    let tokens: Value = response.json().await?;
    let access = tokens["access_token"].as_str().ok_or("pass absent")?;
    let (_, own) = service.get("/me", Some(&cookie)).await?;
    assert_eq!(tokens["person"], own["person"]["id"]);
    let response = client
        .get(format!("{}/me", service.base))
        .bearer_auth(access)
        .send()
        .await?;
    assert_eq!(response.status(), 200);
    let person: Value = response.json().await?;
    assert_eq!(person["person"]["id"], own["person"]["id"]);
    let refresh = tokens["refresh_token"].as_str().ok_or("renewal absent")?;
    let response = client
        .post(format!("{}/oauth/mcp/token", service.base))
        .form(&[
            ("grant_type", "refresh_token"),
            ("client_id", "lys-cli"),
            ("refresh_token", refresh),
        ])
        .send()
        .await?;
    assert_eq!(response.status(), 200);
    let renewed: Value = response.json().await?;
    assert_eq!(renewed["person"], own["person"]["id"]);
    let response = client
        .get(format!("{}/me", service.base))
        .bearer_auth(access)
        .header("cookie", &cookie)
        .send()
        .await?;
    assert_eq!(response.status(), 401);
    let response = client
        .get(format!("{}/me", service.base))
        .bearer_auth(format!("{access}x"))
        .send()
        .await?;
    assert_eq!(response.status(), 401);
    let refusal: Value = response.json().await?;
    assert_eq!(refusal["refusal"], "NotSignedIn");
    Ok(())
}

#[tokio::test]
async fn malformed_person_payloads_keep_the_decoders_named_refusal()
-> Result<(), Box<dyn std::error::Error>> {
    let service = crate::mcp_oauth::tests::service().await?.0;
    let client = reqwest::Client::new();
    let encoding = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    let header = encoding.encode(serde_json::to_vec(&serde_json::json!({ "alg": "ES256" }))?);
    let signature = encoding.encode(rand::random::<[u8; 32]>());
    for (payload, name, cause) in [
        ("%%".to_owned(), "person_payload_invalid", "Invalid symbol"),
        (
            encoding.encode(b"{"),
            "person_claims_invalid",
            "EOF while parsing an object",
        ),
    ] {
        let response = client
            .get(format!("{}/me", service.base))
            .bearer_auth(format!("{header}.{payload}.{signature}"))
            .send()
            .await?;
        assert_eq!(response.status(), 401);
        let refusal: Value = response.json().await?;
        assert_eq!(refusal["refusal"], "PassRefused");
        let reason = refusal["reason"].as_str().ok_or("refusal reason absent")?;
        assert!(reason.contains(name), "decoder refusal name absent");
        assert!(reason.contains(cause), "decoder cause absent");
    }
    Ok(())
}
