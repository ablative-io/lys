#![cfg(test)]
//! Connecting an approved app again leaves the running app's credentials
//! live until the exchange hands over the new ones (DIRECTORY-102 R1): the
//! approval issues nothing the running app holds and ends nothing it holds;
//! the exchange that hands the new values over revokes what they replace,
//! with the reason "the app was connected again". A second connect before
//! any exchange replaces the first's pending values, whose code is then
//! refused by its own name, and its one exchange leaves only the newest live.

use identity_contract::apps::{
    Auth, NOTES, get, login, ok, op, post, refused, seeded, workspace_schema,
};
use identity_contract::harness::{ADMINISTRATOR, Service};
use serde_json::{Value, json};

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

const ORIGIN: &str = "https://notes.example.test";
const REASON: &str = "the app was connected again";

fn connect_body() -> Result<Value> {
    Ok(json!({"operation": op()?,
        "manifest": {"id": NOTES, "name": "Notes", "redirects": [format!("{ORIGIN}/identity/oidc/callback")],
            "profile": true, "schema": workspace_schema(NOTES)},
        "return": format!("{ORIGIN}/identity/lys/connected"), "state": "s1"}))
}

/// Approve a connect as the administrator, answering its one-time code.
async fn approved(service: &Service, admin: &str) -> Result<String> {
    let body = connect_body()?;
    let answer = ok(post(service, "/apps/connect", Auth::Cookie(admin), &body).await?)?;
    let redirect = answer["redirect"].as_str().ok_or("no redirect")?;
    reqwest::Url::parse(redirect)?
        .query_pairs()
        .find(|(key, _)| key == "code")
        .map(|(_, code)| code.into_owned())
        .ok_or_else(|| "no code".into())
}

/// The app's server collecting `code`, answering the workspace credential.
async fn exchanged(service: &Service, code: &str) -> Result<String> {
    let answer = ok(post(
        service,
        "/apps/connect/exchange",
        Auth::Nobody,
        &json!({"code": code}),
    )
    .await?)?;
    answer["workspace_credential"]
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| "no workspace credential".into())
}

/// The app's client credentials as the administrator reads them.
async fn credentials(service: &Service, admin: &str) -> Result<Vec<Value>> {
    let app = ok(get(service, &format!("/apps/{NOTES}"), Auth::Cookie(admin)).await?)?;
    app["client_credentials"]
        .as_array()
        .cloned()
        .ok_or_else(|| "no client credentials".into())
}

fn live(credentials: &[Value]) -> Vec<String> {
    credentials
        .iter()
        .filter(|credential| credential["live"] == true)
        .filter_map(|credential| credential["credential_id"].as_str().map(str::to_owned))
        .collect()
}

fn entry<'a>(credentials: &'a [Value], id: &str) -> Result<&'a Value> {
    credentials
        .iter()
        .find(|credential| credential["credential_id"] == id)
        .ok_or_else(|| format!("credential {id} is not listed").into())
}

#[tokio::test]
async fn reconnect_keeps_the_running_credentials_until_the_exchange_revokes_them() -> Result {
    let (service, _) = seeded().await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    let running = exchanged(&service, &approved(&service, &admin).await?).await?;
    let before = live(&credentials(&service, &admin).await?);
    assert_eq!(before.len(), 1, "{before:?}");
    let code = approved(&service, &admin).await?;
    // Approve ended nothing: the running app's bearer still signs it in, and
    // its client credential is still live with no revocation on it.
    ok(get(&service, "/apps/me", Auth::Bearer(&running)).await?)?;
    let held = credentials(&service, &admin).await?;
    let prior = entry(&held, &before[0])?;
    assert_eq!(prior["live"], true, "{prior}");
    assert_eq!(prior["revoked_reason"], Value::Null, "{prior}");
    let newest = exchanged(&service, &code).await?;
    refused(
        &get(&service, "/apps/me", Auth::Bearer(&running)).await?,
        401,
        "credential_refused",
    )?;
    ok(get(&service, "/apps/me", Auth::Bearer(&newest)).await?)?;
    let after = credentials(&service, &admin).await?;
    let prior = entry(&after, &before[0])?;
    assert_eq!(prior["live"], false, "{prior}");
    assert_eq!(prior["revoked_reason"], REASON, "{prior}");
    let now = live(&after);
    assert_eq!(now.len(), 1, "{after:?}");
    assert_ne!(now[0], before[0]);
    Ok(())
}

#[tokio::test]
async fn a_second_reconnect_before_any_exchange_revokes_the_first_prior_set_once() -> Result {
    let (service, _) = seeded().await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    let running = exchanged(&service, &approved(&service, &admin).await?).await?;
    let first = live(&credentials(&service, &admin).await?);
    let replaced = approved(&service, &admin).await?;
    let pending = live(&credentials(&service, &admin).await?);
    let code = approved(&service, &admin).await?;
    ok(get(&service, "/apps/me", Auth::Bearer(&running)).await?)?;
    refused(
        &post(
            &service,
            "/apps/connect/exchange",
            Auth::Nobody,
            &json!({"code": replaced}),
        )
        .await?,
        400,
        "connect_code_replaced",
    )?;
    let newest = exchanged(&service, &code).await?;
    refused(
        &get(&service, "/apps/me", Auth::Bearer(&running)).await?,
        401,
        "credential_refused",
    )?;
    ok(get(&service, "/apps/me", Auth::Bearer(&newest)).await?)?;
    let after = credentials(&service, &admin).await?;
    assert_eq!(
        after.len(),
        3,
        "one credential issued per connect: {after:?}"
    );
    let revoked: Vec<&Value> = after
        .iter()
        .filter(|credential| credential["revoked_reason"] == REASON)
        .collect();
    // The first prior set, and the pending value the second connect replaced,
    // each revoked by one line; nothing else is.
    assert_eq!(revoked.len(), 2, "{after:?}");
    for id in pending.iter().chain(first.iter()) {
        assert_eq!(entry(&after, id)?["live"], false, "{id}: {after:?}");
    }
    let now = live(&after);
    assert_eq!(now.len(), 1, "{after:?}");
    assert!(!pending.contains(&now[0]), "{after:?}");
    Ok(())
}
