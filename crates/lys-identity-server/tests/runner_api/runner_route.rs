//! Runner reachability cases use the same broker and admission fixture.

use super::*;

/// A machine named with no runner, then given `runner` when there is one.
async fn machine_with(table: &mut Table, runner: Option<Value>) -> Result<String, Box<dyn Error>> {
    let id = operation()?;
    let body = json!({
        "operation": id, "name": format!("Box {id}"), "kind": "laptop", "runtime": "sh",
        "slots": 1, "may_run": [], "may_reach": [],
    });
    table.ok("/network/machines", &body).await?;
    if let Some(runner) = runner {
        table
            .ok(
                &format!("/network/machines/{id}/runner"),
                &json!({ "runner": runner }),
            )
            .await?;
    }
    Ok(id)
}

/// The runner route says whether a machine's runner answers now, by asking
/// it: `true` for the runner started here, `false` with the refusal for one
/// that cannot be reached or is not dialled in, and `null` with no runner.
#[tokio::test(flavor = "multi_thread")]
async fn the_runner_route_says_whether_the_runner_answers_now() -> TestResult {
    let mut table = Table::set(&json!({})).await?;
    let ada = table.ada.clone();
    let runner = |id: &str| format!("/network/machines/{id}/runner");

    let (status, own) = table
        .service
        .get(&runner(&table.machine), Some(&ada))
        .await?;
    assert_eq!(status, 200, "{own}");
    assert_eq!(own["answers"], true, "{own}");
    assert_eq!(own["runner"]["kind"], "lys", "{own}");
    assert!(own.get("refusal").is_none(), "{own}");

    let none = machine_with(&mut table, None).await?;
    let (status, unnamed) = table.service.get(&runner(&none), Some(&ada)).await?;
    assert_eq!(status, 200, "{unnamed}");
    assert_eq!(unnamed["answers"], Value::Null, "{unnamed}");
    assert_eq!(unnamed["runner"], Value::Null, "{unnamed}");

    let absent = table.dir.path().join("no-runner-here.sock");
    let socket = json!({ "kind": "socket", "path": absent.display().to_string() });
    let gone = machine_with(&mut table, Some(socket)).await?;
    let (status, silent) = table.service.get(&runner(&gone), Some(&ada)).await?;
    assert_eq!(status, 200, "{silent}");
    assert_eq!(silent["answers"], false, "{silent}");
    assert_eq!(silent["refusal"], "runner_unreachable", "{silent}");
    let reason = silent["reason"].as_str().ok_or("no reason")?;
    assert!(reason.contains("no-runner-here.sock"), "{reason}");

    let dialled = json!({ "kind": "dialled", "key": "ab".repeat(32) });
    let away = machine_with(&mut table, Some(dialled)).await?;
    let (status, undialled) = table.service.get(&runner(&away), Some(&ada)).await?;
    assert_eq!(status, 200, "{undialled}");
    assert_eq!(undialled["answers"], false, "{undialled}");
    assert_eq!(undialled["refusal"], "runner_not_dialled_in", "{undialled}");
    assert!(
        undialled["reason"]
            .as_str()
            .is_some_and(|words| !words.is_empty()),
        "{undialled}"
    );
    table.close()
}
