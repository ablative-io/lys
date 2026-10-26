#![cfg(test)]

//! The quiet person's mail cannot hold their admission or another person's mail.

use super::Outcome;
use identity_contract::fake_rauthy::{API_KEY, FakeRauthy};
use identity_contract::harness::{ADMINISTRATOR, GRANT_MODEL, Service};
use lys_identity::{Actor, AuthMethod, LoginBinding, Provenance};
use lys_identity_server::{routes::open_directory, session::Sessions};
use serde_json::json;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

#[tokio::test]
async fn quiet_email_change_releases_admission_and_another_persons_change() -> Outcome {
    let quiet = TcpListener::bind("127.0.0.1:0").await?;
    let rauthy = FakeRauthy::start().await?;
    let http = reqwest::Client::new();
    let google: serde_json::Value = http
        .post(format!("{}/providers/create", rauthy.api()))
        .header("authorization", format!("API-Key {API_KEY}"))
        .json(&json!({"name":"Google", "enabled":true, "client_id":"google-client"}))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let provider = google["id"].as_str().ok_or("no provider id")?;
    let mut subjects = Vec::new();
    for email in ["a@example.test", "b@example.test"] {
        let user: serde_json::Value = http
            .post(format!("{}/users", rauthy.api()))
            .header("authorization", format!("API-Key {API_KEY}"))
            .json(
                &json!({"email":email, "given_name":"Person", "language":"en", "roles":[],
                "enabled":true, "email_verified":true, "user_values":{},
                "auth_provider_id":provider, "federation_uid":email}),
            )
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        subjects.push(user["id"].as_str().ok_or("no account id")?.to_owned());
    }
    let port = quiet.local_addr()?.port();
    let (service, cookies) = Service::start_adjusted(
        GRANT_MODEL,
        None,
        None,
        Some(
            lys_identity_server::sign_in_providers::SignInProvidersSettings {
                api: rauthy.api().to_owned(),
                api_key_file: rauthy.api_key_file(),
            },
        ),
        move |config| {
            config.mail = Some(lys_identity_server::mail::MailSettings {
                host: "127.0.0.1".to_owned(),
                port,
                user: None,
                password_file: None,
                from: "Lys <lys@example.test>".to_owned(),
                tls: lys_identity_server::mail::Tls::None,
                sent_file: config.log_dir.with_file_name("sent"),
            })
        },
        |config| {
            lys_identity_server::dev_seed::seed_configured(
                config,
                [ADMINISTRATOR, &subjects[0], &subjects[1]],
            )?;
            let mut directory = open_directory(config)?;
            let mut held = serde_json::Map::new();
            let sessions = Sessions::open(
                config.sessions_file.clone().ok_or("no sessions")?,
                600,
                false,
            )?;
            let mut cookies = Vec::new();
            for (id, email) in subjects.iter().zip(["a@example.test", "b@example.test"]) {
                let binding = LoginBinding::new(&config.issuer, id)?;
                let person = directory
                    .projection()?
                    .person_for(&binding)
                    .ok_or("no person")?;
                held.insert(person.to_string(), json!(email));
                cookies.push(sessions.begin(Actor::new(
                    binding,
                    Provenance::new(AuthMethod::Oidc, lys_identity_server::session::now()),
                ))?);
            }
            std::fs::write(
                config.log_dir.with_file_name("foundings.json"),
                json!({
                    "format":"lys-foundings/1", "foundings":{}, "admitted_email":held,
                })
                .to_string(),
            )?;
            Ok(cookies)
        },
    )
    .await?;
    let mut caller = TcpStream::connect(service.base.trim_start_matches("http://")).await?;
    let body = json!({"email":"a@next.test"}).to_string();
    caller.write_all(format!("POST /me/account/email HTTP/1.1\r\nHost: lys.test\r\nCookie: {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}", cookies[0], body.len()).as_bytes()).await?;
    let (mut first, _) = quiet.accept().await?;
    // A fresh admission for A is read at the issuer and completes while A's mail waits.
    let (callback, browser) = service
        .issuer_answer_from(
            identity_contract::fake_issuer::Login {
                subject: subjects[0].clone(),
                email: "a@example.test".to_owned(),
            },
            provider,
        )
        .await?;
    let (status, answer) = service.get(&callback, Some(&browser)).await?;
    assert_eq!(status, 200, "{answer}");
    let change = json!({"email":"b@next.test"});
    let second = service.post("/me/account/email", Some(&cookies[1]), &change);
    let mut sink = identity_contract::mail_sink::MailSink::start().await?;
    // B's two connections answer through a second transport; A's listener never writes.
    let relaying = async {
        for _letter in 0..2 {
            let (mut socket, _) = quiet.accept().await?;
            let mut answering = TcpStream::connect(("127.0.0.1", sink.port())).await?;
            tokio::io::copy_bidirectional(&mut socket, &mut answering).await?;
        }
        Ok::<_, std::io::Error>(())
    };
    let (answer, relayed) = tokio::join!(second, relaying);
    relayed?;
    let (status, answer) = answer?;
    assert_eq!(status, 200, "{answer}");
    assert!(sink.next().await.is_some());
    assert!(sink.next().await.is_some());
    caller.shutdown().await?;
    drop(caller);
    assert_eq!(first.read(&mut [0; 1]).await?, 0);
    let (status, answer) = service
        .post(
            "/me/account/email/confirm",
            Some(&cookies[0]),
            &json!({"code":"withdrawn", "held_code":"withdrawn"}),
        )
        .await?;
    assert_eq!(status, 409, "{answer}");
    assert_eq!(answer["refusal"], "EmailChangeCodeRefused");
    Ok(())
}
