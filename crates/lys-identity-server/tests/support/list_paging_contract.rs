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
