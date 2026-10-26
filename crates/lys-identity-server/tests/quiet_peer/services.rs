#![cfg(test)]

use super::Outcome;
use axum::response::IntoResponse;
use identity_contract::harness::{GRANT_MODEL, Service};
use tokio::io::AsyncReadExt;
use tokio::net::TcpListener;

#[tokio::test]
async fn a_quiet_issuer_is_waited_for_until_its_socket_closes() -> Outcome {
    let (service, mut config) = Service::start_adjusted(
        GRANT_MODEL,
        None,
        None,
        None,
        |_| {},
        |config| Ok(config.clone()),
    )
    .await?;
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    config.sign_in_api = Some(format!("http://{}", listener.local_addr()?));
    let mut discovering = Box::pin(lys_identity_server::oidc::Oidc::discover(&config));
    let (mut socket, _) = tokio::select! {
        accepted = listener.accept() => accepted?,
        _ = &mut discovering => return Err("quiet issuer answered".into()),
    };
    let mut bytes = [0; 4096];
    tokio::select! {
        read = socket.read(&mut bytes) => assert!(read? > 0),
        _ = &mut discovering => return Err("quiet issuer answered".into()),
    }
    drop(socket);
    let error = discovering.await.err().ok_or("closed issuer succeeded")?;
    let response = error.into_response();
    assert_eq!(
        response.status(),
        axum::http::StatusCode::SERVICE_UNAVAILABLE
    );
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await?;
    let body: serde_json::Value = serde_json::from_slice(&body)?;
    assert_eq!(body["refusal"], "SignInFailed");
    drop(service);
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn a_quiet_spicedb_is_waited_for_until_its_socket_closes() -> Outcome {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?.to_string();
    let asked_address = address.clone();
    let asking = tokio::task::spawn_blocking(move || {
        lys_identity_server::spicedb_http::Client::default().post_json(
            &asked_address,
            "/v1/schema/read",
            "fixture",
            "{}",
        )
    });
    let (mut socket, _) = listener.accept().await?;
    assert!(socket.read(&mut [0; 4096]).await? > 0);
    drop(socket);
    let error = asking.await?.err().ok_or("closed SpiceDB succeeded")?;
    assert!(error.contains(&address));
    assert!(error.contains("could not be reached"));
    let refusal: lys_identity_server::error::ServerError =
        lys_identity::grants::GrantError::PermissionEngineUnavailable { reason: error }.into();
    let response = refusal.into_response();
    assert_eq!(
        response.status(),
        axum::http::StatusCode::SERVICE_UNAVAILABLE
    );
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await?;
    let body: serde_json::Value = serde_json::from_slice(&body)?;
    assert_eq!(body["refusal"], "PermissionEngineUnavailable");
    Ok(())
}

#[tokio::test]
async fn a_letter_cancel_is_mail_not_sent_and_withdraws_its_wait() -> Outcome {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let directory = tempfile::tempdir()?;
    let sender = super::mailer(listener.local_addr()?.port(), directory.path().join("sent"))?;
    let letter = super::letter("cancel@example.test");
    let mut sending = Box::pin(sender.send_once("cancel-letter", &letter));
    let (mut socket, _) = tokio::select! {
        accepted = listener.accept() => accepted?,
        _ = &mut sending => return Err("quiet mail answered".into()),
    };
    let call = lys_identity_server::waits::snapshot()?
        .into_iter()
        .find(|call| call.subject == "cancel-letter")
        .ok_or("mail is not listed")?;
    assert!(call.started_at > 0);
    lys_identity_server::waits::cancel(&call.id, "administrator Ada")?;
    let refused = sending.await.err().ok_or("cancelled mail succeeded")?;
    assert_eq!(refused.name(), "mail_not_sent");
    assert!(refused.to_string().contains("administrator Ada"));
    assert_eq!(socket.read(&mut [0; 1]).await?, 0);
    assert!(
        !lys_identity_server::waits::snapshot()?
            .iter()
            .any(|entry| entry.id == call.id)
    );
    Ok(())
}
