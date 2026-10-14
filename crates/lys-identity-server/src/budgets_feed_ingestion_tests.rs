//! A refused entry never discards the following entries or acknowledges a failed store.

use super::super::{
    entry_tests::{entry, fixture, page},
    keep_page,
};
use crate::budgets_api::with_budgets;
use crate::error::ServerError;
use crate::network_store::Retirement;
use lys_runner::operations::{OperationOutcome, OperationState};
use lys_runner::tracking_store::{Body, Commit, FeedEntry};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[tokio::test]
async fn four_retired_runner_aliases_cannot_charge_the_live_writers_entry() -> TestResult {
    let table = fixture().await?;
    table.track("live", "machine")?;
    {
        let mut store = table
            .state
            .network
            .as_ref()
            .ok_or("missing fixture network")?
            .lock()
            .map_err(|error| error.to_string())?;
        let template = store
            .machine("machine")
            .ok_or("missing fixture machine")?
            .clone();
        let runner = store
            .runner("machine")
            .ok_or("missing fixture runner")?
            .clone();
        for index in 0..4 {
            let id = format!("proof-{index}");
            store.name(crate::network_store::Machine {
                id: id.clone(),
                name: format!("077 proof seat {index}"),
                ..template.clone()
            })?;
            // Direct store writes reproduce the legacy aliases the assignment API now refuses.
            store.name_runner(&id, Some(runner.clone()))?;
            store.retire(
                &id,
                Retirement {
                    by: "owner".to_owned(),
                    at: 6,
                },
            )?;
        }
    }
    let feed = page(vec![entry(1, "live")], "cursor-1");
    for index in 0..4 {
        let result = keep_page(&table.state, &format!("proof-{index}"), feed.clone()).await;
        assert!(
            matches!(result, Err(ServerError::MachineRetired)),
            "{result:?}"
        );
    }
    keep_page(&table.state, "machine", feed.clone()).await?;
    keep_page(&table.state, "machine", feed).await?;
    with_budgets(&table.state, |store| {
        let held = store.held();
        assert_eq!(held.uses.len(), 1);
        assert_eq!(held.uses[0].event, "native:machine:entry-1");
        assert_eq!(held.uses[0].agent, table.agent);
        assert_eq!(held.uses[0].tokens, 2);
        assert_eq!(held.refusals.cursors.len(), 1);
        assert!(held.refusals.cursors.contains_key("machine"));
        Ok(())
    })?;
    table.finish().await
}

#[tokio::test]
async fn a_foreign_session_is_refused_because_its_runner_did_not_write_this_feed() -> TestResult {
    let table = fixture().await?;
    table.track("good", "machine")?;
    table.track("other", "foreign")?;
    let feed = page(
        vec![entry(1, "good"), entry(2, "other"), entry(3, "good")],
        "cursor-3",
    );
    keep_page(&table.state, "machine", feed.clone()).await?;
    keep_page(&table.state, "machine", feed).await?;
    keep_page(
        &table.state,
        "machine",
        page(vec![entry(4, "good")], "cursor-4"),
    )
    .await?;
    with_budgets(&table.state, |store| {
        let held = store.held();
        assert_eq!(held.uses.len(), 3);
        assert_eq!(held.uses.iter().map(|usage| usage.tokens).sum::<u64>(), 6);
        assert!(
            held.uses
                .iter()
                .all(|usage| usage.session.as_deref() == Some("good"))
        );
        assert_eq!(held.refusals.records.len(), 1);
        let skipped = &held.refusals.records[0];
        assert_eq!(skipped.check, "feed_entry_skipped");
        assert_eq!(skipped.attempt, "feed-2");
        assert_eq!(skipped.agent, table.agent);
        assert_eq!(skipped.session, "other");
        assert!(skipped.words.contains("writer_machine=machine"));
        assert!(skipped.words.contains("session_machine=foreign"));
        assert_eq!(
            held.refusals.cursors.get("machine").map(String::as_str),
            Some("cursor-4")
        );
        Ok(())
    })?;
    // Reading the actual writer's feed attributes the refused entry exactly once there.
    keep_page(
        &table.state,
        "foreign",
        page(vec![entry(2, "other")], "foreign-2"),
    )
    .await?;
    with_budgets(&table.state, |store| {
        assert_eq!(store.held().uses.len(), 4);
        assert_eq!(store.held().uses[3].event, "native:foreign:entry-2");
        Ok(())
    })?;
    table.finish().await
}

