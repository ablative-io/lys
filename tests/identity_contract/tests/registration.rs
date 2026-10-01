#![cfg(test)]

//! R1: people and agents registered with enduring ids, each agent under its responsible person.

use std::error::Error;

use identity_contract::fake_issuer::Login;
use identity_contract::fake_rauthy::FakeRauthy;
use identity_contract::fixtures::{administrator, op, shown};
use identity_contract::harness::{ADMINISTRATOR, GRANT_MODEL, Harness, Service};
use lys_identity::{Change, IdentityError, IdentityId, LifecycleState, PersonId, verify_event};
use lys_identity_server::sign_in_providers::SignInProvidersSettings;
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

/// `ID001_DIRECTORY`: register a person and an agent, edit the display
/// profile, list and read both, and reopen: the enduring ids and the signed
/// history are the same bytes after the reopen as before it.
#[test]
fn a_person_and_an_agent_keep_their_ids_and_history_across_a_reopen() -> TestResult {
    let harness = Harness::new(3)?;
    let mut directory = harness.open()?;
    let (person, _) = directory.register_person(administrator()?, op(1), shown("Ada")?, 10)?;
    let (agent, receipt) =
        directory.register_agent(administrator()?, op(2), person, shown("Builder")?, 11)?;
    directory.change_profile(
        administrator()?,
        op(3),
        IdentityId::Agent(agent),
        shown("Builder two")?,
        12,
    )?;
    let live = directory.projection()?.clone();
    let history = (0..directory.log()?.len()?)
        .map(|index| directory.log()?.leaf(index))
        .collect::<Result<Vec<_>, _>>()?;
    drop(directory);

    let mut reopened = harness.open()?;
    assert_eq!(
        reopened.projection()?,
        &live,
        "the projection after reopen equals replay"
    );
    let listed = reopened
        .projection()?
        .records()
        .map(|(id, _)| *id)
        .collect::<Vec<_>>();
    assert_eq!(listed.len(), 2, "the list holds both identities");
    assert!(listed.contains(&IdentityId::Person(person)));
    assert!(listed.contains(&IdentityId::Agent(agent)));
    let read_person = reopened
        .record(IdentityId::Person(person))?
        .ok_or("person missing")?;
    assert_eq!(read_person.profile().display_name(), "Ada");
    assert_eq!(read_person.responsible(), None);
    let record = reopened
        .record(IdentityId::Agent(agent))?
        .ok_or("agent missing")?;
    assert_eq!(record.responsible(), Some(person));
    assert_eq!(record.profile().display_name(), "Builder two");
    assert_eq!(record.state(), LifecycleState::Registered);
    assert_eq!(record.events().len(), 2);
    assert_eq!(receipt.identity(), IdentityId::Agent(agent));
    let reread = (0..reopened.log()?.len()?)
        .map(|index| reopened.log()?.leaf(index))
        .collect::<Result<Vec<_>, _>>()?;
    assert_eq!(history.len(), 3, "three changes, three signed events");
    assert_eq!(reread, history, "history unchanged");
    Ok(())
}

/// The agent's record and its signed registration event both name the
/// person responsible for it.
#[test]
fn an_agents_record_and_its_signed_registration_name_its_responsible_person() -> TestResult {
    let harness = Harness::new(3)?;
    let mut directory = harness.open()?;
    let (person, _) = directory.register_person(administrator()?, op(1), shown("Ada")?, 10)?;
    let (agent, receipt) =
        directory.register_agent(administrator()?, op(2), person, shown("Builder")?, 11)?;
    let leaf = directory
        .log()?
        .leaf(receipt.coordinate().index)?
        .ok_or("leaf missing")?;
    let signed = verify_event(&leaf, &directory.service_key())?;
    assert_eq!(signed.event()?.identity(), IdentityId::Agent(agent));
    assert_eq!(signed.event()?.actor(), &administrator()?);
    let Change::RegisterAgent { responsible, .. } = signed.event()?.change() else {
        return Err("the agent's first event is not its registration".into());
    };
    assert_eq!(*responsible, person, "the signed event names them");
    let record = directory
        .record(IdentityId::Agent(agent))?
        .ok_or("agent missing")?;
    assert_eq!(record.responsible(), Some(person), "so does the record");
    Ok(())
}

#[test]
fn an_agent_needs_a_registered_responsible_person_and_nothing_is_recorded_otherwise() -> TestResult
{
    let harness = Harness::new(3)?;
    let mut directory = harness.open()?;
    let nobody = PersonId::from_bytes([9; 16]);
    let refused = directory.register_agent(administrator()?, op(1), nobody, shown("Orphan")?, 10);
    assert!(matches!(
        refused,
        Err(IdentityError::IdentityUnknown { .. })
    ));
    assert_eq!(directory.log()?.len()?, 0);
    Ok(())
}

