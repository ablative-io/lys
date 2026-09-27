//! The read routes: each route's shape, personal scoping across two people,
//! the administrator's separate wider view, refusals by name that leak no
//! other person's records, and no read that writes. Every answer is read into
//! the route's own wire type.

use std::error::Error;

use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_identity_server::dev_seed::{Seeded, SeededPerson, seed_configured};
use lys_identity_server::read_views::{AgentView, MeView, PeopleView, PersonView};
use serde::de::DeserializeOwned;
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

/// GET `path` as `cookie`, which must answer 200, read into the route's wire
/// type, and the raw body beside it.
async fn read<T: DeserializeOwned>(
    service: &Service,
    path: &str,
    cookie: &str,
) -> Result<(T, Value), Box<dyn Error>> {
    let (status, body) = service.get(path, Some(cookie)).await?;
    assert_eq!(status, 200, "{path}: {body}");
    Ok((serde_json::from_value(body.clone())?, body))
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

fn states(person: &PersonView) -> Vec<&str> {
    let mut states = person
        .agents
        .iter()
        .map(|agent| agent.state.as_str())
        .collect::<Vec<_>>();
    states.sort_unstable();
    states
}

#[tokio::test]
async fn me_answers_the_signed_in_person_their_sign_in_identities_and_service_accounts_apart()
-> TestResult {
    let (service, seeded) = seeded().await?;
    let ada = person(&seeded, ADA)?;
    let cookie = service.sign_in(login(ADA)).await?;
    let (me, body) = read::<MeView>(&service, "/me", &cookie).await?;
    assert_eq!(me.person.id, ada.id.to_string());
    assert_eq!(me.person.display_name, ada.display_name);
    assert_eq!(me.person.state, "active");
    assert_eq!(me.sign_in_identities.len(), 1, "{body}");
    assert_eq!(me.sign_in_identities[0].provider, service.issuer.issuer());
    assert_eq!(me.sign_in_identities[0].subject, ADA);
    assert_eq!(me.signed_in, me.sign_in_identities[0]);
    assert!(
        me.service_accounts.is_empty(),
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
        let (people, body) = read::<PeopleView>(&service, "/people", &cookie).await?;
        assert_eq!(people.scope, "personal");
        assert_eq!(people.people.len(), 1, "{body}");
        let answered = &people.people[0];
        assert_eq!(answered.id, own.id.to_string());
        assert_eq!(states(answered), expected, "{body}");
        for seeded_agent in &own.agents {
            let agent = answered
                .agents
                .iter()
                .find(|agent| agent.id == seeded_agent.id.to_string())
                .ok_or("a seeded agent is missing")?;
            assert_eq!(agent.display_name, seeded_agent.display_name);
            assert_eq!(agent.state, seeded_agent.state.to_string());
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
        let path = format!("/agents/{}", agent.id);
        let (view, body) = read::<AgentView>(&service, &path, &cookie).await?;
        assert_eq!(view.id, agent.id.to_string());
        assert_eq!(view.display_name, agent.display_name);
        assert_eq!(view.state, agent.state.to_string());
        assert_eq!(view.person.id, ada.id.to_string());
        assert_eq!(view.person.display_name, ada.display_name);
        assert!(!view.needs_new_person);
        assert!(view.role.is_none() && view.version.is_none(), "{body}");
        assert_eq!(view.provenance.registered_by.subject, ADMINISTRATOR);
        let first = *view.provenance.events.first().ok_or("no events")?;
        let registration = view
            .provenance
            .registration
            .as_ref()
            .ok_or("no registration receipt")?;
        assert_eq!(registration["identity"], agent.id.to_string());
        assert_eq!(registration["log"]["index"], first);
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
    let (people, body) = read::<PeopleView>(&service, "/directory/people", &cookie).await?;
    assert_eq!(people.scope, "directory");
    assert_eq!(people.people.len(), 2, "{body}");
    for seeded_person in &seeded.people {
        let answered = people
            .people
            .iter()
            .find(|person| person.id == seeded_person.id.to_string())
            .ok_or("a seeded person is missing")?;
        assert_eq!(answered.agents.len(), seeded_person.agents.len(), "{body}");
    }
    let bea = person(&seeded, BEA)?;
    let path = format!("/directory/agents/{}", bea.agents[1].id);
    let (view, _) = read::<AgentView>(&service, &path, &cookie).await?;
    assert_eq!(view.state, "retired");
    assert_eq!(view.person.id, bea.id.to_string());
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
    let (before, _) = read::<PeopleView>(&service, "/directory/people", &administrator).await?;
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
    let (after, _) = read::<PeopleView>(&service, "/directory/people", &administrator).await?;
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
