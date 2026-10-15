#![cfg(test)]
//! Connect an app in one act: the administrator's approval of a manifest
//! registers, approves and issues, and the app's server exchanges the
//! one-time code for exactly its credentials.

use identity_contract::apps::{
    Auth, NOTES, TestResult, get, login, ok, op, post, refused, seeded, workspace_schema,
};
use identity_contract::harness::ADMINISTRATOR;
use serde_json::json;

const ORIGIN: &str = "https://notes.example.test";

fn manifest() -> serde_json::Value {
    json!({"id": NOTES, "name": "Notes", "redirects": [format!("{ORIGIN}/identity/oidc/callback")],
        "post_logout_redirects": [format!("{ORIGIN}/")], "profile": true, "schema": workspace_schema(NOTES)})
}

#[tokio::test]
async fn apps_connect_registers_approves_and_issues_in_one_act() -> TestResult {
    let (service, _) = seeded().await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    let back = format!("{ORIGIN}/identity/lys/connected");
    let body = json!({"operation": op()?, "manifest": manifest(), "return": back, "state": "s123"});
    let answer = ok(post(&service, "/apps/connect", Auth::Cookie(&admin), &body).await?)?;
    let redirect = answer["redirect"].as_str().ok_or("no redirect")?;
    let url = reqwest::Url::parse(redirect)?;
    assert!(redirect.starts_with(&back), "{redirect}");
    let pairs: std::collections::HashMap<_, _> = url.query_pairs().into_owned().collect();
    assert_eq!(pairs["state"], "s123");
    let code = pairs["code"].clone();
    assert_eq!(code.len(), 64);
    let app = ok(get(&service, &format!("/apps/{NOTES}"), Auth::Cookie(&admin)).await?)?;
    assert_eq!(app["state"], "approved", "{app}");
    let exchanged = ok(post(
        &service,
        "/apps/connect/exchange",
        Auth::Nobody,
        &json!({"code": code}),
    )
    .await?)?;
    let mut fields: Vec<_> = exchanged
        .as_object()
        .ok_or("not an object")?
        .keys()
        .cloned()
        .collect();
    fields.sort();
    assert_eq!(
        fields,
        [
            "app",
            "client_id",
            "client_secret",
            "issuer",
            "workspace_credential"
        ]
    );
    assert_eq!(exchanged["app"], NOTES);
    assert_eq!(exchanged["client_id"], NOTES);
    let secret = exchanged["client_secret"]
        .as_str()
        .ok_or("no client secret")?;
    assert!(
        secret.starts_with(&format!("lys-client.{NOTES}.")),
        "{secret}"
    );
    let bearer = exchanged["workspace_credential"]
        .as_str()
        .ok_or("no bearer")?;
    assert!(bearer.starts_with(&format!("lys-app.{NOTES}.")), "{bearer}");
    ok(get(&service, "/apps/me", Auth::Bearer(bearer)).await?)?;
    refused(
        &post(
            &service,
            "/apps/connect/exchange",
            Auth::Nobody,
            &json!({"code": code}),
        )
        .await?,
        400,
        "connect_code_used",
    )?;
    assert_eq!(app["sign_in"]["redirects"], manifest()["redirects"]);
    assert_eq!(app["sign_in"]["profile"], true);
    // Connecting again re-issues: the old client credential and bearer end,
    // and the sign-in settings follow the manifest as it is now.
    let mut moved = manifest();
    moved["redirects"] = json!([
        format!("{ORIGIN}/identity/oidc/callback"),
        format!("{ORIGIN}/identity/google/connect/callback")
    ]);
    moved["profile"] = json!(false);
    let again = json!({"operation": op()?, "manifest": moved, "return": back, "state": "s456"});
    ok(post(&service, "/apps/connect", Auth::Cookie(&admin), &again).await?)?;
    refused(
        &get(&service, "/apps/me", Auth::Bearer(bearer)).await?,
        401,
        "credential_refused",
    )?;
    let kept = ok(get(&service, &format!("/apps/{NOTES}"), Auth::Cookie(&admin)).await?)?;
    assert_eq!(kept["sign_in"]["redirects"], moved["redirects"]);
    assert_eq!(kept["sign_in"]["profile"], false);
    let credentials = kept["client_credentials"]
        .as_array()
        .ok_or("no client credentials")?;
    assert_eq!(credentials.len(), 2, "{kept}");
    let live = credentials
        .iter()
        .filter(|credential| credential["live"] == true)
        .count();
    assert_eq!(live, 1, "{kept}");
    Ok(())
}

#[tokio::test]
async fn apps_connect_refuses_a_return_outside_the_manifest_origin() -> TestResult {
    let (service, _) = seeded().await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    let body = json!({"operation": op()?, "manifest": manifest(),
        "return": "https://evil.example.test/identity/lys/connected", "state": "s"});
    refused(
        &post(&service, "/apps/connect", Auth::Cookie(&admin), &body).await?,
        400,
        "redirect_invalid",
    )?;
    refused(
        &get(&service, &format!("/apps/{NOTES}"), Auth::Cookie(&admin)).await?,
        404,
        "app_unknown",
    )?;
    Ok(())
}
