//! Runner wake, stop and raw terminal controls preserve standing and receipts.

use super::*;

#[tokio::test(flavor = "multi_thread")]
async fn a_wake_types_the_message_into_the_live_session_and_needs_one() -> TestResult {
    let mut table = Table::set(&json!({})).await?;
    let path = format!("/agents/{}/wake", table.agent());
    let ada = table.ada.clone();
    let (status, answer) = table
        .sent(&path, &ada, &json!({ "message": "echo early" }))
        .await?;
    assert_eq!(
        (status, answer["refusal"].as_str()),
        (409, Some("no_live_session")),
        "{answer}"
    );
    let session = table.start().await?;
    let woken = table
        .ok(&path, &json!({ "message": "echo woken-$((1+1))" }))
        .await?;
    assert_eq!(woken["session"], session.as_str());
    assert_eq!(woken["receipt"]["act"]["act"], "wake");
    table.waited(&session, "woken-2").await?;
    table.close()
}

#[tokio::test(flavor = "multi_thread")]
async fn a_stop_ends_every_session_and_each_shows_confirmed_with_its_exit_instant() -> TestResult {
    let mut table = Table::set(&json!({})).await?;
    let first = table.start().await?;
    let second = table.start().await?;
    let path = format!("/agents/{}/stop", table.agent());
    let body = json!({ "operation": operation()?, "reason": "the drill" });
    let stopped = table.ok(&path, &body).await?;
    let confirmed = stopped["sessions_confirmed"]
        .as_array()
        .ok_or("no confirmations")?;
    let mut seen = Vec::new();
    for end in confirmed {
        let words = end["confirmation"].as_str().ok_or("no words")?;
        assert!(words.contains("ms since the Unix epoch"), "{words}");
        seen.push(end["session"].as_str().ok_or("no session")?.to_owned());
    }
    seen.sort();
    let mut expected = vec![first, second];
    expected.sort();
    assert_eq!(seen, expected, "{stopped}");
    assert_eq!(stopped["sessions_refused"], json!([]));
    let (_, live) = table.service.get("/runtime/live", Some(&table.ada)).await?;
    assert_eq!(live["sessions"], json!([]), "{live}");
    table.close()
}

#[tokio::test(flavor = "multi_thread")]
async fn raw_terminal_routes_keep_grants_receipts_and_original_bytes() -> TestResult {
    let mut table = Table::set(&json!({})).await?;
    let session = table.start().await?;
    let input = b"printf 'byte-proof-\\377\\342'\r";
    let bea = table.bea.clone();
    for (route, body) in [
        ("input-bytes", json!({ "data": input })),
        ("read-bytes", json!({ "cursor": 0, "follow": false })),
    ] {
        let (status, answer) = table
            .sent(&format!("/runtime/sessions/{session}/{route}"), &bea, &body)
            .await?;
        assert_eq!(status, 404, "{answer}");
        assert_eq!(answer["refusal"], "RuntimeSessionUnknown");
    }
    let sent = table
        .act(&session, "input-bytes", &json!({ "data": input }))
        .await?;
    assert_eq!(sent["answer"]["kind"], "delivered");
    assert_eq!(sent["receipt"]["act"]["act"], "input_bytes");
    assert_eq!(sent["receipt"]["act"]["text"]["length"], input.len());
    assert!(!sent["receipt"].to_string().contains("byte-proof"));
    table.waited(&session, "byte-proof-").await?;
    let mut cursor = 0;
    let mut bytes = Vec::new();
    while !bytes.windows(2).any(|window| window == [0xff, 0xe2]) {
        let answer = table
            .act(
                &session,
                "read-bytes",
                &json!({ "cursor": cursor, "follow": true }),
            )
            .await?;
        assert_eq!(answer["answer"]["kind"], "bytes");
        assert_eq!(answer["receipt"]["act"]["act"], "read_bytes");
        let output = &answer["answer"]["output"];
        assert_eq!(output["from"], cursor);
        let data: Vec<u8> = serde_json::from_value(output["data"].clone())?;
        cursor = output["cursor"].as_u64().ok_or("missing byte cursor")?;
        bytes.extend(data);
    }
    table.close()
}
