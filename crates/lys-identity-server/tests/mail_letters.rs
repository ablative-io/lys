#![cfg(test)]

//! Lys's letters go out through the install's mail settings and never
//! otherwise (DIRECTORY-110 R3, R4): the added letter to a person admitted
//! whose account the issuer already held, the access letter to a person a
//! root grant names, the founded letter to a founder; each asserted at a
//! capturing sink, each sent once per act, a failed send answered as
//! `mail_not_sent` with the act recorded, and no admit at all without mail
//! settings.

use std::error::Error;
use std::sync::Arc;

use identity_contract::fake_rauthy::FakeRauthy;
use identity_contract::harness::{GRANT_MODEL, Service, session_cookie};
use identity_contract::mail_sink::MailSink;
use lys_identity::OperationId;
use lys_identity::signer::load_service_key;
use lys_identity_server::configuration_store::ConfigurationStore;
use lys_identity_server::mail::{MailSettings, Mailer, Tls};
use lys_identity_server::mail_letters::Letter;
use lys_identity_server::routes::open_directory;
use lys_identity_server::runner_acts::ActStore;
use lys_identity_server::setup::SetupSettings;
use lys_identity_server::sign_in_providers::SignInProvidersSettings;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

type TestResult = Result<(), Box<dyn Error>>;

const CODE: &str = "mail-letters-first-run-code";
const ADMIN_EMAIL: &str = "ada@example.test";
const PASSWORD: &str = "Analytical-Engine-1843";
const BEA: &str = "bea@example.test";
const ALL_STEPS: [&str; 6] = ["register", "account", "bind", "activate", "grant", "letter"];

/// A service with a fake issuer, Ada as its first-run administrator and,
/// when `mail` is set, its letters pointed at a sink of its own.
async fn table(mail: bool) -> Result<(Service, FakeRauthy, String, MailSink), Box<dyn Error>> {
    let rauthy = FakeRauthy::start().await?;
    let sink = MailSink::start().await?;
    let port = sink.port();
    let settings = SignInProvidersSettings {
        api: rauthy.api().to_owned(),
        api_key_file: rauthy.api_key_file(),
    };
    let (service, ()) = Service::start_adjusted(
        GRANT_MODEL,
        None,
        None,
        Some(settings),
        move |config| {
            config.requests_dir = None;
            config.certificates_dir = None;
            config.network_file = None;
            config.roles_file = None;
            config.provisioning_file = None;
            config.runtime_dir = None;
            config.service_accounts_dir = None;
            config.teams_dir = None;
            config.stops_dir = None;
            config.budgets_dir = None;
            config.policies_dir = None;
            config.goals_dir = None;
            config.reviews_dir = None;
            config.administrator = None;
            config.setup = Some(SetupSettings {
                code_file: config.log_dir.with_file_name("setup-code"),
                administrator_file: config.log_dir.with_file_name("administrator.json"),
                email: None,
                organisation: "Acme".to_owned(),
            });
            if mail {
                config.mail = Some(MailSettings {
                    host: "127.0.0.1".to_owned(),
                    port,
                    user: None,
                    password_file: None,
                    from: "Lys <lys@example.test>".to_owned(),
                    tls: Tls::None,
                    sent_file: config.log_dir.with_file_name("letters-sent.jsonl"),
                });
            }
        },
        |config| {
            let setup = config.setup.as_ref().ok_or("setup is configured")?;
            let digest: Vec<String> = Sha256::digest(CODE.as_bytes())
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect();
            let pending = json!({ "purpose": "first-run", "sha256": digest.concat() });
            std::fs::write(&setup.code_file, pending.to_string())?;
            let key = Arc::new(load_service_key(&config.event_key_file)?);
            drop(open_directory(config)?);
            drop(ConfigurationStore::open(
                &config.log_dir.with_file_name("organisation"),
                Arc::clone(&key),
            )?);
            drop(ActStore::open(
                &config.log_dir.with_file_name("runner-acts"),
                Arc::clone(&key),
            )?);
            Ok(())
        },
    )
    .await?;
    rauthy.link(service.issuer.clone())?;
    let made = reqwest::Client::new()
        .post(format!("{}/setup/administrator", service.base))
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(
            json!({
                "code": CODE,
                "operation": OperationId::generate()?.to_string(),
                "display_name": "Ada Lovelace",
                "email": ADMIN_EMAIL,
                "password": PASSWORD,
            })
            .to_string(),
        )
        .send()
        .await?;
    let ada = session_cookie(made).await?;
    Ok((service, rauthy, ada, sink))
}