#[test]
fn the_same_operation_answers_once_and_a_different_request_under_it_is_refused() -> TestResult {
    let harness = Harness::new(3)?;
    let mut directory = harness.open()?;
    let first = directory.register_person(administrator()?, op(1), shown("Ada")?, 10)?;
    let again = directory.register_person(administrator()?, op(1), shown("Ada")?, 99)?;
    assert_eq!(first, again);
    assert_eq!(directory.log()?.len()?, 1);
    let other = directory.register_person(administrator()?, op(1), shown("Grace")?, 10);
    assert!(matches!(other, Err(IdentityError::OperationReused { .. })));
    assert_eq!(directory.log()?.len()?, 1);
    Ok(())
}

fn login(subject: &str) -> Login {
    Login {
        subject: subject.to_owned(),
        email: "shared@example.test".to_owned(),
    }
}

fn text<'a>(body: &'a Value, field: &str) -> Result<&'a str, Box<dyn Error>> {
    body[field]
        .as_str()
        .ok_or_else(|| format!("no {field} in {body}").into())
}

/// A service speaking to the stand-in issuer API, with the administrator
/// signed in and bound to the person Ada, answering Ada's id.
async fn administered() -> Result<(Service, FakeRauthy, String, String), Box<dyn Error>> {
    let rauthy = FakeRauthy::start().await?;
    let settings = SignInProvidersSettings {
        api: rauthy.api().to_owned(),
        api_key_file: rauthy.api_key_file(),
    };
    let (service, ()) =
        Service::start_setting(GRANT_MODEL, None, None, Some(settings), |_| Ok(())).await?;
    let cookie = service.sign_in(login(ADMINISTRATOR)).await?;
    let (status, body) = service
        .post(
            "/people",
            Some(&cookie),
            &json!({ "operation": op(1).to_string(), "display_name": "Ada" }),
        )
        .await?;
    assert_eq!(status, 200, "{body}");
    let ada = text(&body, "person")?.to_owned();
    // Activate the administrator before binding its login: Registered people
    // may sign in, but cannot administer the directory.
    let (status, answer) = service
        .post(
            &format!("/identities/{ada}/transitions"),
            Some(&cookie),
            &json!({ "operation": op(250).to_string(), "transition": "activate" }),
        )
        .await?;
    assert_eq!(status, 200, "{answer}");
    let bind = json!({
        "operation": op(2).to_string(),
        "issuer": service.issuer.issuer(),
        "subject": ADMINISTRATOR,
    });
    let (status, body) = service
        .post(&format!("/people/{ada}/logins"), Some(&cookie), &bind)
        .await?;
    assert_eq!(status, 200, "{body}");
    Ok((service, rauthy, cookie, ada))
}

async fn identity_count(service: &Service, cookie: &str) -> Result<usize, Box<dyn Error>> {
    let (status, body) = service.get("/identities", Some(cookie)).await?;
    assert_eq!(status, 200, "{body}");
    Ok(body["identities"]
        .as_array()
        .ok_or("no identities list")?
        .len())
}

/// Every credential the service holds or has issued: certificates across
/// every agent, service accounts, and the administrator's live sessions.
async fn credential_count(service: &Service, cookie: &str) -> Result<usize, Box<dyn Error>> {
    let (status, body) = service.get("/identities", Some(cookie)).await?;
    assert_eq!(status, 200, "{body}");
    let mut count = 0;
    for identity in body["identities"].as_array().ok_or("no identities list")? {
        let id = text(identity, "id")?;
        if id.starts_with("agent-") {
            let (status, body) = service
                .get(&format!("/agents/{id}/certificates"), Some(cookie))
                .await?;
            assert_eq!(status, 200, "{body}");
            count += body["certificates"]
                .as_array()
                .ok_or("no certificates list")?
                .len();
        }
    }
    let (status, body) = service.get("/service-accounts", Some(cookie)).await?;
    assert_eq!(status, 200, "{body}");
    count += body["service_accounts"]
        .as_array()
        .ok_or("no service accounts list")?
        .len();
    let (status, body) = service.get("/sessions", Some(cookie)).await?;
    assert_eq!(status, 200, "{body}");
    count += body["sessions"].as_array().ok_or("no sessions list")?.len();
    Ok(count)
}

