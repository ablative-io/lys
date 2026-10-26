#![cfg(test)]
//! Provider callbacks compare the issuer account to Lys's durable held address.
use identity_contract::fake_issuer::Login;
use identity_contract::fake_rauthy::{API_KEY, FakeRauthy};
use identity_contract::harness::{ADMINISTRATOR, GRANT_MODEL, Service};
use identity_contract::lys_identity_server as server;
use lys_identity::LoginBinding;
use serde_json::{Value, json};
use server::sign_in_providers::SignInProvidersSettings;

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

async fn table(
    account: &str,
    verified: bool,
    held: Option<&str>,
) -> Result<(Service, FakeRauthy, Login, String)> {
    let rauthy = FakeRauthy::start().await?;
    let provider: Value = reqwest::Client::new()
        .post(format!("{}/providers/create", rauthy.api()))
        .header(reqwest::header::AUTHORIZATION, format!("API-Key {API_KEY}"))
        .json(&json!({"name":"Google", "enabled":true, "client_id":"google-client"}))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    assert_eq!(provider["id"], "provider-1");
    let made: Value = reqwest::Client::new()
        .post(format!("{}/users", rauthy.api()))
        .header(reqwest::header::AUTHORIZATION, format!("API-Key {API_KEY}"))
        .json(
            &json!({"email":account, "email_verified":verified, "enabled":true,
            "language":"en", "roles":[], "user_values":{}}),
        )
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let subject = made["id"].as_str().ok_or("the account")?.to_owned();
    let settings = SignInProvidersSettings {
        api: rauthy.api().to_owned(),
        api_key_file: rauthy.api_key_file(),
    };
    let (service, person) = Service::start_setting(GRANT_MODEL, None, None, Some(settings), |config| {
        server::dev_seed::seed_configured(config, [ADMINISTRATOR, subject.as_str()])?;
        let mut directory = server::routes::open_directory(config)?;
        let person = directory.projection()?.person_for(&LoginBinding::new(&config.issuer, &subject)?)
            .ok_or("the bound person")?.to_string();
        if let Some(held) = held {
            std::fs::write(config.log_dir.with_file_name("foundings.json"), json!({
                "format":crate::foundings_store::FORMAT, "foundings":{}, "admitted_email":{(person.clone()):held},
            }).to_string())?;
        }
        Ok(person)
    }).await?;
    rauthy.remember(
        json!({"id": ADMINISTRATOR, "email": "ada@example.test", "email_verified": true,
        "enabled": true, "language": "en", "roles": [], "user_values": {}}),
    )?;
    Ok((
        service,
        rauthy,
        Login {
            subject,
            email: account.to_owned(),
        },
        person,
    ))
}

async fn callback(service: &Service, login: Login) -> Result<(Option<String>, String)> {
    let (back, binding) = service.issuer_answer_from(login, "provider-1").await?;
    let response = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()?
        .get(format!("{}{back}", service.base))
        .header(reqwest::header::COOKIE, binding)
        .send()
        .await?;
    let cookie = response
        .headers()
        .get(reqwest::header::SET_COOKIE)
        .map(|value| value.to_str().map(str::to_owned))
        .transpose()?;
    let location = response
        .headers()
        .get(reqwest::header::LOCATION)
        .ok_or("the result")?
        .to_str()?
        .to_owned();
    Ok((cookie, location))
}

#[tokio::test]
async fn google_signs_in_as_today_whether_or_not_the_issuer_marks_it_verified() -> Result {
    for verified in [false, true] {
        let (service, _rauthy, login, person) = table(
            "annabel@example.test",
            verified,
            Some(" Annabel@EXAMPLE.test "),
        )
        .await?;
        let (cookie, location) = callback(&service, login).await?;
        assert_eq!(location, "/#/me");
        let cookie =
            cookie.ok_or("the matching account begins a session without an issuer patch")?;
        assert_eq!(
            service.get("/me", Some(&cookie)).await?.1["person"]["id"],
            person
        );
    }
    Ok(())
}

