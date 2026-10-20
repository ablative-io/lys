#![cfg(test)]

//! The proving link activates everything and signs the founder in
//! (DIRECTORY-111 R2): the founder's account made at the issuer, the
//! binding, both activations, the administrator grant, the issuer's own
//! letter recorded as the added letter, a session by address proof and the
//! proof spent last; a failed step retried by the same proof; a proof of an
//! address admitted meanwhile refused.

#[path = "founding/table.rs"]
mod founding;

use std::error::Error;

use founding::{
    BEA, FOUND, TestResult, accounts_for, file_under, found, foundings, leaves, prove, table,
    token_in,
};
use identity_contract::harness::Service;
use identity_contract::mail_sink::MailSink;
use lys_identity::OperationId;
use serde_json::{Value, json};

const CY: &str = "cy@example.test";
const PROOF_REFUSED: &str =
    "This link is not one Lys issued, or it was already used. Found again from the sign-in page.";

/// Found as Bea and prove, answering the proof's status, body and cookie.
async fn found_and_prove(
    service: &Service,
    sink: &mut MailSink,
) -> Result<(u16, Value, Option<String>), Box<dyn Error>> {
    let operation = OperationId::generate()?.to_string();
    let (status, answer) = service.post(FOUND, None, &found(&operation)).await?;
    assert_eq!(status, 200, "{answer}");
    let letter = sink.next().await.ok_or("the founded letter")?;
    let token = token_in(&letter)?;
    prove(service, &token).await
}

#[tokio::test]
async fn the_proof_activates_everything_grants_the_founder_and_signs_them_in() -> TestResult {
    let (service, rauthy, ada, mut sink) = table(true).await?;
    let operation = OperationId::generate()?.to_string();
    let (status, answer) = service.post(FOUND, None, &found(&operation)).await?;
    assert_eq!(status, 200, "{answer}");
    let letter = sink.next().await.ok_or("the founded letter")?;
    let token = token_in(&letter)?;
    let before = leaves(&service).await?;
    let (status, proved, cookie) = prove(&service, &token).await?;
    assert_eq!(status, 200, "{proved}");
    let cookie = cookie.ok_or("a session cookie")?;
    assert_eq!(proved["authority"].as_str().map(str::is_empty), Some(false));
    assert_eq!(proved["signed_in"]["issuer"], service.issuer.issuer());
    let subject = proved["signed_in"]["subject"]
        .as_str()
        .ok_or("the subject")?
        .to_owned();
    let person = proved["person"].as_str().ok_or("the person")?.to_owned();
    let organisation = proved["organisation"]
        .as_str()
        .ok_or("the organisation")?
        .to_owned();
    // bind, activate the person, activate the organisation: three leaves.
    assert_eq!(leaves(&service).await?, before + 3);
    assert_eq!(accounts_for(&rauthy, BEA)?, 1);

    let (status, read) = service
        .get(&format!("/identities/{person}"), Some(&cookie))
        .await?;
    assert_eq!(status, 200, "{read}");
    assert_eq!(read["state"], "active", "{read}");
    assert!(read["logins"].to_string().contains(&subject), "{read}");
    let (status, read) = service
        .get(&format!("/identities/{organisation}"), Some(&cookie))
        .await?;
    assert_eq!(status, 200, "{read}");
    assert_eq!(read["state"], "active", "{read}");

    // The founder holds administrator on the organisation, as 9846f30c's
    // founder-grant test reads it.
    let (status, grants) = service.get("/grants", Some(&cookie)).await?;
    assert_eq!(status, 200, "{grants}");
    let founding = grants["grants"]
        .as_array()
        .ok_or("the grants are listed")?
        .iter()
        .find(|grant| {
            grant["resource"]["kind"] == "organisation" && grant["relation"] == "administrator"
        })
        .ok_or("the founder's administrator grant is listed")?;
    assert_eq!(founding["resource"]["id"], organisation, "{founding}");

    let (status, account) = service.get("/me/account", Some(&cookie)).await?;
    assert_eq!(status, 200, "{account}");
    assert_eq!(account["email"], BEA);
    assert_eq!(account["password"], false, "{account}");

    // ACCESS-009's partition: the founder sees their own organisation's
    // identities and the configured administrator does not see them.
    let (status, own) = service.get("/identities", Some(&cookie)).await?;
    assert_eq!(status, 200, "{own}");
    let ids: Vec<&str> = own["identities"]
        .as_array()
        .ok_or("identities")?
        .iter()
        .filter_map(|identity| identity["id"].as_str())
        .collect();
    assert!(ids.contains(&person.as_str()), "{own}");
    assert!(
        !ids.iter()
            .any(|id| id.starts_with("person-") && *id != person),
        "{own}"
    );
    let (status, theirs) = service.get("/identities", Some(&ada)).await?;
    assert_eq!(status, 200, "{theirs}");
    assert!(!theirs.to_string().contains(&person), "{theirs}");

    // The spend is recorded; the same proof and any other are refused alike.
    let stored = foundings(&service)?.ok_or("the foundings file")?;
    let spent: Vec<bool> = stored["foundings"]
        .as_object()
        .ok_or("foundings")?
        .values()
        .filter_map(|founding| founding["spent"].as_bool())
        .collect();
    assert_eq!(spent, [true], "{stored}");
    let (status, refused, cookie) = prove(&service, &token).await?;
    assert_eq!(status, 403, "{refused}");
    assert_eq!(refused["refusal"], "proof_refused");
    assert!(cookie.is_none());
    let other = {
        use base64::Engine;
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode([9_u8; 32])
    };
    let (status, unknown, _) = prove(&service, &other).await?;
    assert_eq!(status, 403, "{unknown}");
    assert_eq!(unknown["refusal"], "proof_refused");
    assert_eq!(unknown["reason"], refused["reason"], "one text for both");
    assert_eq!(refused["reason"], format!("proof_refused: {PROOF_REFUSED}"));
    assert!(!refused.to_string().contains(&person), "{refused}");
    assert!(!unknown.to_string().contains("person-"), "{unknown}");
    // The letters-sent record holds the founded letter by Lys and the
    // added letter by the issuer, both under the founding operation.
    let record = std::fs::read_to_string(
        file_under(service.dir.path(), "letters-sent.jsonl").ok_or("the letters-sent record")?,
    )?;
    let lines: Vec<Value> = record
        .lines()
        .map(serde_json::from_str)
        .collect::<Result<_, _>>()?;
    assert_eq!(lines.len(), 2, "{record}");
    assert_eq!(lines[1]["letter"], "added");
    assert_eq!(lines[1]["by"], "issuer");
    assert_eq!(lines[1]["operation"], operation);
    assert!(sink.taken().is_none(), "Lys wrote a letter at the proof");
    Ok(())
}

