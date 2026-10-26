#![cfg(test)]

//! Founding and admission on an install with no mail settings (DIRECTORY-112):
//! the link a letter would have carried is handed in the answer, the proof
//! signs the person in as begun by a handed link, an admitted person's
//! handed link signs them in once, and nothing is refused `mail_unconfigured`.
//! With mail settings, the letter carries the link and no answer hands it.

#[path = "founding/table.rs"]
mod founding;

use std::error::Error;

use founding::{
    ADMIN_EMAIL, BEA, FOUND, TestResult, accounts_for, file_under, found, foundings, leaves, prove,
    table, token_in,
};
use identity_contract::harness::Service;
use lys_identity::OperationId;
use serde_json::{Value, json};

/// The token after `path` in a handed link.
fn token_after(link: &str, path: &str) -> Result<String, Box<dyn Error>> {
    let (_, after) = link
        .split_once(path)
        .ok_or_else(|| format!("the link carries {path}: {link}"))?;
    Ok(after.to_owned())
}

/// The handed link in an answer's letter view.
fn handed(answer: &Value) -> Result<String, Box<dyn Error>> {
    assert_eq!(answer["letter"]["sent"], false, "{answer}");
    assert_eq!(answer["letter"]["by"], "lys", "{answer}");
    assert!(
        answer["letter"]["reason"]
            .as_str()
            .is_some_and(|reason| reason.contains("no mail settings")),
        "{answer}"
    );
    Ok(answer["letter"]["handed"]
        .as_str()
        .ok_or("the link is handed in the answer")?
        .to_owned())
}

/// The stored entry under `digest`-less lookup: the one entry whose
/// `handed` and `kind` the test reads.
fn entries(service: &Service) -> Result<Vec<Value>, Box<dyn Error>> {
    let stored = foundings(service)?.ok_or("the foundings file")?;
    Ok(stored["foundings"]
        .as_object()
        .ok_or("foundings")?
        .values()
        .cloned()
        .collect())
}

/// Whether the session store holds a session begun by a handed link.
fn a_session_by_handed_link(service: &Service) -> Result<bool, Box<dyn Error>> {
    let path = file_under(service.dir.path(), "sessions.json").ok_or("the sessions file")?;
    let stored: Value = serde_json::from_str(&std::fs::read_to_string(path)?)?;
    Ok(stored["sessions"]
        .as_array()
        .ok_or("sessions")?
        .iter()
        .any(|session| session["actor"]["method"] == "handed_link"))
}

/// Ada admits Bea, answering the answer.
async fn admit_bea(service: &Service, ada: &str) -> Result<Value, Box<dyn Error>> {
    let (status, admitted) = service
        .post(
            "/people/admit",
            Some(ada),
            &json!({
                "operation": OperationId::generate()?.to_string(),
                "display_name": "Bea",
                "email": BEA,
            }),
        )
        .await?;
    assert_eq!(status, 200, "{admitted}");
    Ok(admitted)
}