#[tokio::test]
async fn malformed_operations_and_commits_are_named_without_discarding_later_usage() -> TestResult {
    let table = fixture().await?;
    table.track("good", "machine")?;
    let outcome = |session: &str, at| {
        Body::Operation(OperationOutcome {
            operation: "operation".to_owned(),
            session: session.to_owned(),
            request: "context_compact".to_owned(),
            state: OperationState::Confirmed,
            at,
            words: "observation".to_owned(),
            text: None,
            ended: None,
        })
    };
    let mut mismatch = entry(2, "good");
    mismatch.body = outcome("other", 5);
    let mut instant = entry(3, "good");
    instant.body = outcome("good", u64::MAX);
    let mut commit = entry(4, "good");
    commit.body = Body::Commit(Commit::default());
    keep_page(
        &table.state,
        "machine",
        page(
            vec![
                entry(1, "good"),
                mismatch,
                instant,
                commit,
                entry(5, "good"),
            ],
            "cursor-5",
        ),
    )
    .await?;
    with_budgets(&table.state, |store| {
        let held = store.held();
        assert_eq!(held.uses.len(), 2);
        assert_eq!(held.refusals.records.len(), 3);
        for (record, (sequence, reason)) in held.refusals.records.iter().zip([
            (2, "different sessions"),
            (3, "instant"),
            (4, "commit"),
        ]) {
            assert_eq!(record.attempt, format!("feed-{sequence}"));
            assert_eq!(record.check, "feed_entry_skipped");
            assert!(record.words.contains(reason), "{}", record.words);
        }
        assert_eq!(
            held.refusals.cursors.get("machine").map(String::as_str),
            Some("cursor-5")
        );
        Ok(())
    })?;
    table.finish().await
}

#[tokio::test]
async fn a_pinned_runner_refuses_another_writers_usage_and_keeps_its_own_next_entry() -> TestResult
{
    let table = fixture().await?;
    table.track("good", "machine")?;
    let writer = "0123456789abcdef0123456789abcdef";
    let socket = crate::refusals_follow::bound_runner(&table.state, "machine")?;
    crate::network_api::with_network(&table.state, |store| {
        store.name_runner(
            "machine",
            Some(crate::runner_client::RunnerRecord::Dialled {
                key: "ab".repeat(32),
                runner: Some(writer.to_owned()),
            }),
        )
    })?;
    // Writer validation sends no runner request.
    let bad = entry(1, "good");
    let runner = crate::refusals_follow::bound_runner(&table.state, "machine")?;
    let reason = super::super::entry_problem(&table.state, "machine", &runner, &bad)?
        .ok_or("another runner's usage was admitted")?;
    assert!(reason.contains("writer"));
    assert!(reason.contains(writer));
    let mut good: FeedEntry = entry(2, "good");
    if let Body::Usage(record) = &mut good.body {
        record.runner = writer.to_owned();
    }
    assert!(super::super::entry_problem(&table.state, "machine", &runner, &good)?.is_none());
    keep_page(&table.state, "machine", page(vec![bad], "cursor-1")).await?;
    // The fixture socket serves this writer and answers the accepted entry's control review.
    crate::network_api::with_network(&table.state, |store| {
        store.name_runner("machine", Some(socket))
    })?;
    keep_page(&table.state, "machine", page(vec![good], "cursor-2")).await?;
    with_budgets(&table.state, |store| {
        assert_eq!(store.held().uses.len(), 1);
        assert_eq!(store.held().uses[0].event, "native:machine:entry-2");
        assert_eq!(store.held().refusals.records.len(), 1);
        assert!(store.held().refusals.records[0].words.contains(writer));
        assert_eq!(
            store
                .held()
                .refusals
                .cursors
                .get("machine")
                .map(String::as_str),
            Some("cursor-2")
        );
        Ok(())
    })?;
    table.finish().await
}

