//! The tree is scoped to the caller and reports absence without inventing state.

use std::error::Error;

use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_identity::OperationId;
use lys_identity_server::dev_seed::seed_configured;
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

fn login(subject: &str) -> Login {
    Login {
        subject: subject.to_owned(),
        email: "operator@example.test".to_owned(),
    }
}

#[tokio::test]
async fn the_tree_shows_only_owned_teams_and_never_turns_people_into_members() -> TestResult {
    let other = "another-subject";
    let (service, seeded) =
        Service::start_with(|config| Ok(seed_configured(config, [ADMINISTRATOR, other])?)).await?;
    let own_cookie = service.sign_in(login(ADMINISTRATOR)).await?;
    let other_cookie = service.sign_in(login(other)).await?;
    let own = OperationId::generate()?.to_string();
    let elsewhere = OperationId::generate()?.to_string();
    for (id, cookie) in [(&own, &own_cookie), (&elsewhere, &other_cookie)] {
        let (status, answer) = service
            .post(
                "/teams",
                Some(cookie),
                &json!({
                    "operation": id, "name": "owned team",
                }),
            )
            .await?;
        assert_eq!(status, 200, "{answer}");
    }
    let agent = seeded.people[0].agents[0].id.to_string();
    let person = seeded.people[0].id.to_string();
    for member in [&agent, &person] {
        let (status, answer) = service
            .post(
                &format!("/teams/{own}/members"),
                Some(&own_cookie),
                &json!({
                    "operation": OperationId::generate()?.to_string(), "member": member,
                }),
            )
            .await?;
        assert_eq!(status, 200, "{answer}");
    }
    let (status, answer) = service.get("/tree", Some(&own_cookie)).await?;
    assert_eq!(status, 200, "{answer}");
    assert_eq!(answer["root"]["id"], person);
    let teams = answer["teams"].as_array().ok_or("tree has no teams")?;
    assert_eq!(teams.len(), 1, "{answer}");
    assert_eq!(teams[0]["id"], own);
    assert_eq!(teams[0]["lead"], Value::Null);
    let members = teams[0]["members"]
        .as_array()
        .ok_or("team has no members")?;
    assert_eq!(members.len(), 1, "people are not tree nodes: {answer}");
    assert_eq!(members[0]["id"], agent);
    assert_eq!(members[0]["session"], "stopped");
    assert_eq!(members[0]["profile"], Value::Null);
    assert_eq!(members[0]["goals"], json!([]));
    assert_eq!(members[0]["budgets"], json!([]));
    assert_eq!(teams[0]["teams"], json!([]));
    let (status, other_tree) = service.get("/tree", Some(&other_cookie)).await?;
    assert_eq!(status, 200, "{other_tree}");
    assert_eq!(
        other_tree["teams"]
            .as_array()
            .ok_or("no other teams")?
            .len(),
        1
    );
    assert_eq!(other_tree["teams"][0]["id"], elsewhere);
    Ok(())
}

#[tokio::test]
async fn the_tree_requires_authentication() -> TestResult {
    let (service, seeded) = Service::start_with(|config| {
        Ok(seed_configured(config, [ADMINISTRATOR, "another-subject"])?)
    })
    .await?;
    assert_eq!(seeded.people.len(), 1);
    let (status, answer) = service.get("/tree", None).await?;
    assert_eq!(status, 401, "{answer}");
    assert_eq!(answer["refusal"], "NotSignedIn");
    Ok(())
}