#[tokio::test]
async fn without_mail_the_founding_hands_the_link_and_the_link_signs_the_founder_in() -> TestResult
{
    let (service, rauthy, _ada, mut sink) = table(false).await?;
    let before = leaves(&service).await?;
    let (status, answer) = service
        .post(FOUND, None, &found(&OperationId::generate()?.to_string()))
        .await?;
    assert_eq!(status, 200, "{answer}");
    assert_eq!(answer["letter"]["letter"], "founded", "{answer}");
    assert_eq!(answer["letter"]["to"], BEA, "{answer}");
    let link = handed(&answer)?;
    let token = token_after(&link, "/#/found/")?;
    assert!(!answer.to_string().contains("person-"), "{answer}");
    assert!(
        leaves(&service).await? > before,
        "the founding was recorded"
    );
    let stored = entries(&service)?;
    assert_eq!(stored.len(), 1, "{stored:?}");
    assert_eq!(stored[0]["handed"], true, "{stored:?}");
    assert_eq!(stored[0]["spent"], false, "{stored:?}");
    assert!(
        !foundings(&service)?
            .ok_or("the file")?
            .to_string()
            .contains(&token),
        "the token is stored"
    );
    assert!(
        sink.taken().is_none(),
        "a letter went with no mail settings"
    );
    assert!(
        file_under(service.dir.path(), "letters-sent.jsonl").is_none(),
        "a letters-sent record was made with no mail settings"
    );
    assert_eq!(accounts_for(&rauthy, BEA)?, 0);

    let (status, proved, cookie) = prove(&service, &token).await?;
    assert_eq!(status, 200, "{proved}");
    let cookie = cookie.ok_or("a session cookie")?;
    assert_eq!(proved["signed_in"]["issuer"], service.issuer.issuer());
    // A founding goes on to nowhere but Lys (DIRECTORY-113 R2).
    assert!(proved.get("continue_to").is_none(), "{proved}");
    let person = proved["person"].as_str().ok_or("the person")?.to_owned();
    let (status, me) = service.get("/me", Some(&cookie)).await?;
    assert_eq!(status, 200, "{me}");
    assert_eq!(me["person"]["id"], person, "{me}");
    assert_eq!(me["person"]["state"], "active", "{me}");
    assert_eq!(me["organisation"]["display_name"], "Bea's Bakery", "{me}");
    assert!(
        a_session_by_handed_link(&service)?,
        "the session says it was begun by a handed link"
    );
    assert_eq!(accounts_for(&rauthy, BEA)?, 1);
    let (status, refused, cookie) = prove(&service, &token).await?;
    assert_eq!(status, 403, "{refused}");
    assert_eq!(refused["refusal"], "proof_refused");
    assert!(cookie.is_none());
    assert!(sink.taken().is_none(), "a letter went at the proof");
    Ok(())
}

#[tokio::test]
async fn without_mail_a_second_founding_for_the_address_hands_a_fresh_link_and_the_first_is_refused()
-> TestResult {
    let (service, _rauthy, _ada, mut sink) = table(false).await?;
    let (status, first) = service
        .post(FOUND, None, &found(&OperationId::generate()?.to_string()))
        .await?;
    assert_eq!(status, 200, "{first}");
    let after_first = leaves(&service).await?;
    let (status, second) = service
        .post(FOUND, None, &found(&OperationId::generate()?.to_string()))
        .await?;
    assert_eq!(status, 200, "{second}");
    assert_eq!(
        leaves(&service).await?,
        after_first,
        "a second founding was recorded"
    );
    let first_link = handed(&first)?;
    let second_link = handed(&second)?;
    assert_ne!(first_link, second_link);
    assert_eq!(entries(&service)?.len(), 1);
    let (status, refused, _) = prove(&service, &token_after(&first_link, "/#/found/")?).await?;
    assert_eq!(status, 403, "{refused}");
    assert_eq!(refused["refusal"], "proof_refused");
    let (status, proved, cookie) =
        prove(&service, &token_after(&second_link, "/#/found/")?).await?;
    assert_eq!(status, 200, "{proved}");
    assert!(cookie.is_some());
    assert!(sink.taken().is_none());
    Ok(())
}

#[tokio::test]
async fn without_mail_an_address_that_signs_in_is_told_on_the_screen_and_nothing_is_founded()
-> TestResult {
    let (service, _rauthy, _ada, mut sink) = table(false).await?;
    let before = leaves(&service).await?;
    let mut body = found(&OperationId::generate()?.to_string());
    body["email"] = json!(ADMIN_EMAIL);
    let (status, answer) = service.post(FOUND, None, &body).await?;
    assert_eq!(status, 200, "{answer}");
    assert_eq!(answer["letter"]["letter"], "already_yours", "{answer}");
    assert_eq!(answer["letter"]["sent"], false, "{answer}");
    assert_eq!(answer["letter"]["to"], ADMIN_EMAIL, "{answer}");
    assert!(answer["letter"]["handed"].is_null(), "{answer}");
    assert!(!answer.to_string().contains("person-"), "{answer}");
    assert_eq!(leaves(&service).await?, before);
    assert!(foundings(&service)?.is_none(), "a foundings file was made");
    assert!(sink.taken().is_none());
    Ok(())
}

