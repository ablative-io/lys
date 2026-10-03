#![cfg(test)]
//! Gates on the open-call journal: a call is journalled before it is sent,
//! a restart records each call left open as `lost` once and never over an
//! outcome already durable, and a journal that cannot be written refuses a
//! call before it is sent or holds it `unrecorded` after.

use std::os::unix::fs::PermissionsExt;
use std::sync::Arc;
use std::sync::atomic::Ordering;

use http_body_util::BodyExt;
use hyper::StatusCode;
use tokio::sync::mpsc as channel;

use crate::proxy::forward::{Base, Proxy, ProxyConfig};
use crate::proxy::forward_tests::{
    Harness, KEY, Res, fake, message_response, messages_request, send, whole,
};
use crate::proxy::journal::{Journal, OpenCall, recover};
use crate::record::Home;
use crate::record::call::{Api, CallStatus, OutcomeMeta, ingest_outcome};

/// A journal record as the proxy writes one before forwarding.
fn open_call(call_id: &str, session: Option<&str>) -> OpenCall {
    OpenCall {
        call_id: call_id.to_owned(),
        provider: "anthropic".to_owned(),
        api: Api::Messages,
        started_at: "2026-09-28T10:00:00.000Z".to_owned(),
        session: session.map(str::to_owned),
        admission_ns: None,
        completed: None,
    }
}

#[test]
fn a_restart_records_each_open_call_lost_once_with_what_was_spooled() -> Res {
    let dir = tempfile::tempdir()?;
    let home = Home::open(dir.path().join("home"))?;
    let journal = Journal::open(dir.path().join("journal"))?;
    let capture = dir.path().join("capture");
    std::fs::create_dir_all(&capture)?;
    journal.write(&open_call("c1", Some(KEY)))?;
    journal.write(&open_call("c2", None))?;
    std::fs::write(capture.join("c1.request"), b"{\"model\": \"m\", \"messa")?;
    let first = recover(&home, &journal, &capture)?;
    assert_eq!(first.len(), 2);
    assert!(
        first
            .iter()
            .all(|r| r.status == CallStatus::Lost && r.entry_id.is_some())
    );
    assert_eq!(first[0].session, KEY);
    assert_eq!(first[1].session, "unlinked-2026-09-28");
    assert_eq!(journal.open_calls()?.len(), 0);
    assert!(!capture.join("c1.request").exists());
    let second = recover(&home, &journal, &capture)?;
    assert!(second.is_empty());
    let session = home.open_session(KEY)?;
    let calls = session.customs_everywhere(crate::record::entries::CUSTOM_CALL)?;
    assert_eq!(calls.len(), 1);
    Ok(())
}

