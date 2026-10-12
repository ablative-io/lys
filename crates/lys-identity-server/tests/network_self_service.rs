//! Personal computer acts are confined to the person and their own agents.

use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, GRANT_MODEL, Service};
use lys_identity::{LifecycleState, OperationId};
use lys_identity_server::dev_seed::{Seeded, seed_configured};
use serde_json::{Value, json};
use std::error::Error;

type Outcome = Result<(), Box<dyn Error>>;
const MEMBER: &str = "computer-owner";

fn operation() -> Result<String, Box<dyn Error>> {
    Ok(OperationId::generate()?.to_string())
}
fn login(subject: &str) -> Login {
    Login {
        subject: subject.to_owned(),
        email: "computer@example.test".to_owned(),
    }
}
fn computer(id: &str, agents: &[String]) -> Value {
    json!({"operation": id, "name": "Own laptop", "kind": "Computer",
        "runtime": "lys-runner", "slots": 0, "may_run": agents,
        "may_run_roles": [], "may_reach": []})
}
fn active_agent(seeded: &Seeded, index: usize) -> Result<String, Box<dyn Error>> {
    Ok(seeded.people[index]
        .agents
        .iter()
        .find(|agent| agent.state == LifecycleState::Active)
        .ok_or("active agent missing")?
        .id
        .to_string())
}
async fn fixture() -> Result<(Service, Seeded, String, String), Box<dyn Error>> {
    let (service, seeded) = Service::start_adjusted(
        GRANT_MODEL,
        None,
        None,
        None,
        |config| "https://lys.example.test/api/callback".clone_into(&mut config.redirect_url),
        |config| Ok(seed_configured(config, [ADMINISTRATOR, MEMBER])?),
    )
    .await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    let member = service.sign_in(login(MEMBER)).await?;
    Ok((service, seeded, admin, member))
}
fn named_refusal(answer: &(u16, Value), name: &str) {
    assert_eq!(answer.0, 403, "{}", answer.1);
    assert_eq!(answer.1["refusal"], name, "{}", answer.1);
}

#[tokio::test]
async fn network_person_adds_own_computer_allows_own_agent_and_joins() -> Outcome {
    let (service, seeded, admin, member) = fixture().await?;
    let own = active_agent(&seeded, 1)?;
    let team = operation()?;
    let (status, answer) = service
        .post(
            "/teams",
            Some(&member),
            &json!({"operation": team, "name": "Personal team"}),
        )
        .await?;
    assert_eq!(status, 200, "{answer}");
    let id = operation()?;
    let body = computer(&id, std::slice::from_ref(&own));
    let first = service
        .post("/network/machines", Some(&member), &body)
        .await?;
    assert_eq!(first.0, 200, "{}", first.1);
    assert_eq!(first.1["owner"], seeded.people[1].id.to_string());
    assert_eq!(first.1["named_by"], first.1["owner"]);
    assert_eq!(first.1["team"], team);
    assert_eq!(first.1["may_run"][0]["id"], own);
    assert_eq!(first.1["may_run_roles"], json!([]));
    assert_eq!(
        service
            .post("/network/machines", Some(&member), &body)
            .await?,
        first
    );
    for allow in [false, true] {
        let answer = service
            .post(
                &format!("/network/machines/{id}/agents"),
                Some(&member),
                &json!({"operation": operation()?, "agent": own, "allow": allow}),
            )
            .await?;
        assert_eq!(answer.0, 200, "{}", answer.1);
        assert_eq!(
            answer.1["machine"]["may_run"]
                .as_array()
                .ok_or("agents missing")?
                .len(),
            usize::from(allow)
        );
    }
    let foreign = active_agent(&seeded, 0)?;
    let denied = service
        .post(
            &format!("/network/machines/{id}/agents"),
            Some(&member),
            &json!({"operation": operation()?, "agent": foreign, "allow": true}),
        )
        .await?;
    named_refusal(&denied, "agent_not_yours");
    let (status, given) = service
        .post(
            &format!("/network/machines/{id}/join-code"),
            Some(&member),
            &json!({"operation": operation()?}),
        )
        .await?;
    assert_eq!(status, 200, "{given}");
    assert_eq!(given["machine"], id);
    assert_eq!(given["server"], "https://lys.example.test/api");
    let key = lys_core::Ed25519Identity::from_seed(&zeroize::Zeroizing::new([41; 32]));
    let answer = service
        .post(
            "/runner/join",
            None,
            &json!({"machine": id, "code": given["code"], "key":
            lys_runner::protocol::hex(&key.public_key_bytes())}),
        )
        .await?;
    assert_eq!(answer.0, 200, "{}", answer.1);
    let (status, machines) = service
        .get("/network/machine-identities", Some(&admin))
        .await?;
    assert_eq!(status, 200, "{machines}");
    assert_eq!(
        machines["machines"][0]["responsible"],
        seeded.people[1].id.to_string()
    );
    let (status, seen) = service.get("/network", Some(&member)).await?;
    assert_eq!(status, 200, "{seen}");
    assert_eq!(seen["machines"][0]["id"], id);
    Ok(())
}