/// An account for `email` at the issuer, made before Lys admits them, so
/// the admit finds it and Lys's own letter is the added letter. The body is
/// the one `accounts::make` sends.
async fn account_held(rauthy: &FakeRauthy, email: &str, name: &str) -> TestResult {
    let key = std::fs::read_to_string(rauthy.api_key_file())?;
    let answer = reqwest::Client::new()
        .post(format!("{}/users", rauthy.api()))
        .header(
            reqwest::header::AUTHORIZATION,
            format!("API-Key {}", key.trim()),
        )
        .json(&json!({
            "email": email,
            "given_name": name,
            "family_name": "Example",
            "language": "en",
            "groups": null,
            "roles": [],
            "user_expires": null,
        }))
        .send()
        .await?;
    assert!(answer.status().is_success(), "{}", answer.status());
    Ok(())
}

/// The act admitting Bea, with a first grant under `relation` when one is named.
fn admit(operation: &str, relation: Option<&str>) -> Value {
    let mut body = json!({
        "operation": operation,
        "display_name": "Bea",
        "email": BEA,
    });
    if let Some(relation) = relation {
        body["grant"] = json!({
            "route": "browser",
            "resource": {"kind": "doc", "id": "alpha"},
            "relation": relation,
        });
    }
    body
}

/// A root grant to `holder` as alpha on doc `id`.
fn root(operation: &str, holder: &str, id: &str) -> Value {
    json!({
        "operation": operation,
        "route": "browser",
        "holder": holder,
        "resource": {"kind": "doc", "id": id},
        "relation": "alpha",
        "pass_on": {"kind": "use_only"},
        "window": {"starts_at": 0, "ends_at": null},
    })
}

/// The people the directory lists, as the administrator reads them.
async fn people(service: &Service, ada: &str) -> Result<Vec<Value>, Box<dyn Error>> {
    let (status, listed) = service.get("/directory/people", Some(ada)).await?;
    assert_eq!(status, 200, "{listed}");
    Ok(listed["people"]
        .as_array()
        .ok_or("the directory lists people")?
        .clone())
}

#[tokio::test]
async fn a_person_whose_account_the_issuer_held_is_written_to_by_lys_once() -> TestResult {
    let (service, rauthy, ada, mut sink) = table(true).await?;
    account_held(&rauthy, BEA, "Bea").await?;
    let operation = OperationId::generate()?.to_string();
    let body = admit(&operation, None);
    let (status, answer) = service.post("/people/admit", Some(&ada), &body).await?;
    assert_eq!(status, 200, "{answer}");
    assert_eq!(
        answer["receipt"]["completed"],
        json!(["register", "account", "bind", "activate", "letter"])
    );
    assert_eq!(answer["letter"]["letter"], "added");
    assert_eq!(answer["letter"]["to"], BEA);
    assert_eq!(answer["letter"]["sent"], true);
    assert_eq!(answer["letter"]["by"], "lys");
    let letter = sink.next().await.ok_or("the added letter")?;
    assert_eq!(letter.to, vec![BEA.to_owned()]);
    assert_eq!(letter.from, "lys@example.test");
    assert_eq!(letter.subject(), "You have been added to Lys");
    assert_eq!(
        letter.body,
        format!(
            "Hello Bea, you have been added to Lys. This sign-in opens your record and signs you in as Bea. Sign in at {} with {BEA}.",
            service.base
        )
    );
    assert!(
        letter.header("To").is_some_and(|to| to.contains("Bea")),
        "{:?}",
        letter.headers
    );
    // Asking again answers the same act and sends nothing twice.
    let (status, again) = service.post("/people/admit", Some(&ada), &body).await?;
    assert_eq!(status, 200, "{again}");
    assert_eq!(again["letter"]["sent"], true);
    assert_eq!(again["letter"]["by"], "lys");
    assert_eq!(again["person"], answer["person"]);
    assert!(sink.taken().is_none(), "a second letter went");
    Ok(())
}

