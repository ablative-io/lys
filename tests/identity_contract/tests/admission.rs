#![cfg(test)]

//! R3: the configured administrator admitted, every other caller refused by
//! name, and an email or a first visit never admitted (`ID001_ADMIN`).

use std::error::Error;

use identity_contract::fake_issuer::Login;
use identity_contract::fixtures::op;
use identity_contract::harness::{ADMINISTRATOR, GRANT_MODEL, Service};
use lys_identity_server::admission::AUTHORITY;
use lys_identity_server::config::ConfiguredLogin;
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

const SHARED_EMAIL: &str = "admin@example.test";

fn login(subject: &str) -> Login {
    Login {
        subject: subject.to_owned(),
        email: SHARED_EMAIL.to_owned(),
    }
}

fn person(n: u8, name: &str) -> serde_json::Value {
    json!({ "operation": op(n).to_string(), "display_name": name })
}

#[tokio::test]
async fn the_configured_administrator_is_admitted() -> TestResult {
    let service = Service::start().await?;
    let cookie = service.sign_in(login(ADMINISTRATOR)).await?;
    let (status, body) = service
        .post("/people", Some(&cookie), &person(1, "Ada"))
        .await?;
    assert_eq!(status, 200, "{body}");
    assert!(
        body["person"]
            .as_str()
            .is_some_and(|id| id.starts_with("person-"))
    );
    let (status, body) = service.get("/identities", Some(&cookie)).await?;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["identities"].as_array().map(Vec::len), Some(1));
    Ok(())
}

#[tokio::test]
async fn another_login_with_the_administrators_email_is_refused_by_name() -> TestResult {
    let service = Service::start().await?;
    let other = service.sign_in(login("someone-else")).await?;
    let (status, body) = service
        .post("/people", Some(&other), &person(1, "Mallory"))
        .await?;
    assert_eq!(status, 403, "{body}");
    assert_eq!(body["refusal"], "NotAdmitted");
    let (status, body) = service.get("/identities", Some(&other)).await?;
    assert_eq!(status, 403, "{body}");
    assert_eq!(body["refusal"], "NotAdmitted", "no read either");
    let administrator = service.sign_in(login(ADMINISTRATOR)).await?;
    let (_, body) = service.get("/identities", Some(&administrator)).await?;
    assert_eq!(
        body["identities"].as_array().map(Vec::len),
        Some(0),
        "the refused caller recorded nothing and was not registered on sight"
    );
    Ok(())
}

#[tokio::test]
async fn a_caller_without_a_session_is_refused_by_name() -> TestResult {
    let service = Service::start().await?;
    let (status, body) = service.post("/people", None, &person(1, "Ada")).await?;
    assert_eq!(status, 401, "{body}");
    assert_eq!(body["refusal"], "NotSignedIn");
    let forged = "lys_directory_session=00";
    let (status, body) = service
        .post("/people", Some(forged), &person(1, "Ada"))
        .await?;
    assert_eq!(status, 401, "{body}");
    assert_eq!(body["refusal"], "NotSignedIn");
    Ok(())
}

/// A real authorization, begun by the service from a provider button, whose
/// valid answer a browser follows to `/callback`: it is sent on to the
/// screens, signed in, and the same answer is never taken twice.
#[tokio::test]
async fn a_browser_coming_back_from_sign_in_is_taken_to_the_screens_signed_in() -> TestResult {
    let service = Service::start().await?;
    let (back, binding) = service.issuer_answer(login(ADMINISTRATOR)).await?;
    let (status, to, cookie) = service.get_page(&back, &binding).await?;
    assert_eq!(status, 303, "a browser is sent on, never shown the JSON");
    assert_eq!(to.as_deref(), Some("/#/me"), "it lands on its own screen");
    assert!(cookie, "and it arrives signed in");
    for again in [binding.as_str(), ""] {
        let (status, to, cookie) = service.get_page(&back, again).await?;
        assert_eq!(status, 303, "an answer is used once");
        assert_eq!(to.as_deref(), Some("/#/sign-in?refused=SignInStateUnknown"));
        assert!(!cookie, "and a second use signs nobody in");
    }
    Ok(())
}

#[tokio::test]
async fn a_first_visit_is_sent_to_lys_and_admits_nobody() -> TestResult {
    let service = Service::start().await?;
    let browser = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    let first = browser
        .get(format!("{}/login", service.base))
        .send()
        .await?;
    assert_eq!(first.status(), 303);
    assert_eq!(
        first.headers().get(reqwest::header::LOCATION),
        Some(&reqwest::header::HeaderValue::from_static("/#/sign-in")),
        "a first visit is sent to Lys's own sign-in screen"
    );
    let (status, body) = service.post("/people", None, &person(1, "Ada")).await?;
    assert_eq!(status, 401, "a first visit is not signed in: {body}");
    let (status, body) = service
        .get("/callback?code=forged&state=never-begun", None)
        .await?;
    assert_eq!(status, 400, "{body}");
    assert_eq!(body["refusal"], "SignInStateUnknown");
    service.issuer.sign_in_as(login(ADMINISTRATOR))?;
    let signed_in = browser
        .post(format!("{}/sign-in", service.base))
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(json!({ "email": SHARED_EMAIL, "password": "Any-Password-123" }).to_string())
        .send()
        .await?;
    assert_eq!(signed_in.status(), 200);
    let body: Value = serde_json::from_str(&signed_in.text().await?)?;
    assert_eq!(
        body["authority"], AUTHORITY,
        "the authority is shown at sign-in"
    );
    assert_eq!(body["signed_in"]["subject"], ADMINISTRATOR);
    Ok(())
}

