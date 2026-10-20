#![cfg(test)]

//! A stranger founds an organisation (DIRECTORY-111): POST
//! /organisations/found records the organisation and its founder and
//! writes one letter with a proving link; POST /organisations/found/prove
//! makes the founder's account, binds it, activates both, issues the
//! administrator grant and signs the founder in, spending the proof last.
//! One answer shape to everyone at the founding; the difference is in the
//! letter. Every act is idempotent by operation, and a failed proving step
//! is retried by the same proof.

#[path = "founding/table.rs"]
mod founding;

use founding::{
    ADMIN_EMAIL, BEA, FOUND, TestResult, accounts_for, found, foundings, leaves, prove, table,
    token_in,
};
use lys_identity::OperationId;
use serde_json::json;

#[tokio::test]
async fn a_founding_records_both_registered_and_writes_one_letter_with_the_link() -> TestResult {
    let (service, rauthy, _ada, mut sink) = table(true).await?;
    let before = leaves(&service).await?;
    let operation = OperationId::generate()?.to_string();
    let (status, answer) = service.post(FOUND, None, &found(&operation)).await?;
    assert_eq!(status, 200, "{answer}");
    assert_eq!(answer["operation"], operation);
    assert_eq!(answer["letter"]["sent"], true);
    assert_eq!(answer["letter"]["to"], BEA);
    assert_eq!(answer["letter"]["letter"], "founded");
    let members: Vec<&str> = answer
        .as_object()
        .ok_or("an object")?
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(
        members,
        ["letter", "operation"],
        "no identity is named: {answer}"
    );
    let letter = sink.next().await.ok_or("the founded letter")?;
    assert_eq!(letter.subject(), "Found Bea's Bakery in Lys");
    assert_eq!(letter.to, vec![BEA.to_owned()]);
    let token = token_in(&letter)?;
    assert_eq!(letter.body.matches("/#/found/").count(), 1);
    assert!(
        letter
            .body
            .contains(&format!("{}/#/found/{token}", service.base)),
        "the link is under the public origin: {}",
        letter.body
    );
    // The organisation and the founder, both Registered, under the founding actor.
    assert_eq!(leaves(&service).await?, before + 2);
    let (status, founding_leaf) = service.get(&format!("/receipts/{before}"), None).await?;
    assert_eq!(status, 200, "{founding_leaf}");
    assert_eq!(
        founding_leaf["receipt"]["actor"]["subject"],
        format!("founding:{operation}"),
        "{founding_leaf}"
    );
    // The store holds the digest and never the token.
    let stored = foundings(&service)?.ok_or("the foundings file")?;
    let digest = lys_identity_server::foundings_store::digest(&token);
    assert!(stored["foundings"][&digest].is_object(), "{stored}");
    assert_eq!(stored["foundings"][&digest]["email"], BEA);
    assert_eq!(stored["foundings"][&digest]["spent"], false);
    assert!(!stored.to_string().contains(&token), "the token is stored");
    // The issuer holds no account until the proof.
    assert_eq!(accounts_for(&rauthy, BEA)?, 0);
    assert!(sink.taken().is_none(), "a second letter went");
    Ok(())
}

#[tokio::test]
async fn the_same_operation_answers_the_same_and_sends_once() -> TestResult {
    let (service, _rauthy, _ada, mut sink) = table(true).await?;
    let before = leaves(&service).await?;
    let operation = OperationId::generate()?.to_string();
    let (status, first) = service.post(FOUND, None, &found(&operation)).await?;
    assert_eq!(status, 200, "{first}");
    let letter = sink.next().await.ok_or("the founded letter")?;
    let (status, again) = service.post(FOUND, None, &found(&operation)).await?;
    assert_eq!(status, 200, "{again}");
    assert_eq!(first.to_string(), again.to_string());
    assert!(sink.taken().is_none(), "a second letter went");
    assert_eq!(leaves(&service).await?, before + 2);
    // The link in the letter still proves: nothing was minted again.
    let (status, proved, cookie) = prove(&service, &token_in(&letter)?).await?;
    assert_eq!(status, 200, "{proved}");
    assert!(cookie.is_some());
    Ok(())
}

