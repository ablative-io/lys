#![cfg(test)]
//! A person's own account (DIRECTORY-101 R3): `GET /me/account` says whether
//! the account has a password and which providers it signs in with; an
//! account with no password sets one without a current password; changing
//! email proves both mailboxes, and an
//! account with a password is asked for it every time.

use identity_contract::fake_rauthy::{API_KEY, FakeRauthy};
use identity_contract::harness::{ADMINISTRATOR, GRANT_MODEL, Service};
use identity_contract::lys_identity_server as server;
use lys_identity::{Actor, AuthMethod, LoginBinding, Provenance};
use serde_json::{Value, json};
use server::sign_in_providers::SignInProvidersSettings;

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

/// Call the issuer's administration API as the installation's key does.
async fn issuer_api(rauthy: &FakeRauthy, path: &str, body: &Value) -> Result<Value> {
    Ok(reqwest::Client::new()
        .post(format!("{}{path}", rauthy.api()))
        .header(reqwest::header::AUTHORIZATION, format!("API-Key {API_KEY}"))
        .json(body)
        .send()
        .await?
        .json()
        .await?)
}

/// The service over an issuer holding Google as a provider and Gail's account,
/// bound to Google with no password, and Gail's session.
async fn table() -> Result<(Service, FakeRauthy, String)> {
    let (service, rauthy, cookie, _sink) = table_with_mail(false).await?;
    Ok((service, rauthy, cookie))
}

async fn table_with_mail(
    mail: bool,
) -> Result<(
    Service,
    FakeRauthy,
    String,
    identity_contract::mail_sink::MailSink,
)> {
    let sink = identity_contract::mail_sink::MailSink::start().await?;
    let port = sink.port();
    let rauthy = FakeRauthy::start().await?;
    let google = issuer_api(
        &rauthy,
        "/providers/create",
        &json!({"name": "Google", "enabled": true, "client_id": "google-client"}),
    )
    .await?;
    let gail = issuer_api(
        &rauthy,
        "/users",
        &json!({
            "email": "gail@example.test", "given_name": "Gail", "language": "en", "roles": [],
            "enabled": true, "email_verified": true, "user_values": {},
            "auth_provider_id": google["id"], "federation_uid": "google-gail",
        }),
    )
    .await?;
    let subject = gail["id"]
        .as_str()
        .ok_or("the issuer names the account")?
        .to_owned();
    let settings = SignInProvidersSettings {
        api: rauthy.api().to_owned(),
        api_key_file: rauthy.api_key_file(),
    };
    let (service, cookie) = Service::start_adjusted(
        GRANT_MODEL,
        None,
        None,
        Some(settings),
        move |config| {
            config.mail =
                mail.then(
                    || identity_contract::lys_identity_server::mail::MailSettings {
                        host: "127.0.0.1".to_owned(),
                        port,
                        user: None,
                        password_file: None,
                        from: "Lys <lys@example.test>".to_owned(),
                        tls: identity_contract::lys_identity_server::mail::Tls::None,
                        sent_file: config.log_dir.with_file_name("letters-sent.jsonl"),
                    },
                );
        },
        |config| {
            identity_contract::lys_identity_server::dev_seed::seed_configured(
                config,
                [ADMINISTRATOR, subject.as_str()],
            )?;
            let mut directory = server::routes::open_directory(config)?;
            let binding = LoginBinding::new(&config.issuer, &subject)?;
            let person = directory
                .projection()?
                .person_for(&binding)
                .ok_or("Gail's person")?;
            std::fs::write(
                config.log_dir.with_file_name("foundings.json"),
                json!({
                    "format": crate::foundings_store::FORMAT, "foundings": {},
                    "admitted_email": {(person.to_string()): "gail@example.test"},
                })
                .to_string(),
            )?;
            drop(directory);
            let sessions = crate::session::Sessions::open(
                config.sessions_file.clone().ok_or("session file missing")?,
                600,
                false,
            )?;
            Ok(sessions.begin(Actor::new(
                LoginBinding::new(&config.issuer, &subject)?,
                Provenance::new(AuthMethod::Oidc, crate::session::now()),
            ))?)
        },
    )
    .await?;
    Ok((service, rauthy, cookie, sink))
}

