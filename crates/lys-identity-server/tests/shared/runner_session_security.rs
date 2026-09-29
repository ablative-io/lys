//! Session routes reveal neither existence nor runner location before admission.

use super::{Table, TestResult, json, operation};

#[tokio::test(flavor = "multi_thread")]
async fn session_routes_hide_unknown_and_foreign_sessions_before_resolving_runners() -> TestResult {
    let mut table = Table::set(&json!({})).await?;
    let session = table.start().await?;
    let unknown = operation()?;
    let mut requests = 0;
    for runner_present in [true, false] {
        if !runner_present {
            table
                .ok(
                    &format!("/network/machines/{}/runner", table.machine),
                    &json!({ "runner": null }),
                )
                .await?;
        }
        for (route, body) in [
            ("input", json!({ "text": "unwanted", "enter": true })),
            ("keys", json!({ "keys": ["ctrl_c"] })),
            ("read", json!({ "follow": false })),
            ("wait", json!({ "pattern": "unwanted" })),
            ("resize", json!({ "columns": 80, "rows": 24 })),
            ("compact", json!({})),
            ("end", json!({})),
            ("input-bytes", json!({ "data": [3] })),
            ("read-bytes", json!({ "follow": false })),
        ] {
            for cookie in [None, Some(table.bea.as_str())] {
                let mut answers = Vec::new();
                for id in [&unknown, &session] {
                    let answer = table
                        .service
                        .post(&format!("/runtime/sessions/{id}/{route}"), cookie, &body)
                        .await?;
                    let expected = if cookie.is_some() {
                        (404, "RuntimeSessionUnknown")
                    } else {
                        (401, "NotSignedIn")
                    };
                    assert_eq!(answer.0, expected.0, "{route}: {}", answer.1);
                    assert_eq!(answer.1["refusal"], expected.1, "{route}: {}", answer.1);
                    let text = answer.1.to_string();
                    assert!(!text.contains(&table.machine), "{text}");
                    assert!(!text.contains(&table.agent()), "{text}");
                    answers.push(answer);
                    requests += 1;
                }
                assert_eq!(answers[0], answers[1], "{route}: existence is hidden");
            }
        }
    }
    assert_eq!(requests, 72);
    table.close()
}