#[tokio::test]
async fn a_second_founding_from_the_address_resends_a_fresh_link_and_founds_nothing() -> TestResult
{
    let (service, _rauthy, _ada, mut sink) = table(true).await?;
    let (status, first) = service
        .post(FOUND, None, &found(&OperationId::generate()?.to_string()))
        .await?;
    assert_eq!(status, 200, "{first}");
    let first_letter = sink.next().await.ok_or("the first letter")?;
    let after_first = leaves(&service).await?;
    let mut other_name = found(&OperationId::generate()?.to_string());
    other_name["organisation"] = json!("Bea's Other Bakery");
    let (status, second) = service.post(FOUND, None, &other_name).await?;
    assert_eq!(status, 200, "{second}");
    let second_letter = sink.next().await.ok_or("the second letter")?;
    assert_eq!(
        leaves(&service).await?,
        after_first,
        "a second founding was recorded"
    );
    let (old, new) = (token_in(&first_letter)?, token_in(&second_letter)?);
    assert_ne!(old, new);
    let stored = foundings(&service)?.ok_or("the foundings file")?;
    assert_eq!(
        stored["foundings"].as_object().map(serde_json::Map::len),
        Some(1),
        "{stored}"
    );
    let (status, refused, cookie) = prove(&service, &old).await?;
    assert_eq!(status, 403, "{refused}");
    assert_eq!(refused["refusal"], "proof_refused");
    assert!(cookie.is_none());
    let (status, proved, cookie) = prove(&service, &new).await?;
    assert_eq!(status, 200, "{proved}");
    assert!(cookie.is_some());
    // The organisation is the one first founded, whatever name was typed the second time.
    let (status, me) = service.get("/me", cookie.as_deref()).await?;
    assert_eq!(status, 200, "{me}");
    assert_eq!(me["organisation"]["id"], proved["organisation"]);
    assert_eq!(me["organisation"]["display_name"], "Bea's Bakery", "{me}");
    Ok(())
}

#[tokio::test]
async fn an_address_that_already_signs_in_gets_the_already_yours_letter_and_nothing_is_founded()
-> TestResult {
    let (service, _rauthy, _ada, mut sink) = table(true).await?;
    let before = leaves(&service).await?;
    let mut body = found(&OperationId::generate()?.to_string());
    body["email"] = json!(ADMIN_EMAIL);
    body["organisation"] = json!("Ada's Engine");
    let (status, answer) = service.post(FOUND, None, &body).await?;
    assert_eq!(status, 200, "{answer}");
    let members: Vec<&str> = answer
        .as_object()
        .ok_or("an object")?
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(
        members,
        ["letter", "operation"],
        "the shape differs: {answer}"
    );
    assert_eq!(answer["letter"]["sent"], true);
    assert_eq!(answer["letter"]["to"], ADMIN_EMAIL);
    let letter = sink.next().await.ok_or("the already-yours letter")?;
    assert_eq!(letter.subject(), "This address already signs in to Lys");
    assert_eq!(
        letter.body,
        format!(
            "Hello, someone asked to found Ada's Engine in Lys with this address. It already signs in to Lys at {}, so nothing was founded; sign in there. If this was not you, ignore this letter.",
            service.base
        )
    );
    assert_eq!(leaves(&service).await?, before);
    assert!(foundings(&service)?.is_none_or(|stored| stored["foundings"] == json!({})));
    assert!(sink.taken().is_none());
    Ok(())
}

#[tokio::test]
async fn a_malformed_body_is_refused_before_any_step() -> TestResult {
    let (service, _rauthy, _ada, mut sink) = table(true).await?;
    let before = leaves(&service).await?;
    let mut bad_email = found(&OperationId::generate()?.to_string());
    bad_email["email"] = json!("not-an-email");
    let mut blank_name = found(&OperationId::generate()?.to_string());
    blank_name["display_name"] = json!("   ");
    let mut blank_organisation = found(&OperationId::generate()?.to_string());
    blank_organisation["organisation"] = json!("");
    let mut unknown = found(&OperationId::generate()?.to_string());
    unknown["person"] = json!("person-1");
    for body in [bad_email, blank_name, blank_organisation, unknown] {
        let (status, refused) = service.post(FOUND, None, &body).await?;
        assert_eq!(status, 400, "{refused}");
        assert_eq!(refused["refusal"], "RequestMalformed", "{refused}");
        assert_eq!(refused["receipt"]["completed"], json!([]), "{refused}");
    }
    assert_eq!(leaves(&service).await?, before);
    assert!(foundings(&service)?.is_none());
    assert!(sink.taken().is_none());
    Ok(())
}
