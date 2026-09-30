#![cfg(test)]
//! Queries narrow admitted lists and page complete rows without changing legacy answers.

use std::collections::BTreeSet;
use std::error::Error;

use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_identity::OperationId;
use lys_identity_server::dev_seed::{Seeded, seed_configured};
use serde_json::{Value, json};

#[path = "support/list_paging_scale.rs"]
mod scale;

#[path = "support/list_paging_rows.rs"]
mod agent_rows;

#[path = "support/list_paging_contract.rs"]
mod contract;

type TestResult = Result<(), Box<dyn Error>>;

const OTHER: &str = "other-subject";
const ROUTES: [(&str, &str); 5] = [
    ("/people", "people"),
    ("/directory/people", "people"),
    ("/network", "machines"),
    ("/runtime/live", "sessions"),
    ("/requests", "requests"),
];

fn login(subject: &str) -> Login {
    Login {
        subject: subject.to_owned(),
        email: "operator@example.test".to_owned(),
    }
}

fn operation() -> Result<String, Box<dyn Error>> {
    Ok(OperationId::generate()?.to_string())
}

async fn read(service: &Service, cookie: &str, path: &str) -> Result<Value, Box<dyn Error>> {
    let (status, answer) = service.get(path, Some(cookie)).await?;
    assert_eq!(status, 200, "{path}: {answer}");
    Ok(answer)
}

async fn post(
    service: &Service,
    cookie: &str,
    path: &str,
    body: Value,
) -> Result<Value, Box<dyn Error>> {
    let (status, answer) = service.post(path, Some(cookie), &body).await?;
    assert_eq!(status, 200, "{path}: {answer}");
    Ok(answer)
}

fn rows<'a>(answer: &'a Value, key: &str) -> Result<&'a Vec<Value>, Box<dyn Error>> {
    answer[key]
        .as_array()
        .ok_or_else(|| format!("no {key} rows: {answer}").into())
}

struct Table {
    service: Service,
    seeded: Seeded,
    cookie: String,
    other: String,
}

impl Table {
    async fn start() -> Result<Self, Box<dyn Error>> {
        let (service, seeded) =
            Service::start_with(|config| Ok(seed_configured(config, [ADMINISTRATOR, OTHER])?))
                .await?;
        let cookie = service.sign_in(login(ADMINISTRATOR)).await?;
        let other = service.sign_in(login(OTHER)).await?;
        Ok(Self {
            service,
            seeded,
            cookie,
            other,
        })
    }

    async fn team(&self, parent: Option<&str>) -> Result<String, Box<dyn Error>> {
        let id = operation()?;
        post(
            &self.service,
            &self.cookie,
            "/teams",
            json!({
                "operation":id, "name":"team", "parent":parent,
            }),
        )
        .await?;
        Ok(id)
    }

    async fn member(&self, team: &str, member: &str) -> TestResult {
        post(
            &self.service,
            &self.cookie,
            &format!("/teams/{team}/members"),
            json!({
                "operation":operation()?, "member":member,
            }),
        )
        .await?;
        Ok(())
    }

    async fn machine(&self, name: &str) -> Result<String, Box<dyn Error>> {
        let id = operation()?;
        post(
            &self.service,
            &self.cookie,
            "/network/machines",
            json!({
                "operation":id, "name":name, "kind":"server", "runtime":"local launcher",
                "slots":4, "may_run":[], "may_reach":[],
            }),
        )
        .await?;
        Ok(id)
    }

    async fn request(&self, resource: &str, why: &str) -> TestResult {
        post(
            &self.service,
            &self.other,
            "/requests",
            json!({
                "operation":operation()?, "resource":{"kind":"doc", "id":resource},
                "relation":"alpha", "ends_at":null, "why":why,
            }),
        )
        .await?;
        Ok(())
    }
}

#[tokio::test]
async fn queries_leave_all_five_legacy_shapes_unchanged() -> TestResult {
    let table = Table::start().await?;
    for (path, key) in ROUTES {
        let before = read(&table.service, &table.cookie, path).await?;
        assert!(rows(&before, key).is_ok());
        for metadata in ["total", "next", "agents_total"] {
            assert!(before.get(metadata).is_none(), "{path}: {before}");
        }
        let queried = read(
            &table.service,
            &table.cookie,
            &format!("{path}?q=absent-name"),
        )
        .await?;
        assert_eq!(rows(&queried, key)?.len(), 0, "{path}: {queried}");
        assert_eq!(queried["total"], 0, "{path}: {queried}");
        assert_eq!(queried.get("next"), Some(&Value::Null), "{path}: {queried}");
        assert_eq!(read(&table.service, &table.cookie, path).await?, before);
    }
    Ok(())
}

