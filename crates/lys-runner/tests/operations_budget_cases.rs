//! Budget delivery exercises through the identity routes and the real runner.
use super::{Table, TestResult, context, json};
use lys_runner::operations::{OperationState, TextDigest};

#[tokio::test(flavor = "multi_thread")]
async fn empty_compaction_defaults_to_the_tui_command() -> TestResult {
    let mut table = Table::set(&json!({})).await?;
    let session = table.start().await?;
    table.budget(json!({"limits":[{"unit":"context_percent","amount":50,"period":null,"act":"compact"}],"warn_at":null,"version":0})).await?;
    let crossed = table.used(context("default", &session, 60)).await?;
    assert_eq!(
        crossed["receipts"][0]["crossing"]["text"], "/compact",
        "{crossed}"
    );
    assert_ne!(
        crossed["receipts"][0]["acted"]["stands"], "refused",
        "{crossed}"
    );
    table.waited(&session, "/compact").await?;
    table.end(&session).await?;
    table.stop_runner()
}

#[tokio::test(flavor = "multi_thread")]
async fn budget_and_route_compact_read_the_start_version() -> TestResult {
    let mut table = Table::set(&json!({"compact":"echo start-version"})).await?;
    let session = table.start().await?;
    table
        .profile_at(&json!({"compact":"echo newer-version"}), 1)
        .await?;
    table
        .ok(
            &format!("/agents/{}/words/compaction", table.agent()),
            &json!({"setting":{"kind":"text","text":"echo newer-words"},"revision":0}),
        )
        .await?;
    table.budget(json!({"limits":[{"unit":"context_percent","amount":50,"period":null,"act":"compact"}],"warn_at":null,"version":0})).await?;
    let crossed = table.used(context("old-profile", &session, 60)).await?;
    assert_eq!(
        crossed["receipts"][0]["crossing"]["text"], "echo start-version",
        "{crossed}"
    );
    let compact_id = crossed["receipts"][0]["crossing"]["operation"]
        .as_str()
        .ok_or("no compact operation")?;
    assert_eq!(
        table
            .serving
            .as_ref()
            .ok_or("no runner")?
            .sessions()
            .outcome(compact_id)?
            .text,
        Some(TextDigest::of("echo start-version"))
    );
    table
        .serving
        .as_ref()
        .ok_or("no runner")?
        .sessions()
        .collect(
            &session,
            &lys_runner::session::Collected::Hook {
                event: "Stop".to_owned(),
                input: json!({}),
            },
        )?;
    let compact = table
        .ok(&format!("/runtime/sessions/{session}/compact"), &json!({}))
        .await?;
    let digest = serde_json::to_value(TextDigest::of("echo start-version"))?;
    assert_eq!(compact["receipt"]["act"]["text"], digest, "{compact}");
    assert_eq!(compact["profile_version"], 1, "{compact}");
    table.waited(&session, "start-version").await?;
    table.end(&session).await?;
    table.stop_runner()
}

#[tokio::test(flavor = "multi_thread")]
async fn preparation_is_the_last_of_multiple_warnings_below_compaction() -> TestResult {
    let mut table = Table::set(&json!({"compact":"echo compacted"})).await?;
    let session = table.start().await?;
    table.budget(json!({"limits":[
        {"unit":"context_percent","amount":25,"period":null,"act":"notice"},
        {"unit":"context_percent","amount":45,"period":null,"act":"notice"},
        {"unit":"context_percent","amount":50,"period":null,"act":"compact"}],"warn_at":null,"version":0})).await?;
    table.used(context("warning", &session, 26)).await?;
    table
        .waited(
            &session,
            "You're at 26% of your context window; compaction is at 50%.",
        )
        .await?;
    table
        .serving
        .as_ref()
        .ok_or("no runner")?
        .sessions()
        .collect(
            &session,
            &lys_runner::session::Collected::Hook {
                event: "Stop".to_owned(),
                input: json!({}),
            },
        )?;
    table.used(context("prepare", &session, 46)).await?;
    let sessions = table.serving.as_ref().ok_or("no runner")?.sessions();
    let receipts = table.receipts().await?;
    let id = receipts[1]["crossing"]["operation"]
        .as_str()
        .ok_or("no operation")?;
    assert_eq!(
        sessions.outcome(id)?.text,
        Some(TextDigest::of(
            "[Lys context watch] You're at 46% of your context window; this session compacts at 50%, at the first turn boundary past that level. Wrap up what you're holding and note anything you need after compaction."
        ))
    );
    table
        .waited(
            &session,
            "this session compacts at 50%, at the first turn boundary past that level",
        )
        .await?;
    table.end(&session).await?;
    table.stop_runner()
}

