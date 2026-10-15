#![cfg(test)]
use super::super::Table;
use crate::seats_acts::{START, admitted};
use crate::seats_state::{Added, Seat};
use axum::http::{HeaderMap, header};
use std::error::Error;
type Result = std::result::Result<(), Box<dyn Error>>;

fn seat(agent: &str) -> Seat {
    Seat {
        qualification: None,
        transport: None,
        added: Added {
            operation: "fixture-seat".to_owned(),
            name: "fixture-seat".to_owned(),
            agent: agent.to_owned(),
            harness: "fixture".to_owned(),
            profile_version: 1,
            machine: "fixture-machine".to_owned(),
            working_folder: None,
            account: None,
            by: "fixture".to_owned(),
            at: 1,
        },
        session: None,
        harness_session: None,
        running: false,
        revision: 1,
        retired: false,
    }
}
fn headers(pass: &str) -> std::result::Result<HeaderMap, Box<dyn Error>> {
    let mut headers = HeaderMap::new();
    headers.insert(header::AUTHORIZATION, format!("Bearer {pass}").parse()?);
    Ok(headers)
}

#[tokio::test]
async fn personapp_start_admits_the_pass_persons_own_seat() -> Result {
    let table = Table::app_fixture().await?;
    let pass = table.pass("fixture_product", false, true)?;
    let headers = headers(&pass)?;
    let (admission, standing) = admitted(
        &table.state,
        &headers,
        &seat(&table.own),
        START,
        "/seats/fixture-seat/start",
    )?;
    assert_eq!(admission.caller, table.person.to_string());
    assert_eq!(admission.by, "responsible");
    assert!(standing.is_responsible);
    assert!(
        !standing.administrator,
        "the app pass must not acquire administrator authority"
    );
    assert!(table.state.secrets.is_none());
    let mut checked = 0;
    for kind in [
        crate::start_checks::Inventory::Handles,
        crate::start_checks::Inventory::ProviderRoutes,
    ] {
        let error = crate::start_checks::inventory(&table.state, &headers, &table.own, kind)
            .await
            .err()
            .ok_or("the fixture unexpectedly configured a broker")?;
        assert!(
            matches!(error, crate::error::ServerError::SecretsUnavailable { .. }),
            "{error}"
        );
        checked += 1;
    }
    assert_eq!(checked, 2);
    Ok(())
}

#[tokio::test]
async fn personapp_start_refuses_another_persons_seat() -> Result {
    let mut table = Table::app_fixture().await?;
    table.administrator()?;
    let pass = table.pass("fixture_product", false, true)?;
    let headers = headers(&pass)?;
    let result = admitted(
        &table.state,
        &headers,
        &seat(&table.other),
        START,
        "/seats/fixture-seat/start",
    );
    let error = result
        .err()
        .ok_or("a person-app pass started another person's seat")?;
    assert_eq!(error.name(), "not_permitted", "{error}");
    let mut checked = 0;
    for kind in [
        crate::start_checks::Inventory::Handles,
        crate::start_checks::Inventory::ProviderRoutes,
    ] {
        let error = crate::start_checks::inventory(&table.state, &headers, &table.other, kind)
            .await
            .err()
            .ok_or("an app pass read another person's launch inventory")?;
        assert_eq!(error.name(), "not_permitted", "{error}");
        checked += 1;
    }
    assert_eq!(checked, 2);
    Ok(())
}