#[tokio::test]
async fn network_person_cannot_change_or_join_another_persons_computer() -> Outcome {
    let (service, seeded, admin, member) = fixture().await?;
    let id = operation()?;
    let answer = service
        .post("/network/machines", Some(&admin), &computer(&id, &[]))
        .await?;
    assert_eq!(answer.0, 200, "{}", answer.1);
    for (suffix, body) in [
        ("join-code", json!({"operation": operation()?})),
        (
            "agents",
            json!({"operation": operation()?, "agent": active_agent(&seeded, 1)?, "allow": true}),
        ),
    ] {
        named_refusal(
            &service
                .post(
                    &format!("/network/machines/{id}/{suffix}"),
                    Some(&member),
                    &body,
                )
                .await?,
            "not_your_computer",
        );
    }
    let foreign = active_agent(&seeded, 0)?;
    named_refusal(
        &service
            .post(
                "/network/machines",
                Some(&member),
                &computer(&operation()?, &[foreign]),
            )
            .await?,
        "agent_not_yours",
    );
    let mut roles = computer(&operation()?, &[]);
    roles["may_run_roles"] = json!([operation()?]);
    named_refusal(
        &service
            .post("/network/machines", Some(&member), &roles)
            .await?,
        "agent_not_yours",
    );
    Ok(())
}

#[tokio::test]
async fn network_admin_still_names_agents_and_issues_join_codes() -> Outcome {
    let (service, seeded, admin, member) = fixture().await?;
    let id = operation()?;
    let own = active_agent(&seeded, 1)?;
    let first = service
        .post(
            "/network/machines",
            Some(&admin),
            &computer(&id, std::slice::from_ref(&own)),
        )
        .await?;
    assert_eq!(first.0, 200, "{}", first.1);
    assert!(first.1["team"].is_null());
    let answer = service
        .post(
            &format!("/network/machines/{id}/agents"),
            Some(&admin),
            &json!({"operation": operation()?, "agent": own, "allow": false}),
        )
        .await?;
    assert_eq!(answer.0, 200, "{}", answer.1);
    let answer = service
        .post(
            &format!("/network/machines/{id}/join-code"),
            Some(&admin),
            &json!({"operation": operation()?}),
        )
        .await?;
    assert_eq!(answer.0, 200, "{}", answer.1);
    named_refusal(
        &service
            .post(
                &format!("/network/machines/{id}/join-code"),
                Some(&member),
                &json!({"operation": operation()?}),
            )
            .await?,
        "not_your_computer",
    );
    Ok(())
}