#[tokio::test]
async fn without_mail_an_admitted_person_is_handed_a_link_that_signs_them_in_once() -> TestResult {
    let (service, rauthy, ada, mut sink) = table(false).await?;
    let admitted = admit_bea(&service, &ada).await?;
    assert_eq!(
        admitted["receipt"]["completed"],
        json!(["register", "account", "bind", "activate", "letter"]),
        "{admitted}"
    );
    assert_eq!(admitted["letter"]["letter"], "added", "{admitted}");
    assert_eq!(admitted["letter"]["to"], BEA, "{admitted}");
    let link = handed(&admitted)?;
    let token = token_after(&link, "/#/added/")?;
    let person = admitted["person"].as_str().ok_or("the person")?.to_owned();
    let stored = entries(&service)?;
    assert_eq!(stored.len(), 1, "{stored:?}");
    assert_eq!(stored[0]["kind"], "admission", "{stored:?}");
    assert_eq!(stored[0]["handed"], true, "{stored:?}");
    assert_eq!(stored[0]["founder"], person, "{stored:?}");
    assert!(
        sink.taken().is_none(),
        "a letter went with no mail settings"
    );
    assert_eq!(accounts_for(&rauthy, BEA)?, 1);
    let (status, ada_me) = service.get("/me", Some(&ada)).await?;
    assert_eq!(status, 200, "{ada_me}");
    let organisation = ada_me["organisation"]["id"].clone();

    let (status, proved, cookie) = prove(&service, &token).await?;
    assert_eq!(status, 200, "{proved}");
    let cookie = cookie.ok_or("a session cookie")?;
    assert_eq!(proved["person"], person, "{proved}");
    assert_eq!(proved["organisation"], organisation, "{proved}");
    // Admitted from a Lys session, the link goes on to nowhere but Lys.
    assert!(proved.get("continue_to").is_none(), "{proved}");
    let (status, me) = service.get("/me", Some(&cookie)).await?;
    assert_eq!(status, 200, "{me}");
    assert_eq!(me["person"]["id"], person, "{me}");
    assert_eq!(me["person"]["state"], "active", "{me}");
    assert_eq!(me["person"]["display_name"], "Bea", "{me}");
    let (status, account) = service.get("/me/account", Some(&cookie)).await?;
    assert_eq!(status, 200, "{account}");
    assert_eq!(account["email"], BEA, "{account}");
    assert_eq!(account["password"], false, "{account}");
    assert!(a_session_by_handed_link(&service)?);
    assert_eq!(entries(&service)?[0]["spent"], true);
    let (status, refused, cookie) = prove(&service, &token).await?;
    assert_eq!(status, 403, "{refused}");
    assert_eq!(refused["refusal"], "proof_refused");
    assert!(cookie.is_none());
    Ok(())
}

#[tokio::test]
async fn without_mail_a_retired_persons_handed_link_is_refused_and_nothing_recorded() -> TestResult
{
    let (service, _rauthy, ada, mut sink) = table(false).await?;
    let admitted = admit_bea(&service, &ada).await?;
    let token = token_after(&handed(&admitted)?, "/#/added/")?;
    let person = admitted["person"].as_str().ok_or("the person")?.to_owned();
    let (status, retired) = service
        .post(
            &format!("/identities/{person}/transitions"),
            Some(&ada),
            &json!({
                "operation": OperationId::generate()?.to_string(),
                "transition": "retire",
                "reason": "left before the link was used",
            }),
        )
        .await?;
    assert_eq!(status, 200, "{retired}");
    let before = leaves(&service).await?;
    let (status, refused, cookie) = prove(&service, &token).await?;
    assert_eq!(status, 403, "{refused}");
    assert_eq!(refused["refusal"], "proof_refused", "{refused}");
    assert!(cookie.is_none());
    assert!(!refused.to_string().contains("person-"), "{refused}");
    assert_eq!(leaves(&service).await?, before);
    assert_eq!(entries(&service)?[0]["spent"], false);
    assert!(sink.taken().is_none());
    Ok(())
}