#[tokio::test]
async fn a_google_join_to_another_held_address_never_begins_a_session() -> Result {
    for verified in [false, true] {
        let (service, _rauthy, login, _) =
            table("annabel@example.test", verified, Some("dean@example.test")).await?;
        let before = std::fs::read(service.dir.path().join("foundings.json"))?;
        let (cookie, location) = callback(&service, login).await?;
        assert!(cookie.is_none());
        assert!(
            location.contains("refused=ProviderEmailMismatch"),
            "{location}"
        );
        let url = reqwest::Url::parse(&format!(
            "http://lys.test/{}",
            location.trim_start_matches("/#/")
        ))?;
        let fields: std::collections::BTreeMap<_, _> = url.query_pairs().into_owned().collect();
        assert_eq!(fields["provider"], "Google");
        assert_eq!(fields["account_email"], "a…@example.test");
        assert_eq!(fields["held_email"], "d…@example.test");
        assert!(!location.contains("annabel@"));
        assert!(!location.contains("dean@"));
        assert_eq!(
            std::fs::read(service.dir.path().join("foundings.json"))?,
            before
        );
    }
    Ok(())
}

#[tokio::test]
async fn the_first_provider_sign_in_records_a_missing_address_durably_and_labels_the_row() -> Result
{
    let (mut service, rauthy, login, person) = table("annabel@example.test", false, None).await?;
    let (cookie, _) = callback(&service, login.clone()).await?;
    assert!(cookie.is_some());
    let stored: Value =
        serde_json::from_slice(&std::fs::read(service.dir.path().join("foundings.json"))?)?;
    assert_eq!(stored["admitted_email"][&person], "annabel@example.test");
    let ada = service
        .sign_in(Login {
            subject: ADMINISTRATOR.to_owned(),
            email: "ada@example.test".to_owned(),
        })
        .await?;
    let (_, rows) = service.get("/directory/people", Some(&ada)).await?;
    let row = rows["people"]
        .as_array()
        .ok_or("people")?
        .iter()
        .find(|row| row["id"] == person)
        .ok_or("Annabel's row")?;
    assert_eq!(row["email_recorded_at_sign_in"], true);
    service.restart().await?;
    let mut user = rauthy.users()?[0].clone();
    user["email"] = json!("dean@example.test");
    reqwest::Client::new()
        .put(format!("{}/users/{}", rauthy.api(), login.subject))
        .header(reqwest::header::AUTHORIZATION, format!("API-Key {API_KEY}"))
        .json(&user)
        .send()
        .await?
        .error_for_status()?;
    let (cookie, location) = callback(
        &service,
        Login {
            email: "dean@example.test".to_owned(),
            ..login
        },
    )
    .await?;
    assert!(cookie.is_none());
    assert!(location.contains("ProviderEmailMismatch"));
    Ok(())
}

#[tokio::test]
async fn an_administrators_email_change_replaces_the_held_address_for_provider_sign_in() -> Result {
    let (service, _rauthy, login, person) =
        table("dean@example.test", false, Some("dean@example.test")).await?;
    let ada = service
        .sign_in(Login {
            subject: ADMINISTRATOR.to_owned(),
            email: "ada@example.test".to_owned(),
        })
        .await?;
    let (status, changed) = service
        .post(
            &format!("/directory/people/{person}/account/email"),
            Some(&ada),
            &json!({"email":"annabel@example.test"}),
        )
        .await?;
    assert_eq!(status, 200, "{changed}");
    let stored: Value =
        serde_json::from_slice(&std::fs::read(service.dir.path().join("foundings.json"))?)?;
    assert_eq!(stored["admitted_email"][&person], "annabel@example.test");
    let (cookie, location) = callback(
        &service,
        Login {
            email: "annabel@example.test".to_owned(),
            ..login
        },
    )
    .await?;
    assert!(cookie.is_some(), "{location}");
    Ok(())
}

#[tokio::test]
async fn a_first_provider_sign_in_that_cannot_save_the_held_address_begins_no_session() -> Result {
    let (service, _rauthy, login, _) = table("annabel@example.test", false, None).await?;
    std::fs::create_dir(service.dir.path().join("foundings.json"))?;
    let (cookie, location) = callback(&service, login).await?;
    assert!(cookie.is_none());
    assert!(
        location.contains("refused=DirectoryUnavailable"),
        "{location}"
    );
    Ok(())
}