/// Every mutation route of the directory, each as `caller` would send it
/// against `person_id` and `agent_id`, answering how many were refused with
/// `status` and the refusal `name`.
async fn refused_everywhere(
    service: &Service,
    caller: Option<&str>,
    (person_id, agent_id): (&str, &str),
    (status, name): (u16, &str),
) -> Result<usize, Box<dyn Error>> {
    let mutations: [(String, Value); 5] = [
        ("/people".to_owned(), person(20, "Mallory")),
        ("/agents".to_owned(), person(21, "Mallory's agent")),
        (
            format!("/identities/{person_id}/profile"),
            person(22, "Ada renamed"),
        ),
        (
            format!("/identities/{agent_id}/transitions"),
            json!({ "operation": op(23).to_string(), "transition": "activate" }),
        ),
        (
            format!("/people/{person_id}/logins"),
            json!({
                "operation": op(24).to_string(),
                "issuer": service.issuer.issuer(),
                "subject": "mallory",
            }),
        ),
    ];
    let mut refused = 0;
    for (path, body) in &mutations {
        let (answered, body) = service.post(path, caller, body).await?;
        assert_eq!(answered, status, "{path}: {body}");
        assert_eq!(body["refusal"], name, "{path}: {body}");
        refused += 1;
    }
    Ok(refused)
}

/// `ID001_ADMIN`: an unauthenticated caller and a non-administrator with the
/// administrator's email are refused by name on every mutation route, and
/// the refused calls change nothing: the log holds exactly what the
/// administrator wrote and every record reads as it did.
#[tokio::test]
async fn every_mutation_by_a_denied_caller_is_refused_and_changes_nothing() -> TestResult {
    let service = Service::start().await?;
    let administrator = service.sign_in(login(ADMINISTRATOR)).await?;
    let (status, body) = service
        .post("/people", Some(&administrator), &person(1, "Ada"))
        .await?;
    assert_eq!(status, 200, "{body}");
    let ada = body["person"].as_str().ok_or("no person")?.to_owned();
    // Activate the administrator before binding its login: Registered people
    // may sign in, but cannot administer the directory.
    let (status, answer) = service
        .post(
            &format!("/identities/{ada}/transitions"),
            Some(&administrator),
            &json!({ "operation": op(250).to_string(), "transition": "activate" }),
        )
        .await?;
    assert_eq!(status, 200, "{answer}");
    let bind = json!({
        "operation": op(2).to_string(),
        "issuer": service.issuer.issuer(),
        "subject": ADMINISTRATOR,
    });
    let path = format!("/people/{ada}/logins");
    let (status, body) = service.post(&path, Some(&administrator), &bind).await?;
    assert_eq!(status, 200, "{body}");
    let (status, body) = service
        .post("/agents", Some(&administrator), &person(3, "Builder"))
        .await?;
    assert_eq!(status, 200, "{body}");
    let agent = body["agent"].as_str().ok_or("no agent")?.to_owned();
    let size = service.log_size().await?;
    assert_eq!(size, 4, "the administrator's four changes are logged");
    let (_, before) = service.get("/identities", Some(&administrator)).await?;

    let same_email = service.sign_in(login("someone-else")).await?;
    let targets = (ada.as_str(), agent.as_str());
    let unsigned = refused_everywhere(&service, None, targets, (401, "NotSignedIn")).await?;
    let denied = (403, "NotAdmitted");
    let emailed = refused_everywhere(&service, Some(&same_email), targets, denied).await?;
    assert_eq!(unsigned + emailed, 10, "five routes, two callers");

    assert_eq!(service.log_size().await?, size, "nothing was logged");
    let (_, after) = service.get("/identities", Some(&administrator)).await?;
    assert_eq!(after, before, "no refused call changed a record");
    Ok(())
}

/// `ID001_ADMIN`: the configured subject signing in at another issuer is not
/// the administrator. The service is configured with the administrator at
/// an issuer nobody signs in through, so the fake issuer's own
/// `administrator-subject` is the right subject at the wrong issuer, and
/// every mutation it asks for is refused with nothing logged.
#[tokio::test]
async fn the_administrators_subject_at_another_issuer_is_refused() -> TestResult {
    let (service, ()) = Service::start_adjusted(
        GRANT_MODEL,
        None,
        None,
        None,
        |config| {
            config.administrator = Some(ConfiguredLogin {
                issuer: "https://elsewhere.example.test".to_owned(),
                subject: ADMINISTRATOR.to_owned(),
            });
        },
        |_| Ok(()),
    )
    .await?;
    let wrong_issuer = service.sign_in(login(ADMINISTRATOR)).await?;
    let person_id = format!("person-{}", "0".repeat(32));
    let agent_id = format!("agent-{}", "0".repeat(32));
    let refusals = refused_everywhere(
        &service,
        Some(&wrong_issuer),
        (person_id.as_str(), agent_id.as_str()),
        (403, "NotAdmitted"),
    )
    .await?;
    assert_eq!(refusals, 5, "every route refused it");
    let (status, body) = service.get("/identities", Some(&wrong_issuer)).await?;
    assert_eq!(status, 403, "{body}");
    assert_eq!(body["refusal"], "NotAdmitted");
    assert_eq!(service.log_size().await?, 0, "nothing was logged");
    Ok(())
}