#[tokio::test]
async fn network_one_runner_is_held_by_one_live_machine() -> Outcome {
    let (service, _, admin, _) = fixture().await?;
    for (first_runner, second_runner) in [
        (
            json!({"kind": "socket", "path": "/tmp/lys-shared-runner.sock"}),
            json!({"kind": "socket", "path": "/tmp/lys-shared-runner.sock"}),
        ),
        (
            json!({"kind": "dialled", "key": "12".repeat(32), "runner": "ab".repeat(16)}),
            json!({"kind": "dialled", "key": "13".repeat(32), "runner": "ab".repeat(16)}),
        ),
    ] {
        let first = operation()?;
        let second = operation()?;
        for id in [&first, &second] {
            let answer = service
                .post("/network/machines", Some(&admin), &computer(id, &[]))
                .await?;
            assert_eq!(answer.0, 200, "{}", answer.1);
        }
        let answer = service
            .post(
                &format!("/network/machines/{first}/runner"),
                Some(&admin),
                &json!({"runner": first_runner}),
            )
            .await?;
        assert_eq!(answer.0, 200, "{}", answer.1);
        let answer = service
            .post(
                &format!("/network/machines/{second}/runner"),
                Some(&admin),
                &json!({"runner": second_runner}),
            )
            .await?;
        assert_eq!(answer.0, 409, "{}", answer.1);
        assert_eq!(
            answer.1["refusal"], "runner_held_by_machine",
            "{}",
            answer.1
        );
        assert!(
            answer.1["reason"]
                .as_str()
                .ok_or("runner refusal has no reason")?
                .contains(&first)
        );
        let answer = service
            .post(
                &format!("/network/machines/{first}/retire"),
                Some(&admin),
                &json!({}),
            )
            .await?;
        assert_eq!(answer.0, 200, "{}", answer.1);
        let answer = service
            .post(
                &format!("/network/machines/{second}/runner"),
                Some(&admin),
                &json!({"runner": second_runner}),
            )
            .await?;
        assert_eq!(answer.0, 200, "{}", answer.1);
    }
    Ok(())
}

#[tokio::test]
async fn network_person_cannot_choose_a_foreign_team_or_reuse_another_owners_operation() -> Outcome
{
    let (service, _, admin, member) = fixture().await?;
    let team = operation()?;
    let answer = service
        .post(
            "/teams",
            Some(&admin),
            &json!({"operation": team, "name": "Another team"}),
        )
        .await?;
    assert_eq!(answer.0, 200, "{}", answer.1);
    let mut body = computer(&operation()?, &[]);
    body["team"] = json!(team);
    named_refusal(
        &service
            .post("/network/machines", Some(&member), &body)
            .await?,
        "not_your_computer",
    );
    let id = operation()?;
    let body = computer(&id, &[]);
    assert_eq!(
        service
            .post("/network/machines", Some(&admin), &body)
            .await?
            .0,
        200
    );
    named_refusal(
        &service
            .post("/network/machines", Some(&member), &body)
            .await?,
        "not_your_computer",
    );
    Ok(())
}

#[tokio::test]
async fn network_runner_key_cannot_be_reused_and_a_refusal_keeps_the_old_assignment() -> Outcome {
    let (service, _, admin, _) = fixture().await?;
    let first = operation()?;
    let second = operation()?;
    for id in [&first, &second] {
        assert_eq!(
            service
                .post("/network/machines", Some(&admin), &computer(id, &[]))
                .await?
                .0,
            200
        );
    }
    let runner = json!({"kind": "dialled", "key": "af".repeat(32)});
    let path = format!("/network/machines/{first}/runner");
    assert_eq!(
        service
            .post(&path, Some(&admin), &json!({"runner": runner}))
            .await?
            .0,
        200
    );
    assert_eq!(
        service
            .post(&path, Some(&admin), &json!({"runner": runner}))
            .await?
            .0,
        200
    );
    let second_path = format!("/network/machines/{second}/runner");
    let original = json!({"kind": "socket", "path": "/tmp/lys-other-runner.sock"});
    assert_eq!(
        service
            .post(&second_path, Some(&admin), &json!({"runner": original}))
            .await?
            .0,
        200
    );
    let denied = service
        .post(&second_path, Some(&admin), &json!({"runner": runner}))
        .await?;
    assert_eq!(denied.0, 409, "{}", denied.1);
    assert_eq!(denied.1["refusal"], "runner_held_by_machine");
    let uppercase = json!({"kind": "dialled", "key": "AF".repeat(32)});
    let denied = service
        .post(&second_path, Some(&admin), &json!({"runner": uppercase}))
        .await?;
    assert_eq!(denied.0, 400, "{}", denied.1);
    assert_eq!(denied.1["refusal"], "RequestMalformed");
    let seen = service.get(&second_path, Some(&admin)).await?;
    assert_eq!(seen.0, 200, "{}", seen.1);
    assert_eq!(seen.1["runner"], original);
    assert_eq!(
        service
            .post(&path, Some(&admin), &json!({"runner": null}))
            .await?
            .0,
        200
    );
    assert_eq!(
        service
            .post(&second_path, Some(&admin), &json!({"runner": runner}))
            .await?
            .0,
        200
    );
    Ok(())
}