#[tokio::test]
async fn a_person_whose_account_is_newly_made_hears_from_the_issuer_and_not_lys() -> TestResult {
    let (service, _rauthy, ada, mut sink) = table(true).await?;
    let operation = OperationId::generate()?.to_string();
    let body = admit(&operation, Some("alpha"));
    let (status, answer) = service.post("/people/admit", Some(&ada), &body).await?;
    assert_eq!(status, 200, "{answer}");
    assert_eq!(answer["receipt"]["completed"], json!(ALL_STEPS));
    assert_eq!(answer["letter"]["letter"], "added");
    assert_eq!(answer["letter"]["by"], "issuer");
    assert_eq!(answer["letter"]["sent"], true);
    assert_eq!(answer["letter"]["to"], BEA);
    assert!(
        sink.taken().is_none(),
        "Lys wrote beside the issuer's letter"
    );
    // The first grant inside the act sends no access letter of its own:
    // one letter per act.
    let (status, again) = service.post("/people/admit", Some(&ada), &body).await?;
    assert_eq!(status, 200, "{again}");
    assert_eq!(again["letter"]["by"], "issuer");
    assert!(sink.taken().is_none(), "a repeat wrote a letter");
    Ok(())
}

#[tokio::test]
async fn a_root_grant_to_a_person_writes_to_them_once_in_the_access_screens_words() -> TestResult {
    let (service, _rauthy, ada, mut sink) = table(true).await?;
    let (status, admitted) = service
        .post(
            "/people/admit",
            Some(&ada),
            &admit(&OperationId::generate()?.to_string(), None),
        )
        .await?;
    assert_eq!(status, 200, "{admitted}");
    let bea = admitted["person"].as_str().ok_or("a person")?;
    let operation = OperationId::generate()?.to_string();
    let body = root(&operation, bea, "beta");
    let (status, answer) = service.post("/grants/roots", Some(&ada), &body).await?;
    assert_eq!(status, 200, "{answer}");
    assert_eq!(answer["letter"]["letter"], "access_given");
    assert_eq!(answer["letter"]["sent"], true);
    assert_eq!(answer["letter"]["by"], "lys");
    assert_eq!(answer["letter"]["to"], BEA);
    let letter = sink.next().await.ok_or("the access letter")?;
    assert_eq!(letter.subject(), "You have been given access in Lys");
    assert_eq!(letter.to, vec![BEA.to_owned()]);
    assert_eq!(letter.from, "lys@example.test");
    assert_eq!(
        letter.body,
        format!(
            "Hello Bea, you have been given access in Lys: alpha on doc beta: Read this resource; Write to this resource. Sign in at {} to use it.",
            service.base
        )
    );
    let (status, again) = service.post("/grants/roots", Some(&ada), &body).await?;
    assert_eq!(status, 200, "{again}");
    assert_eq!(again["grant"], answer["grant"]);
    assert_eq!(again["letter"]["sent"], true);
    assert!(sink.taken().is_none(), "a second letter went");
    Ok(())
}

