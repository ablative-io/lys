#![cfg(test)]
//! Getting started's "Not this one" is kept for the person (DIRECTORY-101
//! R7): `PUT /me/getting-started` keeps it for the caller, `GET /me` answers
//! it to that person and to no one else, and it outlasts a restart.

use identity_contract::apps::{Auth, BEA, login, ok, send};
use identity_contract::harness::{ADMINISTRATOR, Service};
use identity_contract::lys_identity_server::dev_seed::seed_configured;
use serde_json::json;

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

#[tokio::test]
async fn not_this_one_is_kept_for_the_person_who_said_it_and_for_no_one_else() -> Result {
    let (mut service, _) =
        Service::start_with(|config| Ok(seed_configured(config, [ADMINISTRATOR, BEA])?)).await?;
    let ada = service.sign_in(login(ADMINISTRATOR)).await?;
    let bea = service.sign_in(login(BEA)).await?;
    let (status, me) = service.get("/me", Some(&ada)).await?;
    assert_eq!(status, 200, "{me}");
    assert_eq!(
        me["getting_started"],
        json!({"host_declined": false}),
        "{me}"
    );

    let kept = ok(send(
        &service,
        reqwest::Method::PUT,
        "/me/getting-started",
        Auth::Cookie(&ada),
        Some(&json!({"host_declined": true})),
    )
    .await?)?;
    assert_eq!(kept, json!({"host_declined": true}));
    let (_, me) = service.get("/me", Some(&ada)).await?;
    assert_eq!(
        me["getting_started"],
        json!({"host_declined": true}),
        "{me}"
    );
    let (_, other) = service.get("/me", Some(&bea)).await?;
    assert_eq!(
        other["getting_started"],
        json!({"host_declined": false}),
        "{other}"
    );

    service.restart().await?;
    let (_, me) = service.get("/me", Some(&ada)).await?;
    assert_eq!(
        me["getting_started"],
        json!({"host_declined": true}),
        "kept across a restart: {me}"
    );
    Ok(())
}

#[tokio::test]
async fn keeping_it_is_for_a_signed_in_person() -> Result {
    let (service, _) =
        Service::start_with(|config| Ok(seed_configured(config, [ADMINISTRATOR, BEA])?)).await?;
    let (status, answer) = send(
        &service,
        reqwest::Method::PUT,
        "/me/getting-started",
        Auth::Nobody,
        Some(&json!({"host_declined": true})),
    )
    .await?;
    assert_eq!(status, 401, "{answer}");
    Ok(())
}