/// `ID001_DIRECTORY`: registering an agent creates no Rauthy user, sends the
/// issuer nothing at all, and issues no certificate, service account or
/// session. The request counter is first shown to fire on a call that does
/// reach the issuer, so an unchanged count is a count that could have moved.
#[tokio::test]
async fn registering_an_agent_creates_no_issuer_user_and_issues_no_credential() -> TestResult {
    let (service, rauthy, cookie, ada) = administered().await?;
    let quiet = rauthy.request_count();
    let (status, body) = service.get("/sign-in-providers", Some(&cookie)).await?;
    assert_eq!(status, 200, "{body}");
    assert!(
        rauthy.request_count() > quiet,
        "the counter fires on a call that reaches the issuer"
    );

    let users = rauthy.user_count()?;
    let requests = rauthy.request_count();
    let credentials = credential_count(&service, &cookie).await?;
    let (status, body) = service
        .post(
            "/agents",
            Some(&cookie),
            &json!({ "operation": op(3).to_string(), "display_name": "Builder" }),
        )
        .await?;
    assert_eq!(status, 200, "{body}");
    assert_eq!(text(&body, "responsible")?, ada);
    let agent = text(&body, "agent")?.to_owned();

    assert_eq!(rauthy.request_count(), requests, "issuer sent nothing");
    assert_eq!(rauthy.user_count()?, users, "no issuer user was created");
    assert_eq!(
        credential_count(&service, &cookie).await?,
        credentials,
        "no certificate, service account or session was issued"
    );
    let (status, body) = service
        .get(&format!("/identities/{agent}"), Some(&cookie))
        .await?;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["state"], "registered", "never shown running");
    assert_eq!(body["logins"], json!([]), "no login made");
    Ok(())
}

/// A registration without a signed-in caller is refused by name and
/// creates no record, and no request moves an agent to another responsible
/// person: the route takes none, a body naming one is refused, and every
/// change the administrator may make leaves it where registration put it.
#[tokio::test]
async fn a_registration_needs_a_caller_and_the_responsible_person_never_moves() -> TestResult {
    let (service, rauthy, cookie, ada) = administered().await?;
    let before = identity_count(&service, &cookie).await?;
    let unsigned = json!({ "operation": op(3).to_string(), "display_name": "Stray" });
    let (status, body) = service.post("/agents", None, &unsigned).await?;
    assert_eq!(status, 401, "{body}");
    assert_eq!(body["refusal"], "NotSignedIn");
    let (status, body) = service.post("/people", None, &unsigned).await?;
    assert_eq!(status, 401, "{body}");
    assert_eq!(body["refusal"], "NotSignedIn");
    assert_eq!(
        identity_count(&service, &cookie).await?,
        before,
        "a refused registration recorded nothing"
    );

    let (status, body) = service
        .post(
            "/people",
            Some(&cookie),
            &json!({ "operation": op(4).to_string(), "display_name": "Grace" }),
        )
        .await?;
    assert_eq!(status, 200, "{body}");
    let grace = text(&body, "person")?.to_owned();
    let (status, body) = service
        .post(
            "/agents",
            Some(&cookie),
            &json!({ "operation": op(5).to_string(), "display_name": "Builder" }),
        )
        .await?;
    assert_eq!(status, 200, "{body}");
    let agent = text(&body, "agent")?.to_owned();
    let after_agent = identity_count(&service, &cookie).await?;

    let naming = json!({
        "operation": op(6).to_string(),
        "display_name": "Builder",
        "responsible": grace,
    })
    .to_string();
    let refused = reqwest::Client::new()
        .post(format!("{}/agents", service.base))
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .header(reqwest::header::COOKIE, &cookie)
        .body(naming)
        .send()
        .await?;
    assert!(
        refused.status().is_client_error(),
        "a body naming a responsible person is refused: {}",
        refused.status()
    );
    assert_eq!(identity_count(&service, &cookie).await?, after_agent);

    let changes = [
        (
            format!("/identities/{agent}/profile"),
            json!({ "operation": op(7).to_string(), "display_name": "Builder two" }),
        ),
        (
            format!("/identities/{agent}/transitions"),
            json!({ "operation": op(8).to_string(), "transition": "activate" }),
        ),
    ];
    let mut applied = 0;
    for (path, body) in &changes {
        let (status, answer) = service.post(path, Some(&cookie), body).await?;
        assert_eq!(status, 200, "{path}: {answer}");
        let (_, read) = service
            .get(&format!("/identities/{agent}"), Some(&cookie))
            .await?;
        assert_eq!(read["responsible"], ada.as_str(), "after {path}: {read}");
        applied += 1;
    }
    assert_eq!(applied, changes.len());
    assert_eq!(rauthy.user_count()?, 0, "no request made an issuer user");
    Ok(())
}