#[tokio::test]
async fn a_letter_the_server_refuses_leaves_the_act_recorded_and_goes_when_asked_again()
-> TestResult {
    let (service, rauthy, ada, mut sink) = table(true).await?;
    account_held(&rauthy, BEA, "Bea").await?;
    sink.refuse(true);
    let operation = OperationId::generate()?.to_string();
    let body = admit(&operation, Some("alpha"));
    let (status, refused) = service.post("/people/admit", Some(&ada), &body).await?;
    assert_eq!(status, 503, "{refused}");
    assert_eq!(refused["refusal"], "mail_not_sent");
    let reason = refused["reason"].as_str().ok_or("a reason")?;
    assert!(
        reason.contains("added letter to bea@example.test"),
        "{reason}"
    );
    assert!(reason.contains("554"), "{reason}");
    assert_eq!(refused["receipt"]["failed"], "letter");
    assert_eq!(refused["receipt"]["completed"], json!(ALL_STEPS[..5]));
    let person = refused["receipt"]["person"]
        .as_str()
        .ok_or("the person")?
        .to_owned();
    assert!(sink.taken().is_none());
    assert_eq!(
        people(&service, &ada).await?.len(),
        2,
        "the act is recorded"
    );
    // A root to the same person while the server refuses: recorded, and
    // refused by name.
    let root_operation = OperationId::generate()?.to_string();
    let root_body = root(&root_operation, &person, "beta");
    let (status, root_refused) = service
        .post("/grants/roots", Some(&ada), &root_body)
        .await?;
    assert_eq!(status, 503, "{root_refused}");
    assert_eq!(root_refused["refusal"], "mail_not_sent");
    assert!(
        root_refused["reason"]
            .as_str()
            .is_some_and(|reason| reason.contains("access_given letter")),
        "{root_refused}"
    );
    sink.refuse(false);
    let (status, again) = service.post("/people/admit", Some(&ada), &body).await?;
    assert_eq!(status, 200, "{again}");
    assert_eq!(again["person"], person);
    assert_eq!(again["receipt"]["completed"], json!(ALL_STEPS));
    assert_eq!(
        sink.next().await.ok_or("the added letter")?.subject(),
        "You have been added to Lys"
    );
    let (status, root_again) = service
        .post("/grants/roots", Some(&ada), &root_body)
        .await?;
    assert_eq!(status, 200, "{root_again}");
    assert_eq!(root_again["letter"]["sent"], true);
    assert_eq!(
        sink.next().await.ok_or("the access letter")?.subject(),
        "You have been given access in Lys"
    );
    assert!(sink.taken().is_none(), "a letter went twice");
    Ok(())
}

#[tokio::test]
async fn an_admit_refused_at_a_step_delivers_no_letter() -> TestResult {
    let (service, _rauthy, ada, mut sink) = table(true).await?;
    let (status, refused) = service
        .post(
            "/people/admit",
            Some(&ada),
            &admit(&OperationId::generate()?.to_string(), Some("gamma")),
        )
        .await?;
    assert_eq!(status, 403, "{refused}");
    assert_eq!(refused["receipt"]["failed"], "grant");
    assert!(sink.taken().is_none(), "a letter went for a refused act");
    Ok(())
}

#[tokio::test]
async fn without_mail_settings_admission_hands_the_link_and_a_root_says_no_letter_went()
-> TestResult {
    let (service, _rauthy, ada, mut sink) = table(false).await?;
    // DIRECTORY-112 R3: with no mail settings the person is admitted and
    // the link that signs them in once is handed in the answer.
    let (status, admitted) = service
        .post(
            "/people/admit",
            Some(&ada),
            &admit(&OperationId::generate()?.to_string(), None),
        )
        .await?;
    assert_eq!(status, 200, "{admitted}");
    assert_eq!(admitted["letter"]["letter"], "added", "{admitted}");
    assert_eq!(admitted["letter"]["sent"], false, "{admitted}");
    assert!(
        admitted["letter"]["handed"]
            .as_str()
            .is_some_and(|link| link.contains("/#/added/")),
        "{admitted}"
    );
    assert!(admitted["receipt"]["failed"].is_null(), "{admitted}");
    let listed = people(&service, &ada).await?;
    assert_eq!(listed.len(), 2, "{listed:?}");
    // The administrator's root on an existing person is still recorded;
    // the answer says no letter went.
    let ada_person = listed
        .iter()
        .find(|person| person["email"] == ADMIN_EMAIL || person["display_name"] == "Ada Lovelace")
        .and_then(|person| person["id"].as_str())
        .ok_or("Ada's id")?
        .to_owned();
    let (status, answer) = service
        .post(
            "/grants/roots",
            Some(&ada),
            &root(&OperationId::generate()?.to_string(), &ada_person, "alpha"),
        )
        .await?;
    assert_eq!(status, 200, "{answer}");
    assert!(answer["grant"].as_str().is_some(), "{answer}");
    assert_eq!(answer["letter"]["letter"], "access_given");
    assert_eq!(answer["letter"]["sent"], false);
    assert_eq!(answer["letter"]["reason"], "mail_unconfigured");
    assert!(sink.taken().is_none());
    Ok(())
}