#[tokio::test]
async fn a_failed_proving_step_leaves_the_proof_unspent_and_the_same_proof_finishes_it()
-> TestResult {
    let (service, rauthy, _ada, mut sink) = table(true).await?;
    let (status, answer) = service
        .post(FOUND, None, &found(&OperationId::generate()?.to_string()))
        .await?;
    assert_eq!(status, 200, "{answer}");
    let token = token_in(&sink.next().await.ok_or("the founded letter")?)?;
    let before = leaves(&service).await?;
    rauthy.refuse_create_once();
    let (status, refused, cookie) = prove(&service, &token).await?;
    assert_eq!(status, 400, "{refused}");
    assert_eq!(refused["refusal"], "AccountRefused", "{refused}");
    assert_eq!(refused["receipt"]["completed"], json!([]), "{refused}");
    assert_eq!(refused["receipt"]["failed"], "account", "{refused}");
    assert!(cookie.is_none());
    assert_eq!(leaves(&service).await?, before);
    assert_eq!(accounts_for(&rauthy, BEA)?, 0);
    let stored = foundings(&service)?.ok_or("the foundings file")?;
    assert!(stored.to_string().contains("\"spent\":false"), "{stored}");

    let (status, proved, cookie) = prove(&service, &token).await?;
    assert_eq!(status, 200, "{proved}");
    assert!(cookie.is_some());
    assert_eq!(leaves(&service).await?, before + 3);
    assert_eq!(accounts_for(&rauthy, BEA)?, 1);
    Ok(())
}

#[tokio::test]
async fn proving_after_the_address_was_admitted_in_between_is_refused_already_admitted()
-> TestResult {
    let (service, rauthy, ada, mut sink) = table(true).await?;
    let mut body = found(&OperationId::generate()?.to_string());
    body["email"] = json!(CY);
    body["display_name"] = json!("Cy");
    let (status, answer) = service.post(FOUND, None, &body).await?;
    assert_eq!(status, 200, "{answer}");
    let token = token_in(&sink.next().await.ok_or("the founded letter")?)?;
    // Ada admits Cy into Acme meanwhile.
    let (status, admitted) = service
        .post(
            "/people/admit",
            Some(&ada),
            &json!({
                "operation": OperationId::generate()?.to_string(),
                "display_name": "Cy",
                "email": CY,
            }),
        )
        .await?;
    assert_eq!(status, 200, "{admitted}");
    let _added = sink.taken();
    let before = leaves(&service).await?;
    let (status, refused, cookie) = prove(&service, &token).await?;
    assert_eq!(status, 403, "{refused}");
    assert_eq!(refused["refusal"], "already_admitted", "{refused}");
    assert_eq!(refused["receipt"]["completed"], json!([]), "{refused}");
    assert!(cookie.is_none());
    assert!(!refused.to_string().contains("person-"), "{refused}");
    assert_eq!(leaves(&service).await?, before);
    assert_eq!(accounts_for(&rauthy, CY)?, 1);
    let stored = foundings(&service)?.ok_or("the foundings file")?;
    assert!(stored.to_string().contains("\"spent\":false"), "{stored}");
    assert!(sink.taken().is_none());
    Ok(())
}

#[tokio::test]
async fn the_proving_page_route_is_reached_by_the_link_the_letter_carries() -> TestResult {
    let (service, _rauthy, _ada, mut sink) = table(true).await?;
    let (status, _proved, cookie) = found_and_prove(&service, &mut sink).await?;
    assert_eq!(status, 200);
    let (status, me) = service.get("/me", cookie.as_deref()).await?;
    assert_eq!(status, 200, "{me}");
    assert_eq!(me["person"]["display_name"], "Bea Baker", "{me}");
    Ok(())
}
