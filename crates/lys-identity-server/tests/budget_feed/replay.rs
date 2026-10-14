//! The feed fixture's replay cases.

use super::*;

#[tokio::test]
async fn native_spend_and_unavailable_current_context_survive_replay_and_restart() -> TestResult {
    let (said, mut ended) = tokio::sync::watch::channel(None::<String>);
    let say: lys_identity_server::Say = Arc::new(move |line| {
        if line.starts_with("refusals: the feed of machine") {
            drop(said.send_replace(Some(line.to_owned())));
        }
    });
    let (mut service, (agent, machine, dir, mut runner)) = Service::start_saying(
        GRANT_MODEL,
        None,
        None,
        None,
        |config| config.proxy_dir = Some(config.log_dir.with_file_name("proxy")),
        Some(say),
        prepare,
    )
    .await?;
    runner.finish().await?;
    ended.changed().await?;
    assert!(
        ended
            .borrow()
            .as_ref()
            .is_some_and(|reason| reason.contains("the feed of machine")
                && reason.contains("no longer followed"))
    );
    let cookie = service
        .sign_in(Login {
            subject: ADMINISTRATOR.to_owned(),
            email: "operator@example.test".to_owned(),
        })
        .await?;
    // With no limit set, the agent's usage still answers what it used and
    // its account's windows; nothing is held against a limit.
    let (status, usage) = service
        .get(&format!("/agents/{agent}/usage"), Some(&cookie))
        .await?;
    assert_eq!(status, 200, "{usage}");
    assert_eq!(usage["used"], json!([]));
    assert_eq!(
        usage["figures"]
            .as_array()
            .ok_or("no figures")?
            .iter()
            .map(|row| json!([row["unit"], row["period"], row["figure"]]))
            .collect::<Vec<_>>(),
        vec![
            json!(["context_percent", null, null]),
            json!(["tokens", "day", 100]),
            json!(["tokens", "week", 100]),
            json!(["dollars", "day", 500]),
            json!(["dollars", "week", 500]),
            json!(["running_ms", "day", 150]),
            json!(["running_ms", "week", 150]),
            json!(["plan_percent", "five_hour", null]),
            json!(["tokens", "five_hour", 100]),
        ]
    );
    let accounts = usage["accounts"].as_array().ok_or("no accounts")?;
    assert_eq!(accounts.len(), 1, "{usage}");
    assert_eq!(accounts[0]["account"], "shared-account");
    assert_eq!(accounts[0]["windows"][0]["duration_minutes"], 10_080);
    assert_eq!(accounts[0]["windows"][0]["used_percent"], 49);
    // Each call the proxy reported is listed newest first from the kept uses.
    let (status, calls) = service
        .get(&format!("/agents/{agent}/calls"), Some(&cookie))
        .await?;
    assert_eq!(status, 200, "{calls}");
    assert_eq!(
        calls["calls"]
            .as_array()
            .ok_or("no calls")?
            .iter()
            .map(|row| json!([
                row["call_id"],
                row["model"],
                row["input_tokens"],
                row["account"],
                row["status"],
                row["duration_ms"]
            ]))
            .collect::<Vec<_>>(),
        vec![
            json!([
                "1-Spend",
                "model-one",
                70,
                "shared-account",
                "complete",
                1200
            ]),
            json!([
                "0-Spend",
                "model-one",
                30,
                "shared-account",
                "partial",
                null
            ]),
        ]
    );
    assert_eq!(calls["next"], json!(null));
    let (status, refused) = service
        .get(
            &format!("/agents/{agent}/calls?after=nonsense"),
            Some(&cookie),
        )
        .await?;
    assert_eq!(status, 400, "{refused}");
    // A call kept on this computer is read whole: its record and both bodies.
    let (status, whole) = service
        .get(&format!("/agents/{agent}/calls/1-Spend"), Some(&cookie))
        .await?;
    assert_eq!(status, 200, "{whole}");
    assert_eq!(whole["call"]["call_id"], "1-Spend");
    assert_eq!(whole["request"], json!({"model": "model-one"}));
    assert_eq!(whole["response"], json!({"type": "message"}));
    assert_eq!(whole["request_unreadable"], json!(null));
    assert_eq!(whole["response_unreadable"], json!(null));
    // One this computer's proxy does not hold names the computer that made it.
    let (status, elsewhere) = service
        .get(&format!("/agents/{agent}/calls/0-Spend"), Some(&cookie))
        .await?;
    assert_eq!(status, 409, "{elsewhere}");
    let words = elsewhere.to_string();
    assert!(
        words.contains("CallKeptElsewhere") && words.contains(&machine),
        "{words}"
    );
    let (status, unknown) = service
        .get(
            &format!("/agents/{agent}/calls/no-such-call"),
            Some(&cookie),
        )
        .await?;
    assert_eq!(status, 404, "{unknown}");
    assert!(unknown.to_string().contains("CallUnknown"), "{unknown}");
    let path = format!("/budgets/agent/{agent}");
    let limits = json!([
        {"unit": "tokens", "amount": 200, "period": "day", "act": "tell"},
        {"unit": "running_ms", "amount": 200, "period": "day", "act": "tell"},
        {"unit": "context_percent", "amount": 80, "period": null, "act": "tell"},
        {"unit": "dollars", "amount": 600, "period": "day", "act": "tell"},
        {"unit": "plan_percent", "amount": 60, "period": "week", "act": "tell"}
    ]);
    let (status, answer) = send(
        &service,
        reqwest::Method::PUT,
        &path,
        Auth::Cookie(&cookie),
        Some(&json!({"limits": limits, "warn_at": null, "version": 0})),
    )
    .await?;
    assert_eq!(status, 200, "{answer}");
    assert_eq!(
        answer["used"]
            .as_array()
            .ok_or("no used rows")?
            .iter()
            .map(|row| row["figure"].clone())
            .collect::<Vec<_>>(),
        vec![json!(100), json!(150), json!(35), json!(500), json!(49)]
    );
    let (_, tail) = FrontierLog::open(FileLeafStore::open(&dir)?)?;
    let mut held = Held::default();
    held.fold(&tail)?;
    assert_eq!(held.uses.len(), 4);
    assert_eq!(held.refusals.records.len(), 2);
    let skipped = held
        .refusals
        .records
        .iter()
        .find(|record| record.check == "feed_entry_skipped")
        .ok_or("the invalid page's refusal was not kept")?;
    assert_eq!(skipped.session, "untracked-session");
    assert!(
        skipped.words.contains("RuntimeSessionUnknown"),
        "{}",
        skipped.words
    );
    assert!(
        held.refusals
            .records
            .iter()
            .any(|record| record.words == "denied")
    );
    assert_eq!(
        held.refusals.cursors.get(&machine).map(String::as_str),
        Some("cursor-bad")
    );
    service.restart().await?;
    let (status, restored) = service.get(&path, Some(&cookie)).await?;
    assert_eq!(status, 200, "{restored}");
    assert_eq!(restored["used"], answer["used"]);
    let (_, tail) = FrontierLog::open(FileLeafStore::open(&dir)?)?;
    let mut reopened = Held::default();
    reopened.fold(&tail)?;
    assert_eq!(reopened.refusals.records, held.refusals.records);
    assert_eq!(reopened.refusals.cursors, held.refusals.cursors);
    Ok(())
}