#[tokio::test]
async fn an_agent_name_match_returns_the_whole_person_and_counts_all_their_agents() -> TestResult {
    let table = Table::start().await?;
    for path in ["/people", "/directory/people"] {
        let answer = read(
            &table.service,
            &table.cookie,
            &format!("{path}?q=sCrIbE&limit=1"),
        )
        .await?;
        assert_eq!(answer["total"], 1, "{answer}");
        assert_eq!(answer["agents_total"], 3, "{answer}");
        assert_eq!(answer["next"], Value::Null);
        let people = rows(&answer, "people")?;
        assert_eq!(people.len(), 1);
        assert_eq!(people[0]["id"], table.seeded.people[0].id.to_string());
        assert_eq!(rows(&people[0], "agents")?.len(), 3);
    }
    let answer = read(&table.service, &table.cookie, "/directory/people?q=bEa").await?;
    assert_eq!(answer["total"], 1);
    assert_eq!(answer["agents_total"], 2);
    assert_eq!(
        answer["people"][0]["id"],
        table.seeded.people[1].id.to_string()
    );
    Ok(())
}

#[tokio::test]
async fn a_cursor_pages_people_once_and_never_splits_their_agents() -> TestResult {
    let table = Table::start().await?;
    let first = read(&table.service, &table.cookie, "/directory/people?limit=1").await?;
    assert_eq!(first["total"], 2);
    assert_eq!(first["agents_total"], 5);
    assert_eq!(rows(&first, "people")?.len(), 1);
    let cursor = first["next"].as_str().ok_or("first page has no cursor")?;
    assert!(!cursor.is_empty());
    assert_ne!(first["people"][0]["id"], cursor, "cursor is opaque");
    let second = read(
        &table.service,
        &table.cookie,
        &format!("/directory/people?limit=1&after={cursor}"),
    )
    .await?;
    assert_eq!(second["total"], 2);
    assert_eq!(second["agents_total"], 5);
    assert_eq!(second.get("next"), Some(&Value::Null));
    assert_eq!(rows(&second, "people")?.len(), 1);
    assert_ne!(first["people"][0]["id"], second["people"][0]["id"]);
    for answer in [&first, &second] {
        let person = &answer["people"][0];
        let expected = table
            .seeded
            .people
            .iter()
            .find(|seed| person["id"] == seed.id.to_string())
            .ok_or("unexpected person")?;
        assert_eq!(rows(person, "agents")?.len(), expected.agents.len());
    }
    Ok(())
}

#[tokio::test]
async fn team_filters_follow_descendants_and_accept_person_or_agent_membership() -> TestResult {
    let table = Table::start().await?;
    let root = table.team(None).await?;
    let child = table.team(Some(&root)).await?;
    let outside = table.team(None).await?;
    table
        .member(&child, &table.seeded.people[0].agents[0].id.to_string())
        .await?;
    table
        .member(&outside, &table.seeded.people[1].id.to_string())
        .await?;
    let answer = read(
        &table.service,
        &table.cookie,
        &format!("/directory/people?team={root}"),
    )
    .await?;
    assert_eq!(answer["total"], 1);
    assert_eq!(answer["agents_total"], 3);
    assert_eq!(
        answer["people"][0]["id"],
        table.seeded.people[0].id.to_string()
    );
    let answer = read(
        &table.service,
        &table.cookie,
        &format!("/directory/people?team={outside}"),
    )
    .await?;
    assert_eq!(answer["total"], 1);
    assert_eq!(answer["agents_total"], 2);
    let own = read(
        &table.service,
        &table.cookie,
        &format!("/people?team={outside}"),
    )
    .await?;
    assert_eq!(own["total"], 0);
    assert_eq!(own["people"], json!([]));
    Ok(())
}

#[tokio::test]
async fn network_search_pages_matching_machines_before_counting_the_page() -> TestResult {
    let table = Table::start().await?;
    for name in ["Build alpha", "Elsewhere", "Build beta"] {
        table.machine(name).await?;
    }
    let first = read(&table.service, &table.cookie, "/network?q=BUILD&limit=1").await?;
    assert_eq!(first["total"], 2);
    assert_eq!(rows(&first, "machines")?.len(), 1);
    let cursor = first["next"].as_str().ok_or("no machine cursor")?;
    let second = read(
        &table.service,
        &table.cookie,
        &format!("/network?q=BUILD&limit=1&after={cursor}"),
    )
    .await?;
    assert_eq!(second["total"], 2);
    assert_eq!(rows(&second, "machines")?.len(), 1);
    assert_ne!(first["machines"][0]["id"], second["machines"][0]["id"]);
    assert_eq!(second.get("next"), Some(&Value::Null));
    Ok(())
}