fn wrong_control_runner(
    path: &std::path::Path,
    key: [u8; 32],
    controls: std::sync::Arc<std::sync::atomic::AtomicUsize>,
) -> std::io::Result<tokio::task::JoinHandle<std::io::Result<()>>> {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    let listener = tokio::net::UnixListener::bind(path)?;
    Ok(tokio::spawn(async move {
        let mut peers = tokio::task::JoinSet::new();
        loop {
            tokio::select! {
                accepted = listener.accept() => {
                    let (stream, _) = accepted?;
                    let controls = std::sync::Arc::clone(&controls);
                    peers.spawn(async move {
                        let greeting = lys_runner::protocol::Greeting::fresh("0123456789abcdef0123456789abcdef");
                        let mut stream = BufReader::new(stream);
                        stream.get_mut().write_all(format!("{}\n", greeting.line()).as_bytes()).await?;
                        let mut request = String::new();
                        stream.read_line(&mut request).await?;
                        match lys_runner::protocol::verify_request(&request, &key, &greeting)
                            .map_err(std::io::Error::other)?
                        {
                            lys_runner::Act::Owned => {}
                            lys_runner::Act::ControlStatus { .. } => {
                                controls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                            }
                            other => return Err(std::io::Error::other(format!("unexpected control fixture request {other:?}"))),
                        }
                        let reply = serde_json::to_string(&lys_runner::protocol::Reply {
                            version: lys_runner::protocol::PROTOCOL_VERSION,
                            // Ownership remains readable while control reviews receive the wrong answer.
                            answer: lys_runner::Answer::Owned { owners: Vec::new(), unreachable: Vec::new() },
                        }).map_err(std::io::Error::other)?;
                        stream.get_mut().write_all(format!("{reply}\n").as_bytes()).await
                    });
                }
                finished = peers.join_next(), if !peers.is_empty() => {
                    match finished {
                        Some(Ok(result)) => result?,
                        Some(Err(error)) => return Err(std::io::Error::other(error)),
                        None => return Err(std::io::Error::other("control fixture peer disappeared")),
                    }
                }
            }
        }
    }))
}

#[tokio::test]
async fn a_control_review_refusal_keeps_the_charged_usage_and_continues_through_replay_and_next_page()
-> TestResult {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    let table = fixture().await?;
    table.track("good", "machine")?;
    let socket = table
        .state
        .estate_plan_file
        .parent()
        .ok_or("missing fixture directory")?
        .join("control-refused.sock");
    let controls = Arc::new(AtomicUsize::new(0));
    let server = wrong_control_runner(
        &socket,
        table.state.runners.public_key(),
        Arc::clone(&controls),
    )?;
    let result = async {
        crate::network_api::with_network(&table.state, |store| {
            store.name_runner(
                "machine",
                Some(crate::runner_client::RunnerRecord::Socket {
                    path: socket.to_string_lossy().into_owned(),
                }),
            )
        })?;
        let feed = page(vec![entry(1, "good"), entry(2, "good")], "cursor-2");
        keep_page(&table.state, "machine", feed.clone()).await?;
        with_budgets(&table.state, |store| {
            assert_eq!(store.held().uses.len(), 2);
            assert_eq!(store.held().refusals.records.len(), 2);
            assert_eq!(
                store
                    .held()
                    .refusals
                    .cursors
                    .get("machine")
                    .map(String::as_str),
                Some("cursor-2")
            );
            Ok(())
        })?;
        keep_page(&table.state, "machine", feed).await?;
        keep_page(
            &table.state,
            "machine",
            page(vec![entry(3, "good")], "cursor-3"),
        )
        .await?;
        let sealed = with_budgets(&table.state, |store| {
            store
                .held()
                .encode()
                .map_err(|reason| ServerError::RuntimeUnavailable { reason })
        })?;
        let held = crate::budgets_state::Held::decode(&sealed)?;
        assert_eq!(held.uses.len(), 3);
        assert_eq!(held.uses.iter().map(|usage| usage.tokens).sum::<u64>(), 6);
        assert_eq!(
            held.refusals.records.len(),
            3,
            "replay repeated a control refusal"
        );
        for sequence in 1..=3 {
            assert!(
                held.charged
                    .contains(&format!("native:machine:entry-{sequence}"))
            );
            let refusal = held
                .refusals
                .records
                .iter()
                .find(|record| record.attempt == format!("feed-{sequence}"))
                .ok_or("charged usage has no control refusal")?;
            assert_eq!(refusal.check, "feed_control_refused");
            assert_eq!(refusal.source, "identity-feed:machine");
            assert_eq!(refusal.session, "good");
            assert_eq!(refusal.agent, table.agent);
            assert!(refusal.words.contains(&format!("seq={sequence}")));
            assert!(refusal.words.contains("retained"));
            assert!(refusal.words.contains("control status read"));
            assert!(refusal.words.contains("owned"));
        }
        assert_eq!(
            held.refusals.cursors.get("machine").map(String::as_str),
            Some("cursor-3")
        );
        assert!(controls.load(Ordering::SeqCst) >= 3);
        Ok::<(), Box<dyn std::error::Error>>(())
    }
    .await;
    server.abort();
    let server_result: TestResult = match server.await {
        Ok(result) => result.map_err(Into::into),
        Err(error) if error.is_cancelled() => Ok(()),
        Err(error) => Err(error.into()),
    };
    let fixture_result = table.finish().await;
    result?;
    server_result?;
    fixture_result
}
