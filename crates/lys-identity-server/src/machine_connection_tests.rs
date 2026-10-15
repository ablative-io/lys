//! A signed runner reports its schedule; only its owner can interrupt the live control wait.
use super::*;
use axum::body::{Body, to_bytes};
use axum::http::{Request, header};
use lys_core::Ed25519Identity;
use lys_identity::{Actor, AuthMethod, Provenance};
use lys_runner::dial::{EPOCH_HEADER, NONCE_HEADER, SIGNATURE_HEADER, dial_signed_bytes};
use lys_runner::protocol::{hex, nonce};
use std::error::Error;
use std::future::Future;
use std::task::{Context, Waker};
use tower::ServiceExt;

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

async fn signed_report(state: &Arc<AppState>, key: &Ed25519Identity, status: Value) -> TestResult {
    let route = "/runner/dial/own/connection";
    let body = serde_json::to_vec(&status)?;
    let epoch = state.runners.hub().epoch();
    let nonce = nonce();
    let signature = hex(&key.sign(&dial_signed_bytes("POST", route, &epoch, &nonce, &body)));
    let response = super::routes()
        .with_state(Arc::clone(state))
        .oneshot(
            Request::post(route)
                .header(EPOCH_HEADER, epoch)
                .header(NONCE_HEADER, nonce)
                .header(SIGNATURE_HEADER, signature)
                .body(Body::from(body))?,
        )
        .await?;
    assert_eq!(
        response.status(),
        200,
        "the signed connection report was refused"
    );
    Ok(())
}

async fn owner_request(
    state: &Arc<AppState>,
    cookie: &str,
    path: &str,
    method: &str,
) -> TestResult<Value> {
    let response = crate::machine_folders::routes()
        .with_state(Arc::clone(state))
        .oneshot(
            Request::builder()
                .method(method)
                .uri(path)
                .header(header::COOKIE, cookie)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(
        response.status(),
        200,
        "the owner's runner connection route was unavailable"
    );
    Ok(serde_json::from_slice(
        &to_bytes(response.into_body(), 16384).await?,
    )?)
}

#[tokio::test]
async fn the_owner_reads_the_signed_schedule_and_reconnect_releases_the_control_stream()
-> TestResult {
    let (directory, state, runner, _) =
        super::super::machine_folder_admission_tests::fixture().await?;
    let key = Ed25519Identity::load(&state.grant_setup.key_file)?;
    crate::network_api::with_network(&state, |store| {
        store.name_runner(
            "own",
            Some(RunnerRecord::Dialled {
                key: hex(&key.public_key_bytes()),
                runner: None,
            }),
        )
    })?;
    let actor = Actor::new(
        state
            .admission
            .administrator_login()?
            .ok_or("administrator absent")?,
        Provenance::new(AuthMethod::Oidc, crate::session::now()),
    );
    let cookie = state.sessions.begin(actor)?;
    let cookie = cookie.split(';').next().ok_or("cookie absent")?;
    let status = json!({"state":"refused", "retry":{"refused_at":1000,"next_dial":4000,"attempt":3,"reason":"The connection was refused."}});
    signed_report(&state, &key, status.clone()).await?;
    let reported = owner_request(&state, cookie, "/network/machines/own/connection", "GET").await?;
    assert_eq!(reported, status);
    let control = state.runners.hub().control_after("own", 0);
    tokio::pin!(control);
    let mut context = Context::from_waker(Waker::noop());
    assert!(control.as_mut().poll(&mut context).is_pending());
    let reply = owner_request(&state, cookie, "/network/machines/own/reconnect", "POST").await?;
    assert_eq!(reply["reconnecting"], true);
    assert_eq!(control.await?, 1);
    let absent = state
        .runners
        .hub()
        .reconnect("own")
        .err()
        .ok_or("an absent control stream was reported as signalled")?;
    assert_eq!(absent.name(), "runner_reconnect_unavailable");
    runner.stop()?;
    drop(directory);
    Ok(())
}

#[tokio::test]
async fn an_unsigned_connection_report_is_refused_without_changing_status() -> TestResult {
    let (directory, state, runner, _) =
        super::super::machine_folder_admission_tests::fixture().await?;
    let response = super::routes()
        .with_state(Arc::clone(&state))
        .oneshot(Request::post("/runner/dial/own/connection").body(Body::from("{}"))?)
        .await?;
    assert!(response.status().is_client_error());
    assert_eq!(
        state.runners.hub().connection_status("own")?["state"],
        "unreported"
    );
    runner.stop()?;
    drop(directory);
    Ok(())
}

#[tokio::test]
async fn a_local_runner_without_bridge_control_refuses_reconnect_by_name() -> TestResult {
    let (_directory, state, runner, _) =
        super::super::machine_folder_admission_tests::fixture().await?;
    let actor = Actor::new(
        state
            .admission
            .administrator_login()?
            .ok_or("administrator absent")?,
        Provenance::new(AuthMethod::Oidc, crate::session::now()),
    );
    let cookie = state.sessions.begin(actor)?;
    let cookie = cookie.split(';').next().ok_or("cookie absent")?;
    for (method, path) in [
        ("GET", "/network/machines/own/connection"),
        ("POST", "/network/machines/own/reconnect"),
    ] {
        let response = crate::machine_folders::routes()
            .with_state(Arc::clone(&state))
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri(path)
                    .header(header::COOKIE, cookie)
                    .body(Body::empty())?,
            )
            .await?;
        assert!(response.status().is_client_error());
        let body: Value = serde_json::from_slice(&to_bytes(response.into_body(), 16384).await?)?;
        assert_eq!(body["refusal"], "runner_reconnect_control_failed");
    }
    runner.stop()?;
    Ok(())
}
