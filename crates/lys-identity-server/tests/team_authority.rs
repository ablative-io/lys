//! A team's goals and budgets act on its members, so a member joins only by
//! the act of someone who may speak for them: an agent's operator, or the
//! person themselves, or the administrator. A goal's words reach a session
//! as typed text, so they stay one line.

use std::error::Error;
use std::time::{SystemTime, UNIX_EPOCH};

use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_identity::OperationId;
use lys_identity_server::dev_seed::{Seeded, seed_configured};
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

const BEA: &str = "bea-subject";

fn login(subject: &str) -> Login {
    Login {
        subject: subject.to_owned(),
        email: "shared@example.test".to_owned(),
    }
}

fn operation() -> Result<String, Box<dyn Error>> {
    Ok(OperationId::generate()?.to_string())
}

/// Ada (the administrator) and Bea, each with an agent, and a team Bea made.
struct Table {
    service: Service,
    seeded: Seeded,
    bea: String,
    team: String,
}

impl Table {
    async fn set() -> Result<Self, Box<dyn Error>> {
        let (service, seeded) =
            Service::start_with(|config| Ok(seed_configured(config, [ADMINISTRATOR, BEA])?))
                .await?;
        let bea = service.sign_in(login(BEA)).await?;
        let team = operation()?;
        let (status, made) = service
            .post(
                "/teams",
                Some(&bea),
                &json!({ "operation": team, "name": "screens" }),
            )
            .await?;
        assert_eq!(status, 200, "{made}");
        Ok(Self {
            service,
            seeded,
            bea,
            team,
        })
    }

    async fn add(&self, member: &str) -> Result<(u16, Value), Box<dyn Error>> {
        let path = format!("/teams/{}/members", self.team);
        let body = json!({ "operation": operation()?, "member": member });
        self.service.post(&path, Some(&self.bea), &body).await
    }

    async fn goal(&self, words: &str) -> Result<(u16, Value), Box<dyn Error>> {
        let path = format!("/teams/{}/goals", self.team);
        let deadline = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() + 86_400;
        let body = json!({
            "operation": operation()?, "kind": "goal", "words": words, "deadline": deadline,
        });
        self.service.post(&path, Some(&self.bea), &body).await
    }
}

#[tokio::test]
async fn a_person_cannot_put_another_persons_agent_on_their_team() -> TestResult {
    let table = Table::set().await?;
    let adas_agent = table.seeded.people[0].agents[0].id.to_string();
    let (status, answer) = table.add(&adas_agent).await?;
    assert_eq!(status, 403, "{answer}");
    assert_eq!(answer["refusal"], "not_permitted", "{answer}");
    Ok(())
}

#[tokio::test]
async fn a_person_cannot_put_another_person_on_their_team() -> TestResult {
    let table = Table::set().await?;
    let ada = table.seeded.people[0].id.to_string();
    let (status, answer) = table.add(&ada).await?;
    assert_eq!(status, 403, "{answer}");
    assert_eq!(answer["refusal"], "NotAdmitted", "{answer}");
    Ok(())
}

#[tokio::test]
async fn a_person_puts_themselves_and_their_own_agent_on_their_team() -> TestResult {
    let table = Table::set().await?;
    let bea = table.seeded.people[1].id.to_string();
    let beas_agent = table.seeded.people[1].agents[0].id.to_string();
    let (status, answer) = table.add(&bea).await?;
    assert_eq!(status, 200, "{answer}");
    let (status, answer) = table.add(&beas_agent).await?;
    assert_eq!(status, 200, "{answer}");
    assert_eq!(
        answer["members"].as_array().map(Vec::len),
        Some(2),
        "{answer}"
    );
    Ok(())
}

#[tokio::test]
async fn a_team_goal_whose_words_would_type_a_second_line_is_refused() -> TestResult {
    let table = Table::set().await?;
    for words in ["ship it\nrm -rf the tree", "ship it\u{1b}[2K"] {
        let (status, answer) = table.goal(words).await?;
        assert_eq!(status, 400, "{answer}");
        assert_eq!(answer["refusal"], "RequestMalformed", "{answer}");
    }
    let (status, answer) = table.goal("ship the sign-in page").await?;
    assert_eq!(status, 200, "{answer}");
    Ok(())
}