#[tokio::test]
async fn request_search_matches_each_screen_field_and_pages_visible_requests() -> TestResult {
    let table = Table::start().await?;
    table.request("quarter-needle", "audit").await?;
    table.request("elsewhere", "needle explanation").await?;
    table.request("elsewhere-again", "audit").await?;
    for (query, count) in [("needle", 2), ("bea", 3), ("alpha", 3)] {
        let answer = read(
            &table.service,
            &table.cookie,
            &format!("/requests?q={query}&limit=1"),
        )
        .await?;
        assert_eq!(answer["total"], count, "{answer}");
        assert_eq!(rows(&answer, "requests")?.len(), 1);
    }
    let first = read(&table.service, &table.cookie, "/requests?q=needle&limit=1").await?;
    let cursor = first["next"].as_str().ok_or("no request cursor")?;
    let second = read(
        &table.service,
        &table.cookie,
        &format!("/requests?q=needle&limit=1&after={cursor}"),
    )
    .await?;
    assert_eq!(second["total"], 2);
    assert_eq!(rows(&second, "requests")?.len(), 1);
    assert_ne!(first["requests"][0]["id"], second["requests"][0]["id"]);
    assert_eq!(second.get("next"), Some(&Value::Null));
    Ok(())
}

#[tokio::test]
async fn queries_never_broaden_authentication_or_the_personal_scope() -> TestResult {
    let table = Table::start().await?;
    for (route, _) in ROUTES {
        let (status, answer) = table.service.get(&format!("{route}?limit=1"), None).await?;
        assert_eq!(status, 401, "{route}: {answer}");
        assert_eq!(answer["refusal"], "NotSignedIn");
    }
    let (status, answer) = table
        .service
        .get("/directory/people?q=scribe&limit=1", Some(&table.other))
        .await?;
    assert_eq!(status, 403, "{answer}");
    assert_eq!(answer["refusal"], "NotAdmitted");
    let own = read(&table.service, &table.other, "/people?q=scribe").await?;
    assert_eq!(own["total"], 0);
    assert_eq!(own["agents_total"], 0);
    assert_eq!(own["people"], json!([]));
    Ok(())
}

#[tokio::test]
async fn malformed_limits_and_cursors_are_named_boundary_refusals() -> TestResult {
    let table = Table::start().await?;
    for (route, _) in ROUTES {
        for query in ["limit=0", "limit=-1", "limit=word", "after=not-a-cursor"] {
            let (status, answer) = table
                .service
                .get(&format!("{route}?{query}"), Some(&table.cookie))
                .await?;
            assert_eq!(status, 400, "{route}?{query}: {answer}");
            assert_eq!(answer["refusal"], "RequestMalformed", "{answer}");
        }
    }
    Ok(())
}

#[tokio::test]
async fn twelve_hundred_agents_across_forty_teams_keep_complete_people_and_exact_totals()
-> TestResult {
    let (service, fixture) = Service::start_with(scale::seed).await?;
    assert_eq!(fixture.people.len(), 240);
    assert_eq!(fixture.teams.len(), 40);
    assert_eq!(
        fixture
            .people
            .iter()
            .map(|person| person.agents.len())
            .sum::<usize>(),
        1_200
    );
    let cookie = service.sign_in(login(ADMINISTRATOR)).await?;
    let first = read(&service, &cookie, "/directory/people?q=").await?;
    assert_eq!(rows(&first, "people")?.len(), 50, "default page size");
    assert_eq!(first["total"], 240);
    assert_eq!(first["agents_total"], 1_200);
    let capped = read(&service, &cookie, "/directory/people?limit=201").await?;
    assert_eq!(rows(&capped, "people")?.len(), 200, "maximum page size");
    let mut seen = BTreeSet::new();
    let mut answer = first;
    let mut pages = 0;
    loop {
        pages += 1;
        assert_eq!(answer["total"], 240);
        assert_eq!(answer["agents_total"], 1_200);
        for person in rows(&answer, "people")? {
            let id = person["id"].as_str().ok_or("person has no id")?;
            assert!(seen.insert(id.to_owned()), "person repeated across pages");
            assert_eq!(rows(person, "agents")?.len(), 5);
        }
        let Some(cursor) = answer["next"].as_str() else {
            assert_eq!(answer.get("next"), Some(&Value::Null));
            break;
        };
        answer = read(
            &service,
            &cookie,
            &format!("/directory/people?q=&after={cursor}"),
        )
        .await?;
    }
    assert_eq!(pages, 5);
    assert_eq!(seen.len(), 240);
    let subtree = read(
        &service,
        &cookie,
        &format!("/directory/people?team={}&limit=200", fixture.teams[0]),
    )
    .await?;
    assert_eq!(subtree["total"], 120);
    assert_eq!(subtree["agents_total"], 600);
    assert_eq!(rows(&subtree, "people")?.len(), 120);
    assert_eq!(subtree.get("next"), Some(&Value::Null));
    let searched = read(&service, &cookie, "/directory/people?q=Agent%20007-").await?;
    assert_eq!(searched["total"], 1);
    assert_eq!(searched["agents_total"], 5);
    assert_eq!(searched["people"][0]["id"], fixture.people[7].id);
    Ok(())
}