#[tokio::test]
async fn with_mail_the_letter_carries_the_link_and_no_answer_hands_it() -> TestResult {
    let (service, _rauthy, _ada, mut sink) = table(true).await?;
    let (status, answer) = service
        .post(FOUND, None, &found(&OperationId::generate()?.to_string()))
        .await?;
    assert_eq!(status, 200, "{answer}");
    assert_eq!(answer["letter"]["sent"], true, "{answer}");
    assert!(answer["letter"]["handed"].is_null(), "{answer}");
    assert_eq!(entries(&service)?[0]["handed"], false);
    let letter = sink.next().await.ok_or("the founded letter")?;
    let (status, proved, cookie) = prove(&service, &token_in(&letter)?).await?;
    assert_eq!(status, 200, "{proved}");
    assert!(cookie.is_some());
    assert!(
        !a_session_by_handed_link(&service)?,
        "a sent link is never a handed one"
    );
    Ok(())
}

#[tokio::test]
async fn an_administrator_withdraws_a_link_by_digest_and_the_persons_events_name_the_actor()
-> TestResult {
    let (service, _rauthy, ada, _sink) = table(false).await?;
    let admitted = admit_bea(&service, &ada).await?;
    let person = admitted["person"].as_str().ok_or("the person")?;
    assert_eq!(admitted["display_name"], "Bea");
    assert_eq!(admitted["letter"]["display_name"], "Bea");
    let token = token_after(&handed(&admitted)?, "/#/added/")?;
    let digest = lys_identity_server::foundings_store::digest(&token);
    let path = format!("/people/{person}/admission");
    let (status, waiting) = service.get(&path, Some(&ada)).await?;
    assert_eq!(status, 200, "{waiting}");
    assert_eq!(waiting["waiting"], true);
    let (status, answer) = service
        .post(&format!("{path}/withdraw"), Some(&ada), &json!({}))
        .await?;
    assert_eq!(status, 200, "{answer}");
    assert_eq!(answer["person"], person);
    assert_eq!(answer["digest"], digest);
    let (status, record) = service
        .get(&format!("/identities/{person}"), Some(&ada))
        .await?;
    assert_eq!(status, 200, "{record}");
    let index = record["events"]
        .as_array()
        .and_then(|events| events.last())
        .ok_or("the withdrawal event")?;
    let (status, event) = service
        .get(&format!("/receipts/{index}"), Some(&ada))
        .await?;
    assert_eq!(status, 200, "{event}");
    assert_eq!(event["receipt"]["change_kind"], 13);
    let (status, key) = service.get("/service-key", None).await?;
    assert_eq!(status, 200, "{key}");
    let key: [u8; 32] = unhex(key["ed25519"].as_str().ok_or("the public key")?)?
        .try_into()
        .map_err(|bytes: Vec<u8>| format!("a 32-byte key, not {} bytes", bytes.len()))?;
    let signed = lys_identity::verify_event(
        &unhex(event["message"].as_str().ok_or("the signed event")?)?,
        &key,
    )?;
    assert_eq!(
        signed.event()?.change(),
        &lys_identity::Change::AdmissionWithdrawn {
            digest: digest.clone()
        }
    );
    let (_, me) = service.get("/me", Some(&ada)).await?;
    assert_eq!(
        event["receipt"]["actor"]["subject"],
        me["signed_in"]["subject"]
    );
    assert!(
        entries(&service)?
            .iter()
            .any(|entry| entry["withdrawn"] == true)
    );
    let (status, refused, cookie) = prove(&service, &token).await?;
    assert_eq!(status, 403, "{refused}");
    assert_eq!(refused["refusal"], "proof_refused");
    assert!(cookie.is_none());
    let (status, refused) = service
        .post(&format!("{path}/withdraw"), Some(&ada), &json!({}))
        .await?;
    assert_eq!(status, 404, "{refused}");
    assert_eq!(refused["refusal"], "NoUnspentAdmission");
    let (_, waiting) = service.get(&path, Some(&ada)).await?;
    assert_eq!(waiting["waiting"], false);
    let later = admit_bea(&service, &ada).await?;
    let (status, proved, cookie) =
        prove(&service, &token_after(&handed(&later)?, "/#/added/")?).await?;
    assert_eq!(status, 200, "{proved}");
    assert!(cookie.is_some());
    assert_eq!(proved["person"], person);
    let (status, refused) = service
        .post(&format!("{path}/withdraw"), Some(&ada), &json!({}))
        .await?;
    assert_eq!(status, 404, "{refused}");
    assert_eq!(refused["refusal"], "NoUnspentAdmission");
    Ok(())
}