#[tokio::test(flavor = "multi_thread")]
async fn a_confirmed_compaction_queues_exactly_one_rendered_wake_up() -> TestResult {
    let mut table = Table::set(&json!({"compact":"echo compacted"})).await?;
    let session = table.start().await?;
    table
        .ok(
            &format!("/agents/{}/words/wake_up", table.agent()),
            &json!({"setting":{"kind":"text","text":"Resume after {{operation}}"},"revision":0}),
        )
        .await?;
    table.budget(json!({"limits":[{"unit":"context_percent","amount":50,"period":null,"act":"compact"}],"warn_at":null,"version":0})).await?;
    let crossed = table.used(context("compact", &session, 60)).await?;
    let id = crossed["receipts"][0]["crossing"]["operation"]
        .as_str()
        .ok_or("no operation")?;
    table.waited(&session, "compacted").await?;
    let sessions = table.serving.as_ref().ok_or("no runner")?.sessions();
    for _ in 0..2 {
        sessions.collect(
            &session,
            &lys_runner::session::Collected::Hook {
                event: "PreCompact".to_owned(),
                input: json!({}),
            },
        )?;
        sessions.collect(
            &session,
            &lys_runner::session::Collected::Hook {
                event: "Stop".to_owned(),
                input: json!({}),
            },
        )?;
    }
    assert_eq!(sessions.outcome(id)?.state, OperationState::Confirmed);
    let wake = sessions.outcome(&format!("wake-up-{id}"))?;
    assert_eq!(
        wake.text,
        Some(TextDigest::of(&format!("Resume after {id}")))
    );
    assert!(wake.words.contains(id), "{wake:?}");
    table
        .waited(&session, &format!("Resume after {id}"))
        .await?;
    let shown = table.end(&session).await?;
    assert_eq!(
        shown.matches(&format!("Resume after {id}")).count(),
        1,
        "{shown}"
    );
    table.stop_runner()
}

#[tokio::test(flavor = "multi_thread")]
async fn one_warning_remains_context_warning_and_busy_delivery_waits() -> TestResult {
    let mut table = Table::set(&json!({"compact":"echo compacted"})).await?;
    let session = table.start().await?;
    table.budget(json!({"limits":[
        {"unit":"context_percent","amount":45,"period":null,"act":"notice"},
        {"unit":"context_percent","amount":50,"period":null,"act":"compact"}],"warn_at":null,"version":0})).await?;
    let sessions = table.serving.as_ref().ok_or("no runner")?.sessions();
    sessions.collect(
        &session,
        &lys_runner::session::Collected::Hook {
            event: "UserPromptSubmit".to_owned(),
            input: json!({}),
        },
    )?;
    let crossed = table.used(context("warning", &session, 46)).await?;
    assert_eq!(
        crossed["receipts"][0]["acted"]["stands"], "accepted",
        "{crossed}"
    );
    let operation = crossed["receipts"][0]["crossing"]["operation"]
        .as_str()
        .ok_or("no operation")?;
    assert_eq!(sessions.outcome(operation)?.state, OperationState::Accepted);
    let client = lys_runner::Client::new(
        table.socket.clone(),
        std::sync::Arc::new(lys_core::Ed25519Identity::load(
            table.service.event_key_file(),
        )?),
    );
    let output = client.ask(&lys_runner::Act::Read {
        session: session.clone(),
        cursor: Some(0),
        lines: None,
        bytes: None,
        follow: false,
    })?;
    let lys_runner::Answer::Output { output } = output else {
        return Err("no output".into());
    };
    assert!(!output.text.contains("You're at 46% of your context window"));
    sessions.collect(
        &session,
        &lys_runner::session::Collected::Hook {
            event: "Stop".to_owned(),
            input: json!({}),
        },
    )?;
    table
        .waited(&session, "You're at 46% of your context window")
        .await?;
    let mut cursor = None;
    loop {
        let lys_runner::Answer::Feed { page } = client.ask(&lys_runner::Act::Feed {
            cursor,
            follow: true,
        })?
        else {
            return Err("delivery wait was not answered with a feed page".into());
        };
        let settled = page.entries.iter().any(|entry| {
            matches!(
                &entry.body,
                lys_runner::tracking_store::Body::Operation(outcome)
                    if outcome.operation == operation
                        && !matches!(outcome.state, OperationState::Accepted | OperationState::Delivering)
            )
        });
        if settled {
            break;
        }
        cursor = Some(page.cursor);
    }
    assert_eq!(
        sessions.outcome(operation)?.state,
        OperationState::Delivered
    );
    let shown = table.end(&session).await?;
    assert_eq!(
        shown
            .matches("You're at 46% of your context window")
            .count(),
        1,
        "{shown}"
    );
    table.stop_runner()
}
