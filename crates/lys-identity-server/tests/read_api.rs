//! The read routes: each route's shape, personal scoping across two people,
//! the administrator's separate wider view, refusals by name that leak no
//! other person's records, and no read that writes.

use std::error::Error;

use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_identity_server::dev_seed::{Seeded, SeededPerson, seed_configured};
use serde_json::Value;

type TestResult = Result<(), Box<dyn Error>>;

const ADA: &str = "ada-subject";
const BEA: &str = "bea-subject";

fn login(subject: &str) -> Login {
    Login {
        subject: subject.to_owned(),
        email: "shared@example.test".to_owned(),
    }
}

async fn seeded() -> Result<(Service, Seeded), Box<dyn Error>> {
    Service::start_with(|config| Ok(seed_configured(config, [ADA, BEA])?)).await
}

fn person<'a>(seeded: &'a Seeded, subject: &str) -> Result<&'a SeededPerson, Box<dyn Error>> {
    Ok(seeded
        .people
        .iter()
        .find(|person| person.subject == subject)
        .ok_or("the seed holds no such person")?)
}

/// Every name and id of `person` and their agents, none of which another person's view may carry.
fn marks(person: &SeededPerson) -> Vec<String> {
    let mut marks = vec![person.id.to_string(), person.display_name.clone()];
    for agent in &person.agents {
        marks.push(agent.id.to_string());
        marks.push(agent.display_name.clone());
    }
    marks
}

fn carries_none_of(body: &Value, marks: &[String]) -> bool {
    let text = body.to_string();
    marks.iter().all(|mark| !text.contains(mark.as_str()))
}

fn states(person: &Value) -> Vec<String> {
    person["agents"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|agent| agent["state"].as_str().map(str::to_owned))
        .collect()
}

#[tokio::test]
async fn me_answers_the_signed_in_person_their_sign_in_identities_and_service_accounts_apart()
-> TestResult {
    let (service, seeded) = seeded().await?;
    let ada = person(&seeded, ADA)?;
    let cookie = service.sign_in(login(ADA)).await?;
    let (status, body) = service.get("/me", Some(&cookie)).await?;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["person"]["id"], ada.id.to_string());
    assert_eq!(body["person"]["display_name"], ada.display_name.as_str());
    assert_eq!(body["person"]["state"], "active");
    let identities = body["sign_in_identities"]
        .as_array()
        .ok_or("no sign-in identities")?;
    assert_eq!(identities.len(), 1, "{body}");
    assert_eq!(identities[0]["provider"], service.issuer.issuer());
    assert_eq!(identities[0]["subject"], ADA);
    assert_eq!(body["signed_in"]["subject"], ADA);
    assert_eq!(
        body["service_accounts"].as_array().map(Vec::len),
        Some(0),
        "service accounts are their own list, apart from sign-in identities"
    );
    assert!(carries_none_of(&body, &marks(person(&seeded, BEA)?)));
    Ok(())
}

#[tokio::test]
async fn people_answers_only_the_signed_in_person_with_their_agents() -> TestResult {
    let (service, seeded) = seeded().await?;
    for (subject, other, expected) in [
        (ADA, BEA, vec!["active", "registered", "suspended"]),
        (BEA, ADA, vec!["active", "retired"]),
    ] {
        let own = person(&seeded, subject)?;
        let cookie = service.sign_in(login(subject)).await?;
        let (status, body) = service.get("/people", Some(&cookie)).await?;
        assert_eq!(status, 200, "{body}");
        assert_eq!(body["scope"], "personal");
        let people = body["people"].as_array().ok_or("no people")?;
        assert_eq!(people.len(), 1, "{body}");
        assert_eq!(people[0]["id"], own.id.to_string());
        let mut answered = states(&people[0]);
        answered.sort();
        assert_eq!(answered, expected, "{body}");
        let answered_agents = people[0]["agents"].as_array().ok_or("no agents")?;
        for seeded_agent in &own.agents {
            let agent = answered_agents
                .iter()
                .find(|agent| agent["id"] == seeded_agent.id.to_string())
                .ok_or("a seeded agent is missing")?;
            assert_eq!(agent["display_name"], seeded_agent.display_name.as_str());
            assert_eq!(agent["state"], seeded_agent.state.to_string());
        }
        assert!(
            carries_none_of(&body, &marks(person(&seeded, other)?)),
            "{body}"
        );
    }
    Ok(())
}

#[tokio::test]
async fn an_agent_answers_its_person_role_version_state_and_provenance() -> TestResult {
    let (service, seeded) = seeded().await?;
    let ada = person(&seeded, ADA)?;
    let cookie = service.sign_in(login(ADA)).await?;
    for agent in &ada.agents {
        let (status, body) = service
            .get(&format!("/agents/{}", agent.id), Some(&cookie))
            .await?;
        assert_eq!(status, 200, "{body}");
        assert_eq!(body["id"], agent.id.to_string());
        assert_eq!(body["display_name"], agent.display_name.as_str());
        assert_eq!(body["state"], agent.state.to_string());
        assert_eq!(body["person"]["id"], ada.id.to_string());
        assert_eq!(body["person"]["display_name"], ada.display_name.as_str());
        assert_eq!(body["needs_new_person"], false);
        assert!(
            body["role"].is_null() && body["version"].is_null(),
            "{body}"
        );
        let provenance = &body["provenance"];
        assert_eq!(provenance["registered_by"]["subject"], ADMINISTRATOR);
        let events = provenance["events"].as_array().ok_or("no events")?;
        assert!(!events.is_empty(), "{body}");
        assert_eq!(provenance["registration"]["identity"], agent.id.to_string());
        assert_eq!(provenance["registration"]["log"]["index"], events[0]);
    }
    Ok(())
}