#[tokio::test]
async fn a_non_administrator_cannot_read_or_withdraw_an_admission() -> TestResult {
    let (service, _rauthy, ada, _sink) = table(false).await?;
    let admitted = admit_bea(&service, &ada).await?;
    let (_, _, bea) = prove(&service, &token_after(&handed(&admitted)?, "/#/added/")?).await?;
    let bea = bea.ok_or("Bea's session")?;
    let later = admit_bea(&service, &ada).await?;
    let person = admitted["person"].as_str().ok_or("the person")?;
    let before = foundings(&service)?;
    let path = format!("/people/{person}/admission");
    let (status, refused) = service
        .post(&format!("{path}/withdraw"), Some(&bea), &json!({}))
        .await?;
    assert_eq!(status, 403, "{refused}");
    assert_eq!(refused["refusal"], "NotAdmitted");
    let (status, refused) = service.get(&path, Some(&bea)).await?;
    assert_eq!(status, 403, "{refused}");
    assert_eq!(refused["refusal"], "NotAdmitted");
    assert_eq!(foundings(&service)?, before);
    let (status, proved, _) = prove(&service, &token_after(&handed(&later)?, "/#/added/")?).await?;
    assert_eq!(status, 200, "{proved}");
    Ok(())
}

#[tokio::test]
async fn a_reused_persons_mailed_admission_names_the_record_it_opens() -> TestResult {
    let (service, _rauthy, ada, mut sink) = table(true).await?;
    let first = admit_bea(&service, &ada).await?;
    // The first account's letter belongs to the issuer; Lys writes on reuse.
    let (status, reused) = service
        .post(
            "/people/admit",
            Some(&ada),
            &json!({
                "operation": OperationId::generate()?.to_string(),
                "person": first["person"], "display_name": "Annabel", "email": BEA,
            }),
        )
        .await?;
    assert_eq!(status, 200, "{reused}");
    let letter = sink.next().await.ok_or("the added letter")?;
    assert!(
        letter.body.contains("signs you in as Bea"),
        "{}",
        letter.body
    );
    assert!(!letter.body.contains("Annabel"), "{}", letter.body);
    Ok(())
}

fn unhex(value: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    (0..value.len())
        .step_by(2)
        .map(|at| {
            Ok(u8::from_str_radix(
                value.get(at..at + 2).ok_or("a hex byte")?,
                16,
            )?)
        })
        .collect()
}

#[tokio::test]
async fn an_administrator_of_another_organisation_cannot_withdraw_the_link() -> TestResult {
    let (service, _rauthy, ada, _sink) = table(false).await?;
    let (status, founded) = service
        .post(FOUND, None, &found(&OperationId::generate()?.to_string()))
        .await?;
    assert_eq!(status, 200, "{founded}");
    let (_, proved, bea) = prove(&service, &token_after(&handed(&founded)?, "/#/found/")?).await?;
    let bea = bea.ok_or("the other administrator")?;
    let (status, admitted) = service.post("/people/admit", Some(&bea), &json!({
        "operation": OperationId::generate()?.to_string(), "display_name": "Cy", "email": "cy@example.test",
    })).await?;
    assert_eq!(status, 200, "{admitted}; {proved}");
    let person = admitted["person"].as_str().ok_or("Cy")?;
    let before = foundings(&service)?;
    let (status, refused) = service
        .post(
            &format!("/people/{person}/admission/withdraw"),
            Some(&ada),
            &json!({}),
        )
        .await?;
    assert_eq!(status, 403, "{refused}");
    assert_eq!(refused["refusal"], "NotAdmitted");
    assert_eq!(foundings(&service)?, before);
    Ok(())
}
