//! A person's budget limits that person, so only an administrator sets it:
//! the person it holds reads it and cannot raise it.

use std::error::Error;

use identity_contract::apps::{Auth, send};
use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_identity_server::dev_seed::seed_configured;
use serde_json::json;

type TestResult = Result<(), Box<dyn Error>>;

const BEA: &str = "bea-subject";

fn login(subject: &str) -> Login {
    Login {
        subject: subject.to_owned(),
        email: "shared@example.test".to_owned(),
    }
}

#[tokio::test]
async fn a_person_cannot_raise_the_budget_the_administrator_set_on_them() -> TestResult {
    let (service, seeded) =
        Service::start_with(|config| Ok(seed_configured(config, [ADMINISTRATOR, BEA])?)).await?;
    let ada = service.sign_in(login(ADMINISTRATOR)).await?;
    let bea = service.sign_in(login(BEA)).await?;
    let path = format!("/budgets/person/{}", seeded.people[1].id);
    let set = json!({ "measure": "context_percent", "limit": 50, "act": "compact", "version": 0 });
    let (status, answer) = send(
        &service,
        reqwest::Method::PUT,
        &path,
        Auth::Cookie(&ada),
        Some(&set),
    )
    .await?;
    assert_eq!(status, 200, "{answer}");

    let (status, read) = send(
        &service,
        reqwest::Method::GET,
        &path,
        Auth::Cookie(&bea),
        None,
    )
    .await?;
    assert_eq!(status, 200, "the person reads their own budget: {read}");
    assert_eq!(read["budgets"][0]["limit"], 50, "{read}");

    let raised =
        json!({ "measure": "context_percent", "limit": 100, "act": "compact", "version": 1 });
    let (status, answer) = send(
        &service,
        reqwest::Method::PUT,
        &path,
        Auth::Cookie(&bea),
        Some(&raised),
    )
    .await?;
    assert_eq!(status, 403, "{answer}");
    assert_eq!(answer["refusal"], "not_permitted", "{answer}");

    let (_, kept) = send(
        &service,
        reqwest::Method::GET,
        &path,
        Auth::Cookie(&ada),
        None,
    )
    .await?;
    assert_eq!(kept["budgets"][0]["limit"], 50, "{kept}");
    assert_eq!(kept["budgets"][0]["version"], 1, "{kept}");
    Ok(())
}
