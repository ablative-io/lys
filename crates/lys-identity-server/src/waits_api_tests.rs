#![cfg(test)]

use std::error::Error;

use identity_contract::apps::{BEA, login, seeded};
use identity_contract::harness::ADMINISTRATOR;
use identity_contract::lys_identity_server::waits::{Call, snapshot};

#[tokio::test]
async fn waits_are_oldest_first_and_an_administrators_cancel_names_them()
-> Result<(), Box<dyn Error>> {
    let (service, _) = seeded().await?;
    let cookie = service.sign_in(login(ADMINISTRATOR)).await?;
    let first = Call::begin("mail", "quiet.test:25", "email_change", "waits-test-a")?;
    let second = Call::begin("issuer", "issuer.test", "read", "waits-test-b")?;
    let (status, answer) = service.get("/waits", Some(&cookie)).await?;
    assert_eq!(status, 200, "{answer}");
    let entries = answer["waits"].as_array().ok_or("no waits list")?;
    let ids: Vec<_> = entries
        .iter()
        .map(|entry| entry["id"].as_str().unwrap())
        .collect();
    assert!(
        ids.iter().position(|id| *id == first.id()).unwrap()
            < ids.iter().position(|id| *id == second.id()).unwrap()
    );
    let (status, answer) = service
        .post(
            &format!("/waits/{}/cancel", first.id()),
            Some(&cookie),
            &serde_json::json!({}),
        )
        .await?;
    assert_eq!(status, 200, "{answer}");
    let refused = first
        .run(std::future::pending::<Result<(), String>>())
        .await
        .unwrap_err();
    assert!(refused.contains("administrator"));
    assert!(refused.contains("cancel"));
    drop(second);
    assert!(
        !snapshot()?
            .iter()
            .any(|entry| entry.subject.starts_with("waits-test-"))
    );
    Ok(())
}

#[tokio::test]
async fn a_non_administrator_cannot_read_or_cancel_waits() -> Result<(), Box<dyn Error>> {
    let (service, _) = seeded().await?;
    let cookie = service.sign_in(login(BEA)).await?;
    let call = Call::begin("mail", "quiet.test:25", "send", "private-wait")?;
    for (status, body) in [
        service.get("/waits", Some(&cookie)).await?,
        service
            .post(
                &format!("/waits/{}/cancel", call.id()),
                Some(&cookie),
                &serde_json::json!({}),
            )
            .await?,
    ] {
        assert_eq!(status, 403, "{body}");
        assert_eq!(body["refusal"], "NotAdmitted");
    }
    Ok(())
}

#[tokio::test]
async fn cancelling_a_listed_quiet_smtp_call_answers_mail_not_sent_with_the_administrator()
-> Result<(), Box<dyn Error>> {
    use identity_contract::lys_identity_server::{mail, mail_letters::Letter};
    use tokio::io::AsyncReadExt;
    let (service, _) = seeded().await?;
    let cookie = service.sign_in(login(ADMINISTRATOR)).await?;
    let quiet = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let root = tempfile::tempdir()?;
    let sender = mail::Mailer::open(&mail::MailSettings {
        host: "127.0.0.1".to_owned(),
        port: quiet.local_addr()?.port(),
        user: None,
        password_file: None,
        from: "Lys <lys@example.test>".to_owned(),
        tls: mail::Tls::None,
        sent_file: root.path().join("sent"),
    })?;
    let letter = Letter::EmailChange {
        email: "waiting@example.test".to_owned(),
        code: "proof".to_owned(),
        leaving: false,
    };
    let mut sending = Box::pin(sender.send_once("api-cancel-letter", &letter));
    let (mut peer, _) = tokio::select! {
        accepted = quiet.accept() => accepted?,
        _ = &mut sending => return Err("quiet mail answered".into()),
    };
    let (status, view) = service.get("/waits", Some(&cookie)).await?;
    assert_eq!(status, 200, "{view}");
    let entry = view["waits"]
        .as_array()
        .ok_or("waits absent")?
        .iter()
        .find(|entry| entry["subject"] == "api-cancel-letter")
        .ok_or("quiet SMTP absent")?;
    assert_eq!(entry["service"], "mail");
    assert!(entry["started_at"].as_u64().ok_or("start absent")? > 0);
    let id = entry["id"].as_str().ok_or("identity absent")?;
    let (status, receipt) = service
        .post(
            &format!("/waits/{id}/cancel"),
            Some(&cookie),
            &serde_json::json!({}),
        )
        .await?;
    assert_eq!(status, 200, "{receipt}");
    let refusal = sending.await.err().ok_or("cancelled mail succeeded")?;
    assert_eq!(refusal.name(), "mail_not_sent");
    assert!(
        refusal.to_string().contains(
            receipt["by"]
                .as_str()
                .ok_or("cancel did not name administrator")?
        )
    );
    assert_eq!(peer.read(&mut [0; 1]).await?, 0);
    assert!(!snapshot()?.iter().any(|call| call.id == id));
    Ok(())
}

#[tokio::test]
async fn caller_close_reaches_a_synchronous_gateway_call_and_withdraws_its_entry()
-> Result<(), Box<dyn Error>> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let quiet = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let peer = quiet.local_addr()?.to_string();
    let gateway = peer.clone();
    let app = axum::Router::new()
        .route(
            "/call",
            axum::routing::get(move || {
                let gateway = gateway.clone();
                async move {
                    match crate::spicedb_outbound::post_json(
                        &crate::spicedb_http::Client::default(),
                        &gateway,
                        "/read",
                        "fixture",
                        "{}",
                    ) {
                        Ok(answer) => answer.body,
                        Err(error) => error,
                    }
                }
            }),
        )
        .layer(axum::middleware::from_fn(crate::wait_request::observe));
    let listening = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let address = listening.local_addr()?;
    let serving = tokio::spawn(async move { axum::serve(listening, app).await });
    let mut caller = tokio::net::TcpStream::connect(address).await?;
    caller
        .write_all(b"GET /call HTTP/1.1\r\nHost: fixture\r\n\r\n")
        .await?;
    let (mut socket, _) = quiet.accept().await?;
    assert!(socket.read(&mut [0; 4096]).await? > 0);
    assert!(
        crate::waits::snapshot()?
            .iter()
            .any(|call| call.peer == peer)
    );
    caller.shutdown().await?;
    drop(caller);
    assert_eq!(socket.read(&mut [0; 1]).await?, 0);
    // The transport's EOF is followed by the worker settling its outcome.
    let (mut generation, mut calls) = crate::waits::after(None).await?;
    while calls.iter().any(|call| call.peer == peer) {
        (generation, calls) = crate::waits::after(Some(generation)).await?;
    }
    assert!(
        !crate::waits::snapshot()?
            .iter()
            .any(|call| call.peer == peer)
    );
    serving.abort();
    assert!(serving.await.unwrap_err().is_cancelled());
    Ok(())
}