#[tokio::test]
async fn another_persons_agent_is_refused_as_an_unknown_one_is_and_leaks_nothing() -> TestResult {
    let (service, seeded) = seeded().await?;
    let bea = person(&seeded, BEA)?;
    let cookie = service.sign_in(login(ADA)).await?;
    let unknown = "agent-00000000000000000000000000000000";
    let (status, refused_unknown) = service
        .get(&format!("/agents/{unknown}"), Some(&cookie))
        .await?;
    assert_eq!(status, 404, "{refused_unknown}");
    assert_eq!(refused_unknown["refusal"], "AgentNotVisible");
    for agent in &bea.agents {
        let (status, body) = service
            .get(&format!("/agents/{}", agent.id), Some(&cookie))
            .await?;
        assert_eq!(status, 404, "{body}");
        assert_eq!(
            body, refused_unknown,
            "not yours answers exactly as not held"
        );
        assert!(carries_none_of(&body, &marks(bea)), "{body}");
    }
    let (status, body) = service.get("/directory/people", Some(&cookie)).await?;
    assert_eq!(status, 403, "{body}");
    assert_eq!(body["refusal"], "NotAdmitted");
    let path = format!("/directory/agents/{}", bea.agents[0].id);
    let (status, body) = service.get(&path, Some(&cookie)).await?;
    assert_eq!(status, 403, "{body}");
    assert!(carries_none_of(&body, &marks(bea)), "{body}");
    Ok(())
}

#[tokio::test]
async fn the_administrators_wider_view_is_its_own_route() -> TestResult {
    let (service, seeded) = seeded().await?;
    let cookie = service.sign_in(login(ADMINISTRATOR)).await?;
    let (status, body) = service.get("/directory/people", Some(&cookie)).await?;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["scope"], "directory");
    let people = body["people"].as_array().ok_or("no people")?;
    assert_eq!(people.len(), 2, "{body}");
    let agents: usize = people.iter().map(|person| states(person).len()).sum();
    assert_eq!(agents, 5, "{body}");
    let bea = person(&seeded, BEA)?;
    let path = format!("/directory/agents/{}", bea.agents[1].id);
    let (status, body) = service.get(&path, Some(&cookie)).await?;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["state"], "retired");
    assert_eq!(body["person"]["id"], bea.id.to_string());
    let (status, body) = service.get("/me", Some(&cookie)).await?;
    assert_eq!(status, 403, "{body}");
    assert_eq!(
        body["refusal"], "NoPerson",
        "the administrator's login is bound to no seeded person"
    );
    Ok(())
}

const ROUTES: [&str; 5] = [
    "/me",
    "/people",
    "/agents/agent-00000000000000000000000000000000",
    "/directory/people",
    "/directory/agents/agent-00000000000000000000000000000000",
];

#[tokio::test]
async fn an_unauthenticated_caller_is_refused_by_name_on_every_route() -> TestResult {
    let (service, _) = seeded().await?;
    for path in ROUTES {
        for cookie in [None, Some("lys_directory_session=00")] {
            let (status, body) = service.get(path, cookie).await?;
            assert_eq!(status, 401, "{path}: {body}");
            assert_eq!(body["refusal"], "NotSignedIn", "{path}");
        }
    }
    let stranger = service.sign_in(login("stranger")).await?;
    for path in &ROUTES[..3] {
        let (status, body) = service.get(path, Some(&stranger)).await?;
        assert_eq!(status, 403, "{path}: {body}");
        assert_eq!(body["refusal"], "NoPerson", "{path}");
    }
    Ok(())
}

#[tokio::test]
async fn no_read_and_no_refusal_writes_anything() -> TestResult {
    let (service, seeded) = seeded().await?;
    let administrator = service.sign_in(login(ADMINISTRATOR)).await?;
    let (_, before) = service
        .get("/directory/people", Some(&administrator))
        .await?;
    let ada = service.sign_in(login(ADA)).await?;
    let stranger = service.sign_in(login("stranger")).await?;
    let bea_agent = format!("/agents/{}", person(&seeded, BEA)?.agents[0].id);
    for cookie in [
        None,
        Some(ada.as_str()),
        Some(stranger.as_str()),
        Some(administrator.as_str()),
    ] {
        for path in ROUTES.iter().copied().chain([bea_agent.as_str()]) {
            service.get(path, cookie).await?;
        }
    }
    let (_, after) = service
        .get("/directory/people", Some(&administrator))
        .await?;
    assert_eq!(before, after, "the directory is as it was");
    let last = seeded.tree_size - 1;
    let (status, body) = service.get(&format!("/receipts/{last}"), None).await?;
    assert_eq!(status, 200, "the seed's last event is in the log: {body}");
    let (status, body) = service
        .get(&format!("/receipts/{}", seeded.tree_size), None)
        .await?;
    assert_eq!(status, 400, "no event was appended after the seed: {body}");
    Ok(())
}
