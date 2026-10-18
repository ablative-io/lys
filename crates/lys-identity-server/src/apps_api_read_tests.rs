#![cfg(test)]
//! What `GET /apps` answers beside the apps (DIRECTORY-102 R2, R4): Lys's
//! own address for an app to call back, as `dial_address` computes it for a
//! runner, or null with the reason it has none; and each app's
//! `connect_origin`, the origin it last connected from, null for an app
//! registered by hand.

use identity_contract::apps::{
    Auth, FILES, NOTES, get, login, ok, op, post, registered, seeded, workspace_schema,
};
use identity_contract::harness::{ADMINISTRATOR, GRANT_MODEL, Service};
use identity_contract::lys_identity_server::dev_seed::seed_configured;
use serde_json::{Value, json};

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

/// The service with the apps' broker, its own address `redirect`, and the
/// administrator and Bea seeded.
async fn served_at(redirect: &'static str) -> Result<Service> {
    let broker = identity_contract::app_custody::start().await?;
    let (service, ()) = Service::start_adjusted(
        GRANT_MODEL,
        None,
        None,
        None,
        move |config| {
            config.redirect_url = redirect.to_owned();
            config.secrets = Some(
                identity_contract::lys_identity_server::secrets_api::SecretsSettings {
                    broker,
                    service: "identity".to_owned(),
                    service_key_file: config.event_key_file.clone(),
                },
            );
        },
        |config| {
            seed_configured(config, [ADMINISTRATOR, "bea-subject"])?;
            Ok(())
        },
    )
    .await?;
    Ok(service)
}

async fn listed(service: &Service) -> Result<Value> {
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    ok(get(service, "/apps", Auth::Cookie(&admin)).await?)
}

#[tokio::test]
async fn get_apps_answers_the_address_dial_address_gives_for_a_registered_https_address() -> Result
{
    let redirect = "https://lys.example.test/identity/callback";
    let service = served_at(redirect).await?;
    let answer = listed(&service).await?;
    let dialled = crate::network_join::dial_address(redirect)?;
    assert_eq!(answer["public_address"], dialled.as_str(), "{answer}");
    // The second party: the address as a person would write it by hand.
    assert_eq!(
        answer["public_address"],
        "https://lys.example.test/identity"
    );
    assert_eq!(answer["public_address_reason"], Value::Null, "{answer}");
    Ok(())
}

#[tokio::test]
async fn get_apps_answers_no_public_address_and_its_reason_for_a_loopback_address() -> Result {
    let (service, _) = seeded().await?;
    let answer = listed(&service).await?;
    assert_eq!(answer["public_address"], Value::Null, "{answer}");
    let reason = answer["public_address_reason"]
        .as_str()
        .ok_or("no reason for the missing address")?;
    let here = reqwest::Url::parse(&service.base)?
        .origin()
        .ascii_serialization();
    assert!(reason.contains(&here), "{reason}");
    assert!(reason.contains("only the computer it runs on"), "{reason}");
    assert!(reason.contains("https"), "{reason}");
    Ok(())
}

#[tokio::test]
async fn get_apps_answers_no_public_address_and_its_reason_for_plain_http() -> Result {
    let service = served_at("http://lys.example.test/callback").await?;
    let answer = listed(&service).await?;
    assert_eq!(answer["public_address"], Value::Null, "{answer}");
    let reason = answer["public_address_reason"]
        .as_str()
        .ok_or("no reason for the missing address")?;
    assert!(reason.contains("plain http"), "{reason}");
    assert!(reason.contains("http://lys.example.test"), "{reason}");
    Ok(())
}

#[tokio::test]
async fn connect_origin_is_where_an_app_connected_from_and_null_for_one_registered_by_hand()
-> Result {
    let (service, _) = seeded().await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    registered(&service, &admin, FILES).await?;
    let origin = "https://notes.example.test";
    let body = json!({"operation": op()?,
        "manifest": {"id": NOTES, "name": "Notes", "redirects": [format!("{origin}/identity/oidc/callback")],
            "profile": false, "schema": workspace_schema(NOTES)},
        "return": format!("{origin}/identity/lys/connected"), "state": "s1"});
    ok(post(&service, "/apps/connect", Auth::Cookie(&admin), &body).await?)?;
    let answer = ok(get(&service, "/apps", Auth::Cookie(&admin)).await?)?;
    let apps = answer["apps"].as_array().ok_or("no apps")?;
    let app = |id: &str| {
        apps.iter()
            .find(|app| app["id"] == id)
            .cloned()
            .ok_or_else(|| format!("{id} is not listed: {answer}"))
    };
    assert_eq!(app(NOTES)?["connect_origin"], origin, "{answer}");
    let by_hand = app(FILES)?;
    assert!(
        by_hand
            .as_object()
            .is_some_and(|app| app.contains_key("connect_origin")),
        "{by_hand}"
    );
    assert_eq!(by_hand["connect_origin"], Value::Null, "{by_hand}");
    let one = ok(get(&service, &format!("/apps/{NOTES}"), Auth::Cookie(&admin)).await?)?;
    assert_eq!(one["connect_origin"], origin, "{one}");
    Ok(())
}