#[test]
fn a_call_whose_outcome_was_durable_is_not_recorded_lost() -> Res {
    let dir = tempfile::tempdir()?;
    let home = Home::open(dir.path().join("home"))?;
    let journal = Journal::open(dir.path().join("journal"))?;
    let capture = dir.path().join("capture");
    std::fs::create_dir_all(&capture)?;
    {
        let mut session = home.create_session(KEY, "", None)?;
        let meta = OutcomeMeta {
            call_id: "c9".to_owned(),
            provider: "anthropic".to_owned(),
            api: Api::Messages,
            status: CallStatus::Cancelled,
            started_at: "2026-09-28T10:00:00.000Z".to_owned(),
            duration_ms: Some(3),
            stream: true,
        };
        ingest_outcome(&mut session, &home.blocks()?, &meta, None, None)?;
    }
    // The process died after the entry and before the record was retired.
    journal.write(&open_call("c9", Some(KEY)))?;
    let reports = recover(&home, &journal, &capture)?;
    assert_eq!(reports.len(), 1);
    assert!(reports[0].already_recorded);
    let session = home.open_session(KEY)?;
    let calls = session.customs_everywhere(crate::record::entries::CUSTOM_CALL)?;
    assert_eq!(calls.len(), 1);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_call_open_when_the_proxy_stops_is_recorded_lost_by_the_next_start() -> Res {
    let (arrived_tx, mut arrived_rx) = channel::channel::<()>(1);
    let (upstream, _) = fake(move || {
        let arrived_tx = arrived_tx.clone();
        async move {
            // The upstream never answers: the call is open when the next start comes.
            if arrived_tx.send(()).await.is_ok() {
                std::future::pending::<()>().await;
            }
            whole(StatusCode::OK, &message_response())
        }
    })
    .await?;
    let harness = Harness::start(upstream).await?;
    let addr = harness.addr;
    let in_flight =
        tokio::spawn(async move { send(addr, messages_request(Some(KEY), false)?).await });
    arrived_rx
        .recv()
        .await
        .ok_or("the upstream saw no request")?;
    drop(harness.proxy.sink().pause()?);
    assert_eq!(std::fs::read_dir(harness.state("journal"))?.count(), 1);
    let base = Base::parse(&format!("http://{upstream}"))?;
    let restarted = Proxy::start(ProxyConfig {
        home: harness.dir.path().join("home"),
        state: harness.dir.path().join("state"),
        anthropic: base.clone(),
        openai: base,
    })?;
    assert_eq!(restarted.lost.len(), 1);
    assert_eq!(restarted.lost[0].status, CallStatus::Lost);
    assert_eq!(restarted.lost[0].session, KEY);
    let calls = harness.calls(KEY)?;
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].status, CallStatus::Lost);
    assert_eq!(calls[0].model.as_deref(), Some("claude-test-model"));
    in_flight.abort();
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_journal_that_cannot_be_written_refuses_the_call_before_it_is_sent() -> Res {
    let (upstream, count) = fake(|| async { whole(StatusCode::OK, &message_response()) }).await?;
    let harness = Harness::start(upstream).await?;
    let journal = harness.state("journal");
    std::fs::set_permissions(&journal, std::fs::Permissions::from_mode(0o500))?;
    let sent = send(harness.addr, messages_request(Some(KEY), false)?).await;
    std::fs::set_permissions(&journal, std::fs::Permissions::from_mode(0o700))?;
    let (response, _connection) = sent?;
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let text = http_body_util::BodyExt::collect(response.into_body())
        .await?
        .to_bytes();
    assert!(String::from_utf8_lossy(&text).contains("open-call journal could not be written"));
    assert_eq!(count.load(Ordering::SeqCst), 0);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_journal_lost_after_admission_forwards_and_records_unrecorded_once_writable() -> Res {
    let (go_tx, go_rx) = channel::channel::<()>(1);
    let (arrived_tx, mut arrived_rx) = channel::channel::<()>(1);
    let go_rx = Arc::new(tokio::sync::Mutex::new(go_rx));
    let (upstream, _) = fake(move || {
        let go_rx = Arc::clone(&go_rx);
        let arrived_tx = arrived_tx.clone();
        async move {
            if arrived_tx.send(()).await.is_ok() {
                go_rx.lock().await.recv().await;
            }
            whole(StatusCode::OK, &message_response())
        }
    })
    .await?;
    let harness = Harness::start(upstream).await?;
    let addr = harness.addr;
    let call = tokio::spawn(async move {
        let (response, _connection) = send(addr, messages_request(Some(KEY), false)?).await?;
        let body = http_body_util::BodyExt::collect(response.into_body()).await?;
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(body.to_bytes())
    });
    arrived_rx
        .recv()
        .await
        .ok_or("the upstream saw no request")?;
    let journal = harness.state("journal");
    std::fs::set_permissions(&journal, std::fs::Permissions::from_mode(0o500))?;
    go_tx.send(()).await?;
    let received = call.await??;
    let held = harness.report()?;
    std::fs::set_permissions(&journal, std::fs::Permissions::from_mode(0o700))?;
    assert_eq!(
        received,
        hyper::body::Bytes::from(message_response().to_string())
    );
    assert_eq!(held.status, CallStatus::Unrecorded);
    assert!(held.held.is_some());
    assert!(held.entry_id.is_none());
    harness.proxy.sink().settle()?;
    let recorded = harness.report()?;
    assert_eq!(recorded.status, CallStatus::Unrecorded);
    assert!(recorded.held.is_none());
    assert!(recorded.retired);
    let calls = harness.calls(KEY)?;
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].status, CallStatus::Unrecorded);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_read_only_capture_directory_is_unrecorded_and_the_client_gets_it_all() -> Res {
    use std::os::unix::fs::PermissionsExt;
    let (go_tx, go_rx) = channel::channel::<()>(1);
    let (arrived_tx, mut arrived_rx) = channel::channel::<()>(1);
    let go_rx = Arc::new(tokio::sync::Mutex::new(go_rx));
    let (upstream, _) = fake(move || {
        let go_rx = Arc::clone(&go_rx);
        let arrived_tx = arrived_tx.clone();
        async move {
            if arrived_tx.send(()).await.is_ok() {
                go_rx.lock().await.recv().await;
            }
            whole(StatusCode::OK, &message_response())
        }
    })
    .await?;
    let harness = Harness::start(upstream).await?;
    let addr = harness.addr;
    let call = tokio::spawn(async move {
        let (response, _connection) = send(addr, messages_request(Some(KEY), false)?).await?;
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(
            response.into_body().collect().await?.to_bytes(),
        )
    });
    arrived_rx
        .recv()
        .await
        .ok_or("the upstream saw no request")?;
    drop(harness.proxy.sink().pause()?);
    let capture = harness.state("capture");
    std::fs::set_permissions(&capture, std::fs::Permissions::from_mode(0o500))?;
    go_tx.send(()).await?;
    let received = call.await??;
    let report = harness.report()?;
    std::fs::set_permissions(&capture, std::fs::Permissions::from_mode(0o700))?;
    assert_eq!(
        received,
        hyper::body::Bytes::from(message_response().to_string())
    );
    assert_eq!(report.status, CallStatus::Unrecorded);
    let calls = harness.calls(KEY)?;
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].status, CallStatus::Unrecorded);
    assert!(calls[0].response.is_empty());
    assert!(calls[0].raw_response.is_none());
    // A body the store never took claims no durability: the record says so by name.
    let timing = calls[0]
        .capture
        .as_ref()
        .ok_or("an unrecorded call keeps its capture timing")?;
    assert_eq!(
        timing.durable,
        crate::record::call::captured::DurableTime::NotPlaced
    );
    assert!(report.held.is_none());
    assert_eq!(report.spool_kept, 1);
    assert!(!report.retired);
    let timing = calls[0]
        .capture
        .as_ref()
        .ok_or("capture measurements absent")?;
    assert!(
        timing
            .refusals
            .iter()
            .any(|reason| reason.contains("Permission denied"))
    );
    let recovered = recover(
        &harness.home()?,
        &Journal::open(harness.state("journal"))?,
        &capture,
    )?;
    assert_eq!(recovered.len(), 1);
    assert!(recovered[0].already_recorded);
    assert!(recovered[0].retired);
    assert_eq!(std::fs::read_dir(capture)?.count(), 0);
    assert_eq!(harness.calls(KEY)?.len(), 1);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_whole_large_response_stays_complete_after_journal_recovery() -> Res {
    let mut sent = vec![b'\n'; 20 * 1024 * 1024];
    sent.extend_from_slice(super::capture_decode_tests::PLAIN);
    let harness =
        super::capture_decode_tests::capture_whole(None, &sent, CallStatus::Complete, 65536)
            .await?;
    let calls = harness.calls(KEY)?;
    let home = harness.home()?;
    let journal = Journal::open(harness.state("journal"))?;
    journal.write(&open_call(&calls[0].call_id, Some(KEY)))?;
    let reports = recover(&home, &journal, &harness.state("capture"))?;
    assert_eq!(reports.len(), 1);
    assert!(reports[0].already_recorded);
    let recovered = harness.calls(KEY)?;
    assert_eq!(recovered.len(), 1);
    assert_eq!(recovered[0].status, CallStatus::Complete);
    let raw = recovered[0]
        .raw_response
        .as_ref()
        .ok_or("response absent")?;
    let bytes = home
        .blocks()?
        .get(&crate::record::blocks::Hash::parse(raw)?)?;
    assert_eq!(bytes.len(), sent.len());
    assert_eq!(bytes, sent);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_stalled_capture_worker_does_not_hold_or_spool_the_client_response() -> Res {
    let (upstream, _) = fake(|| async { whole(StatusCode::OK, &message_response()) }).await?;
    let harness = Harness::start(upstream).await?;
    let release = harness.proxy.sink().pause()?;
    let (response, connection) = send(harness.addr, messages_request(Some(KEY), false)?).await?;
    let received = response.into_body().collect().await?.to_bytes();
    let spools = std::fs::read_dir(harness.state("capture"))?.count();
    drop(release);
    let report = harness.report()?;
    connection.await?;
    assert_eq!(
        received,
        hyper::body::Bytes::from(message_response().to_string())
    );
    assert_eq!(
        spools, 0,
        "forwarding must do no spool work while the worker is stopped"
    );
    assert_eq!(report.status, CallStatus::Complete);
    let calls = harness.calls(KEY)?;
    let timing = calls[0]
        .capture
        .as_ref()
        .ok_or("capture measurements absent")?;
    assert!(timing.admission_ns.is_some());
    let crate::record::call::captured::DurableTime::Measured(body_ns) = timing.durable else {
        return Err("body durability measurement absent".into());
    };
    assert!(
        report
            .time_to_record_ns
            .ok_or("entry append measurement absent")?
            >= body_ns
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn closing_the_fixture_releases_the_proxy_before_returning() -> Res {
    let (upstream, _) = fake(|| async { whole(StatusCode::OK, &message_response()) }).await?;
    let harness = Harness::start(upstream).await?;
    let proxy = Arc::downgrade(&harness.proxy);
    drop(harness);
    assert!(
        proxy.upgrade().is_none(),
        "the fixture left its server owning the proxy"
    );
    Ok(())
}

#[test]
fn a_stopped_capture_worker_closes_its_reports_before_shutdown_returns() -> Res {
    let dir = tempfile::tempdir()?;
    let home = Home::open(dir.path().join("home"))?;
    let journal = Journal::open(dir.path().join("journal"))?;
    let (reports, received) = std::sync::mpsc::channel();
    let sink = crate::proxy::journal::Sink::start(home, journal, reports);
    sink.shutdown()?;
    assert!(matches!(
        received.try_recv(),
        Err(std::sync::mpsc::TryRecvError::Disconnected)
    ));
    Ok(())
}