#[tokio::test]
async fn a_google_account_with_no_password_sets_one_without_a_current_then_needs_it() -> Result {
    let (service, rauthy, gail) = table().await?;
    let (status, shown) = service.get("/me/account", Some(&gail)).await?;
    assert_eq!(status, 200, "{shown}");
    assert_eq!(shown["password"], false, "{shown}");
    assert_eq!(shown["providers"], json!(["google"]), "{shown}");

    let (status, changed) = service
        .post(
            "/me/account/email",
            Some(&gail),
            &json!({"email": "gail@lovelace.test"}),
        )
        .await?;
    assert_eq!(status, 409, "{changed}");
    assert_eq!(changed["refusal"], "EmailChangeNeedsMail");
    let (_, unchanged) = service.get("/me/account", Some(&gail)).await?;
    assert_eq!(unchanged["email"], "gail@example.test");
    assert_eq!(rauthy.users()?[0]["email_verified"], true);

    let (status, set) = service
        .post(
            "/me/account/password",
            Some(&gail),
            &json!({"password": "Gail-First-Password-77"}),
        )
        .await?;
    assert_eq!(status, 200, "{set}");
    assert_eq!(set["password"], true, "{set}");
    assert_eq!(set["providers"], json!(["google"]), "{set}");

    let (status, again) = service
        .post(
            "/me/account/password",
            Some(&gail),
            &json!({"password": "Gail-Second-Password-88"}),
        )
        .await?;
    assert_eq!(
        (status, again["refusal"].clone()),
        (400, json!("AccountRefused")),
        "{again}"
    );
    let (status, email) = service
        .post(
            "/me/account/email",
            Some(&gail),
            &json!({"email": "gail@example.test"}),
        )
        .await?;
    assert_eq!(
        (status, email["refusal"].clone()),
        (400, json!("AccountRefused")),
        "{email}"
    );
    Ok(())
}

#[tokio::test]
async fn an_unverified_address_stays_unverified_when_setting_a_first_password() -> Result {
    let (service, rauthy, gail) = table().await?;
    let mut user = rauthy.users()?[0].clone();
    let id = user["id"].as_str().ok_or("Gail's account")?.to_owned();
    user["email_verified"] = json!(false);
    reqwest::Client::new()
        .put(format!("{}/users/{id}", rauthy.api()))
        .header(reqwest::header::AUTHORIZATION, format!("API-Key {API_KEY}"))
        .json(&user)
        .send()
        .await?
        .error_for_status()?;
    let (status, answer) = service
        .post(
            "/me/account/email",
            Some(&gail),
            &json!({"email":"annabel@example.test"}),
        )
        .await?;
    assert_eq!(status, 409, "{answer}");
    assert_eq!(answer["refusal"], "EmailChangeNeedsMail");
    assert_eq!(rauthy.users()?[0], user);
    let (status, answer) = service
        .post(
            "/me/account/password",
            Some(&gail),
            &json!({"password":"Gail-First-Password-77"}),
        )
        .await?;
    assert_eq!(status, 200, "{answer}");
    assert_eq!(rauthy.users()?[0]["email_verified"], false);
    Ok(())
}

/// Codes are taken from letters, with no clock or mailbox polling.
fn code(letter: &identity_contract::mail_sink::Captured) -> Result<&str> {
    letter
        .body
        .split("Code: ")
        .nth(1)
        .and_then(|s| s.split_whitespace().next())
        .ok_or_else(|| "the mailed code".into())
}

#[tokio::test]
async fn both_mailed_codes_are_bound_to_the_person_replaced_and_spent() -> Result {
    let (service, rauthy, gail, mut sink) = table_with_mail(true).await?;
    let before = rauthy.users()?;
    let own = service.get("/me", Some(&gail)).await?.1;
    let person = own["person"]["id"].as_str().ok_or("Gail's person")?;
    let ask = json!({"email":"gail@lovelace.test"});
    let (status, pending) = service.post("/me/account/email", Some(&gail), &ask).await?;
    assert_eq!(status, 200, "{pending}");
    assert_eq!(pending["pending"], true);
    assert_eq!(rauthy.users()?, before);
    let first = sink.next().await.ok_or("the new mailbox's code")?;
    let held = sink.next().await.ok_or("the held mailbox's code")?;
    assert_eq!(first.to, vec!["gail@lovelace.test"]);
    assert_eq!(held.to, vec!["gail@example.test"]);
    assert!(first.body.contains("new email"));
    assert!(held.body.contains("email you are leaving"));
    assert_ne!(code(&first)?, code(&held)?);
    for body in [
        json!({}),
        json!({"code":code(&first)?}),
        json!({"held_code":code(&held)?}),
        json!({"code":"wrong", "held_code":code(&held)?}),
        json!({"code":code(&first)?, "held_code":"wrong"}),
    ] {
        let (status, refused) = service
            .post("/me/account/email/confirm", Some(&gail), &body)
            .await?;
        assert_eq!(status, 409, "{refused}");
        assert_eq!(refused["refusal"], "EmailChangeCodeRefused");
        assert_eq!(rauthy.users()?, before);
    }
    assert_eq!(
        service
            .post("/me/account/email", Some(&gail), &ask)
            .await?
            .0,
        200
    );
    let second = sink.next().await.ok_or("the replacement new code")?;
    let next_held = sink.next().await.ok_or("the replacement held code")?;
    assert_ne!(code(&first)?, code(&second)?);
    assert_ne!(code(&held)?, code(&next_held)?);
    for body in [
        json!({"code":code(&first)?, "held_code":code(&held)?}),
        json!({"code":code(&second)?, "held_code":code(&held)?}),
        json!({"code":code(&first)?, "held_code":code(&next_held)?}),
    ] {
        assert_eq!(
            service
                .post("/me/account/email/confirm", Some(&gail), &body)
                .await?
                .1["refusal"],
            "EmailChangeCodeRefused"
        );
    }
    let proof = json!({"code":code(&second)?, "held_code":code(&next_held)?});
    let bea = service
        .sign_in(identity_contract::fake_issuer::Login {
            subject: "other-subject".to_owned(),
            email: "bea@example.test".to_owned(),
        })
        .await?;
    let (status, wrong_person) = service
        .post("/me/account/email/confirm", Some(&bea), &proof)
        .await?;
    assert_eq!(status, 409, "{wrong_person}");
    assert_eq!(wrong_person["refusal"], "EmailChangeCodeRefused");
    assert_eq!(rauthy.users()?, before);
    let (status, changed) = service
        .post("/me/account/email/confirm", Some(&gail), &proof)
        .await?;
    assert_eq!(status, 200, "{changed}");
    assert_eq!(changed["email"], "gail@lovelace.test");
    assert_eq!(rauthy.users()?[0]["email_verified"], true);
    let stored: Value =
        serde_json::from_slice(&std::fs::read(service.dir.path().join("foundings.json"))?)?;
    assert_eq!(stored["admitted_email"][person], "gail@lovelace.test");
    assert_eq!(
        service
            .post("/me/account/email/confirm", Some(&gail), &proof)
            .await?
            .1["refusal"],
        "EmailChangeCodeRefused"
    );
    let subject = before[0]["id"].as_str().ok_or("the confirmed account")?;
    let (back, binding) = service
        .issuer_answer_from(
            identity_contract::fake_issuer::Login {
                subject: subject.to_owned(),
                email: "gail@lovelace.test".to_owned(),
            },
            "provider-1",
        )
        .await?;
    let (_, location, cookie) = service.get_page(&back, &binding).await?;
    assert_eq!(location.as_deref(), Some("/#/me"));
    assert!(cookie, "a confirmed R2 change becomes the R3 held address");
    // The next request must mail the address just confirmed, never the former one.
    assert_eq!(
        service
            .post(
                "/me/account/email",
                Some(&gail),
                &json!({"email":"third@example.test"})
            )
            .await?
            .0,
        200
    );
    let _new = sink.next().await.ok_or("third mailbox")?;
    assert_eq!(
        sink.next().await.ok_or("current held mailbox")?.to,
        vec!["gail@lovelace.test"]
    );
    Ok(())
}