#[tokio::test]
async fn the_founded_letter_carries_the_link_and_is_sent_once_per_operation() -> TestResult {
    let mut sink = MailSink::start().await?;
    let dir = tempfile::tempdir()?;
    let mailer = Mailer::open(&sink.settings(dir.path().join("letters-sent.jsonl")))?;
    let letter = Letter::Founded {
        name: "Ada Lovelace".to_owned(),
        email: ADMIN_EMAIL.to_owned(),
        link: "https://lys.example.test/found/abc".to_owned(),
        organisation: "Acme".to_owned(),
    };
    let view = mailer.send_once("op-1", &letter).await?;
    assert_eq!((view.sent, view.letter.as_str()), (true, "founded"));
    let taken = sink.next().await.ok_or("the founded letter")?;
    assert_eq!(taken.subject(), "Found Acme in Lys");
    assert_eq!(taken.to, vec![ADMIN_EMAIL.to_owned()]);
    assert_eq!(
        taken.body,
        "Hello Ada Lovelace, open https://lys.example.test/found/abc to prove this address and found Acme in Lys. The link is yours alone; if you did not ask for it, ignore this letter."
    );
    assert!(
        taken
            .header("To")
            .is_some_and(|to| to.contains("Ada Lovelace")),
        "{:?}",
        taken.headers
    );
    let again = mailer.send_once("op-1", &letter).await?;
    assert!(again.sent);
    assert!(sink.taken().is_none(), "sent twice");
    let record = std::fs::read_to_string(dir.path().join("letters-sent.jsonl"))?;
    assert_eq!(record.lines().count(), 1, "{record}");
    assert!(
        !record.contains("abc"),
        "the record carries the link: {record}"
    );
    let other = mailer.send_once("op-2", &letter).await?;
    assert!(other.sent);
    assert!(sink.next().await.is_some());
    Ok(())
}

/// DIRECTORY-111 R4: the already-yours letter, word for word, with no
/// name, since it says nothing the asker gave.
#[tokio::test]
async fn the_already_yours_letter_names_the_organisation_and_the_address_and_no_person()
-> TestResult {
    let mut sink = MailSink::start().await?;
    let dir = tempfile::tempdir()?;
    let mailer = Mailer::open(&sink.settings(dir.path().join("letters-sent.jsonl")))?;
    let letter = Letter::AlreadyYours {
        email: ADMIN_EMAIL.to_owned(),
        organisation: "Acme".to_owned(),
        address: "https://lys.example.test".to_owned(),
    };
    let view = mailer.send_once("op-1", &letter).await?;
    assert_eq!((view.sent, view.letter.as_str()), (true, "already_yours"));
    let taken = sink.next().await.ok_or("the already-yours letter")?;
    assert_eq!(taken.subject(), "This address already signs in to Lys");
    assert_eq!(taken.to, vec![ADMIN_EMAIL.to_owned()]);
    assert_eq!(
        taken.body,
        "Hello, someone asked to found Acme in Lys with this address. It already signs in to Lys at https://lys.example.test, so nothing was founded; sign in there. If this was not you, ignore this letter."
    );
    assert!(
        taken
            .header("To")
            .is_some_and(|to| !to.contains('<') || to.trim().starts_with('<')),
        "the letter carries a name it was never given: {:?}",
        taken.headers
    );
    let again = mailer.send_once("op-1", &letter).await?;
    assert!(again.sent);
    assert!(sink.taken().is_none(), "sent twice");
    Ok(())
}
