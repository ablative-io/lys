#![cfg(test)]
//! Agent rows retain their admitted scope before search and subtree filtering.

use std::sync::Arc;

use lys_core::Ed25519Identity;
use lys_identity_server::requests_store::{Asked, RequestStore};

use super::{
    ADMINISTRATOR, OTHER, Service, Table, TestResult, json, login, operation, post, read, rows,
    seed_configured,
};

#[tokio::test]
async fn live_sessions_filter_by_agent_name_and_agent_or_person_membership() -> TestResult {
    let table = Table::start().await?;
    let agents = [
        table.seeded.people[0].agents[0].id.to_string(),
        table.seeded.people[0].agents[1].id.to_string(),
        table.seeded.people[1].agents[0].id.to_string(),
    ];
    let machine = operation()?;
    post(&table.service, &table.cookie, "/network/machines", json!({
        "operation":machine, "name":"session machine", "kind":"server", "runtime":"local launcher",
        "slots":4, "may_run":agents, "may_reach":[],
    })).await?;
    for agent in &agents {
        let session = operation()?;
        let path = format!("/agents/{agent}/runtime/sessions/{session}/reports");
        for state in ["starting", "running"] {
            post(
                &table.service,
                &table.cookie,
                &path,
                json!({
                    "operation":operation()?, "state":state, "machine":machine,
                    "what":"session fixture", "confirmation":"",
                }),
            )
            .await?;
        }
    }
    let root = table.team(None).await?;
    let child = table.team(Some(&root)).await?;
    table.member(&child, &agents[0]).await?;
    let answer = read(
        &table.service,
        &table.cookie,
        &format!("/runtime/live?team={root}"),
    )
    .await?;
    assert_eq!(answer["total"], 1);
    assert_eq!(rows(&answer, "sessions")?.len(), 1);
    assert_eq!(answer["sessions"][0]["agent"], agents[0]);
    table
        .member(&child, &table.seeded.people[1].id.to_string())
        .await?;
    let first = read(
        &table.service,
        &table.cookie,
        &format!("/runtime/live?team={root}&limit=1"),
    )
    .await?;
    assert_eq!(first["total"], 2);
    assert_eq!(rows(&first, "sessions")?.len(), 1);
    let cursor = first["next"].as_str().ok_or("no session cursor")?;
    let second = read(
        &table.service,
        &table.cookie,
        &format!("/runtime/live?team={root}&limit=1&after={cursor}"),
    )
    .await?;
    assert_eq!(second["total"], 2);
    assert_eq!(rows(&second, "sessions")?.len(), 1);
    assert_ne!(
        first["sessions"][0]["session"],
        second["sessions"][0]["session"]
    );
    assert_eq!(second.get("next"), Some(&serde_json::Value::Null));
    let named = read(&table.service, &table.cookie, "/runtime/live?q=sCrIbE").await?;
    assert_eq!(named["total"], 1);
    assert_eq!(named["sessions"][0]["agent"], agents[0]);
    let own = read(
        &table.service,
        &table.other,
        &format!("/runtime/live?team={root}"),
    )
    .await?;
    assert_eq!(own["total"], 1);
    assert_eq!(own["sessions"][0]["agent"], agents[2]);
    Ok(())
}

#[tokio::test]
async fn agent_requests_filter_by_agent_or_responsible_person_without_widening_visibility()
-> TestResult {
    let (service, seeded) = Service::start_with(|config| {
        let seeded = seed_configured(config, [ADMINISTRATOR, OTHER])?;
        let key = Arc::new(Ed25519Identity::load(&config.event_key_file)?);
        let mut store = RequestStore::open(
            config
                .requests_dir
                .as_deref()
                .ok_or("no requests directory")?,
            key,
        )?;
        for (person, agent) in [(0, 0), (0, 1), (1, 0)] {
            store.ask(Asked {
                id: operation()?,
                asked_by: seeded.people[person].agents[agent].id.to_string(),
                responsible: seeded.people[person].id.to_string(),
                resource_kind: "doc".to_owned(),
                resource_id: "quarter".to_owned(),
                relation: "alpha".to_owned(),
                ends_at: None,
                why: "read the quarter".to_owned(),
                asked_at: 1,
            })?;
        }
        Ok(seeded)
    })
    .await?;
    let cookie = service.sign_in(login(ADMINISTRATOR)).await?;
    let other = service.sign_in(login(OTHER)).await?;
    let table = Table {
        service,
        seeded,
        cookie,
        other,
    };
    let root = table.team(None).await?;
    let child = table.team(Some(&root)).await?;
    let agent = table.seeded.people[0].agents[0].id.to_string();
    table.member(&child, &agent).await?;
    let first = read(
        &table.service,
        &table.cookie,
        &format!("/requests?team={root}"),
    )
    .await?;
    assert_eq!(first["total"], 1);
    assert_eq!(rows(&first, "requests")?.len(), 1);
    assert_eq!(first["requests"][0]["asked_by"], agent);
    table
        .member(&child, &table.seeded.people[1].id.to_string())
        .await?;
    let second = read(
        &table.service,
        &table.cookie,
        &format!("/requests?team={root}"),
    )
    .await?;
    assert_eq!(second["total"], 2);
    assert_eq!(rows(&second, "requests")?.len(), 2);
    let own = read(
        &table.service,
        &table.other,
        &format!("/requests?team={root}"),
    )
    .await?;
    assert_eq!(own["total"], 1);
    assert_eq!(
        own["requests"][0]["asked_by"],
        table.seeded.people[1].agents[0].id.to_string()
    );
    Ok(())
}