#[tokio::test]
async fn another_persons_held_email_is_refused_before_either_letter() -> Result {
    let (service, rauthy, gail, mut sink) = table_with_mail(true).await?;
    rauthy.remember(
        json!({"id": ADMINISTRATOR, "email": "ada@example.test", "email_verified": true,
        "enabled": true, "language": "en", "roles": [], "user_values": {}}),
    )?;
    let ada = service
        .sign_in(identity_contract::fake_issuer::Login {
            subject: ADMINISTRATOR.to_owned(),
            email: "ada@example.test".to_owned(),
        })
        .await?;
    let (status, admitted) = service
        .post(
            "/people/admit",
            Some(&ada),
            &json!({
                "operation":lys_identity::OperationId::generate()?.to_string(),
                "display_name":"Dean", "email":"dean@example.test",
            }),
        )
        .await?;
    assert_eq!(status, 200, "{admitted}");
    // A fresh account's letter is the issuer's own, so Lys writes none.
    assert_eq!(admitted["letter"]["by"], "issuer", "{admitted}");
    let before = rauthy.users()?;
    let sent = std::fs::read(service.dir.path().join("letters-sent.jsonl")).ok();
    let (status, refused) = service
        .post(
            "/me/account/email",
            Some(&gail),
            &json!({"email":"dean@example.test"}),
        )
        .await?;
    assert_eq!(status, 409, "{refused}");
    assert_eq!(refused["refusal"], "EmailHeldByAnother");
    assert!(refused["reason"].as_str().ok_or("reason")?.contains("Dean"));
    assert_eq!(
        std::fs::read(service.dir.path().join("letters-sent.jsonl")).ok(),
        sent
    );
    assert!(sink.taken().is_none());
    assert_eq!(rauthy.users()?, before);
    Ok(())
}

#[tokio::test]
async fn the_departing_code_goes_to_lys_held_email_even_after_an_issuer_mix_up() -> Result {
    let (service, rauthy, gail, mut sink) = table_with_mail(true).await?;
    let mut user = rauthy.users()?[0].clone();
    let id = user["id"].as_str().ok_or("Gail's account")?.to_owned();
    user["email"] = json!("annabel@example.test");
    reqwest::Client::new()
        .put(format!("{}/users/{id}", rauthy.api()))
        .header(reqwest::header::AUTHORIZATION, format!("API-Key {API_KEY}"))
        .json(&user)
        .send()
        .await?
        .error_for_status()?;
    let (status, pending) = service
        .post(
            "/me/account/email",
            Some(&gail),
            &json!({"email":"new@example.test"}),
        )
        .await?;
    assert_eq!(status, 200, "{pending}");
    let _new = sink.next().await.ok_or("the new code")?;
    let held = sink.next().await.ok_or("the departing code")?;
    assert_eq!(held.to, vec!["gail@example.test"]);
    assert_eq!(rauthy.users()?[0], user);
    Ok(())
}
