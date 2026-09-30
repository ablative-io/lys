#![cfg(test)]
//! A cursor retains its filter scope and GET queries are described as parameters.

use super::{Table, TestResult, read, rows};

#[tokio::test]
async fn cursors_cannot_be_reused_for_another_route_or_search() -> TestResult {
    let table = Table::start().await?;
    let first = read(&table.service, &table.cookie, "/directory/people?limit=1").await?;
    let cursor = first["next"].as_str().ok_or("no cursor")?;
    for path in [
        format!("/directory/people?q=scribe&after={cursor}"),
        format!("/people?after={cursor}"),
        format!("/network?after={cursor}"),
        format!("/requests?after={cursor}"),
        format!("/runtime/live?after={cursor}"),
    ] {
        let (status, answer) = table.service.get(&path, Some(&table.cookie)).await?;
        assert_eq!(status, 400, "{path}: {answer}");
        assert_eq!(answer["refusal"], "RequestMalformed");
    }
    Ok(())
}

#[test]
fn the_openapi_document_declares_optional_query_parameters_without_a_request_body() -> TestResult {
    let document = lys_identity_server::openapi::document()?;
    for (path, _) in super::ROUTES {
        let operation = &document["paths"][path]["get"];
        assert!(operation.get("requestBody").is_none(), "{path}");
        let parameters = rows(operation, "parameters")?;
        assert_eq!(parameters.len(), 4, "{path}: {operation}");
        for name in ["q", "team", "after", "limit"] {
            let parameter = parameters
                .iter()
                .find(|parameter| parameter["name"] == name)
                .ok_or("missing query parameter")?;
            assert_eq!(parameter["in"], "query");
            assert_eq!(parameter["required"], false);
        }
    }
    Ok(())
}

#[tokio::test]
async fn unknown_teams_are_named_and_never_turn_into_an_unfiltered_list() -> TestResult {
    let table = Table::start().await?;
    let unknown = super::operation()?;
    for path in ["/people", "/directory/people", "/requests", "/runtime/live"] {
        let (status, answer) = table
            .service
            .get(&format!("{path}?team={unknown}"), Some(&table.cookie))
            .await?;
        assert_eq!(status, 404, "{path}: {answer}");
        assert_eq!(answer["refusal"], "TeamUnknown");
    }
    Ok(())
}

#[tokio::test]
async fn network_team_search_uses_machine_ownership_and_includes_descendants() -> TestResult {
    let table = Table::start().await?;
    let root = table.team(None).await?;
    let child = table.team(Some(&root)).await?;
    let outside = table.team(None).await?;
    let root_machine = table.machine("Build root").await?;
    let child_machine = table.machine("Build child").await?;
    let other_machine = table.machine("Build outside").await?;
    table.machine("Build unowned").await?;
    for (machine, team) in [
        (&root_machine, &root),
        (&child_machine, &child),
        (&other_machine, &outside),
    ] {
        let answer = super::post(
            &table.service,
            &table.cookie,
            &format!("/network/machines/{machine}/team"),
            super::json!({
                "operation":super::operation()?, "team":team,
            }),
        )
        .await?;
        assert_eq!(answer["machine"]["team"], team.as_str());
    }
    let first = read(
        &table.service,
        &table.cookie,
        &format!("/network?team={root}&q=build&limit=1"),
    )
    .await?;
    assert_eq!(first["total"], 2, "{first}");
    assert_eq!(rows(&first, "machines")?.len(), 1);
    let cursor = first["next"].as_str().ok_or("no owned machine cursor")?;
    let second = read(
        &table.service,
        &table.cookie,
        &format!("/network?team={root}&q=build&limit=1&after={cursor}"),
    )
    .await?;
    assert_eq!(second["total"], 2);
    assert_eq!(rows(&second, "machines")?.len(), 1);
    assert_eq!(second.get("next"), Some(&serde_json::Value::Null));
    let ids: std::collections::BTreeSet<&str> = [&first, &second]
        .iter()
        .map(|answer| {
            answer["machines"][0]["id"]
                .as_str()
                .ok_or("owned machine has no id")
        })
        .collect::<Result<_, _>>()?;
    assert_eq!(
        ids,
        std::collections::BTreeSet::from([root_machine.as_str(), child_machine.as_str()])
    );
    Ok(())
}
