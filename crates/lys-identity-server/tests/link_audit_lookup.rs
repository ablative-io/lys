//! The link-audit source asks which person holds a login, by the login's
//! exact issuer and subject. Only the configured source may ask, the person
//! is read from the directory's bindings and never inferred, and a login no
//! person holds is refused by name.

use std::error::Error;

use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, LINK_AUDIT_SOURCE, Service};
use lys_identity_server::dev_seed::{Seeded, seed_configured};
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

const PATH: &str = "/link-audit/person";
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

fn person_id(seeded: &Seeded, subject: &str) -> Result<String, Box<dyn Error>> {
    Ok(seeded
        .people
        .iter()
        .find(|person| person.subject == subject)
        .ok_or("the seed holds no such person")?
        .id
        .to_string())
}

fn refused(answer: &(u16, Value), status: u16, name: &str) {
    assert_eq!(answer.0, status, "{}", answer.1);
    assert_eq!(answer.1["refusal"], name, "{}", answer.1);
    assert!(answer.1.get("person").is_none(), "{}", answer.1);
}

#[tokio::test]
async fn the_source_learns_the_person_who_holds_a_login() -> TestResult {
    let (service, seeded) = seeded().await?;
    let source = service.sign_in(login(LINK_AUDIT_SOURCE)).await?;
    for subject in [ADA, BEA] {
        let asked = json!({ "issuer": service.issuer.issuer(), "subject": subject });
        let (status, answer) = service.post(PATH, Some(&source), &asked).await?;
        assert_eq!(status, 200, "{answer}");
        assert_eq!(answer, json!({ "person": person_id(&seeded, subject)? }));
    }
    Ok(())
}

#[tokio::test]
async fn a_login_no_person_holds_is_refused_by_name() -> TestResult {
    let (service, _seeded) = seeded().await?;
    let source = service.sign_in(login(LINK_AUDIT_SOURCE)).await?;
    let issuer = service.issuer.issuer();
    let unbound = [
        json!({ "issuer": issuer, "subject": "nobody-subject" }),
        json!({ "issuer": "https://another.issuer.test", "subject": ADA }),
        json!({ "issuer": issuer, "subject": ADA.to_uppercase() }),
        json!({ "issuer": issuer, "subject": LINK_AUDIT_SOURCE }),
    ];
    for asked in unbound {
        let answer = service.post(PATH, Some(&source), &asked).await?;
        refused(&answer, 404, "LoginUnbound");
    }
    Ok(())
}

#[tokio::test]
async fn only_the_configured_source_may_ask() -> TestResult {
    let (service, _seeded) = seeded().await?;
    let asked = json!({ "issuer": service.issuer.issuer(), "subject": ADA });
    let anonymous = service.post(PATH, None, &asked).await?;
    refused(&anonymous, 401, "NotSignedIn");
    for subject in [ADMINISTRATOR, ADA, BEA] {
        let cookie = service.sign_in(login(subject)).await?;
        let answer = service.post(PATH, Some(&cookie), &asked).await?;
        refused(&answer, 403, "NotAdmitted");
    }
    Ok(())
}

#[tokio::test]
async fn a_question_in_other_words_is_refused() -> TestResult {
    let (service, _seeded) = seeded().await?;
    let source = service.sign_in(login(LINK_AUDIT_SOURCE)).await?;
    let issuer = service.issuer.issuer();
    let malformed = [
        json!({ "issuer": issuer, "subject": ADA, "email": "shared@example.test" }),
        json!({ "issuer": issuer }),
        json!({ "issuer": issuer, "subject": "" }),
    ];
    for asked in malformed {
        let answer = service.post(PATH, Some(&source), &asked).await?;
        assert_eq!(answer.0, 400, "{}", answer.1);
        assert!(answer.1.get("person").is_none(), "{}", answer.1);
    }
    Ok(())
}
