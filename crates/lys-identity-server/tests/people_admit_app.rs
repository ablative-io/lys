#![cfg(test)]

//! A product admits a person through its own person pass (DIRECTORY-113):
//! POST /people/admit asked with an approved app's pass is judged as the
//! person the pass speaks for, admits only with a first grant on a kind the
//! app owns, refuses before any step otherwise, and its handed link goes on
//! to the app's own origin, which the proof answers.

use std::error::Error;

use base64::Engine;
use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use identity_contract::app_custody::secret;
use identity_contract::harness::Service;
use lys_identity::OperationId;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

#[path = "people_admit_app/table.rs"]
mod table;
use table::{APP, BEA, STEPS, accounts_for, admit_with, bea, member_of, people, table};

type TestResult = Result<(), Box<dyn Error>>;

/// The address the fixture app's registration lists, and its origin.
const BACK: &str = "https://app.example.test/signed-in";
const ORIGIN: &str = "https://app.example.test";
const VERIFIER: &str = "a-pass-verifier-of-enough-length-0123456789-abcdefghij";

/// The fixture app's person pass for the person signed in with `cookie`.
async fn app_pass(service: &Service, cookie: &str) -> Result<String, Box<dyn Error>> {
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(VERIFIER.as_bytes()));
    let sent = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()?
        .get(format!("{}/oauth/authorize", service.base))
        .query(&[
            ("client_id", APP),
            ("redirect_uri", BACK),
            ("response_type", "code"),
            ("scope", "openid"),
            ("code_challenge", challenge.as_str()),
            ("code_challenge_method", "S256"),
        ])
        .header(reqwest::header::COOKIE, cookie)
        .send()
        .await?;
    assert_eq!(sent.status(), 303);
    let back = sent
        .headers()
        .get(reqwest::header::LOCATION)
        .ok_or("the answer sends the browser nowhere")?
        .to_str()?;
    let code = reqwest::Url::parse(back)?
        .query_pairs()
        .find(|(key, _)| key == "code")
        .map(|(_, value)| value.into_owned())
        .ok_or("the app is given a code")?;
    let answer: Value = reqwest::Client::new()
        .post(format!("{}/oauth/token", service.base))
        .header(
            reqwest::header::AUTHORIZATION,
            format!("Basic {}", STANDARD.encode(format!("{APP}:{}", secret()))),
        )
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", code.as_str()),
            ("redirect_uri", BACK),
            ("code_verifier", VERIFIER),
        ])
        .send()
        .await?
        .json()
        .await?;
    Ok(answer["access_token"]
        .as_str()
        .ok_or_else(|| format!("the token route issues a pass: {answer}"))?
        .to_owned())
}

/// The token after `#/added/` in a handed link.
fn token_in(answer: &Value) -> Result<String, Box<dyn Error>> {
    let link = answer["letter"]["handed"]
        .as_str()
        .ok_or_else(|| format!("the answer hands a link: {answer}"))?;
    let (_, token) = link
        .split_once("#/added/")
        .ok_or_else(|| format!("the link is an admission link: {link}"))?;
    Ok(token.to_owned())
}

/// Post `token` to prove, answering the status and the body.
async fn prove(service: &Service, token: &str) -> Result<(u16, Value), Box<dyn Error>> {
    let answer = reqwest::Client::new()
        .post(format!("{}/organisations/found/prove", service.base))
        .json(&json!({ "proof": token }))
        .send()
        .await?;
    let status = answer.status().as_u16();
    Ok((status, answer.json().await?))
}

#[tokio::test]
async fn a_product_admits_a_person_as_the_person_whose_pass_it_holds() -> TestResult {
    let (service, rauthy, ada) = table().await?;
    let pass = app_pass(&service, &ada).await?;
    let operation = OperationId::generate()?.to_string();
    let asked = bea(&operation, member_of("fixture_product.workspace"));
    let (status, answer) = admit_with(&service, &pass, &asked).await?;
    assert_eq!(status, 200, "{answer}");
    assert_eq!(answer["receipt"]["completed"], json!(STEPS), "{answer}");
    assert!(answer["receipt"]["failed"].is_null(), "{answer}");
    // The act is Ada's, signed as her session, and its record names the app
    // it came through beside her (DIRECTORY-113 Amendment 2).
    let actor = &answer["receipt"]["actor"];
    assert_eq!(actor["through_app"], APP, "{answer}");
    let (status, me) = service.get("/me", Some(&ada)).await?;
    assert_eq!(status, 200, "{me}");
    assert_eq!(
        actor["subject"], me["signed_in"]["subject"],
        "{answer} {me}"
    );
    assert_eq!(
        actor["issuer"], me["signed_in"]["provider"],
        "{answer} {me}"
    );
    let person = answer["person"].as_str().ok_or("a person")?;
    let grant = answer["grant"].as_str().ok_or("a first grant")?;
    token_in(&answer)?;
    let (status, held) = service.get(&format!("/grants/{grant}"), Some(&ada)).await?;
    assert_eq!(status, 200, "{held}");
    let text = held.to_string();
    for word in [person, "fixture_product.workspace", "ws-1", "member"] {
        assert!(text.contains(word), "the grant names {word}: {held}");
    }
    // Asked again with the same operation, the act replays: the same person
    // and no second account.
    let (status, again) = admit_with(&service, &pass, &asked).await?;
    assert_eq!(status, 200, "{again}");
    assert_eq!(again["person"], answer["person"]);
    assert_eq!(again["receipt"]["logged"], answer["receipt"]["logged"]);
    assert_eq!(accounts_for(&rauthy, BEA)?, 1);
    Ok(())
}

#[tokio::test]
async fn a_product_is_refused_before_any_step_without_a_grant_on_its_own_kind() -> TestResult {
    let (service, rauthy, ada) = table().await?;
    let pass = app_pass(&service, &ada).await?;
    let before = people(&service, &ada).await?;
    let asks = [
        (
            bea(&OperationId::generate()?.to_string(), Value::Null),
            400,
            "app_admit_grant_required",
        ),
        (
            bea(
                &OperationId::generate()?.to_string(),
                member_of("cambium.workspace"),
            ),
            403,
            "not_your_app",
        ),
        (
            bea(
                &OperationId::generate()?.to_string(),
                member_of("fixture_product.room"),
            ),
            403,
            "kind_not_registered",
        ),
    ];
    for (asked, status, refusal) in asks {
        let (answered, answer) = admit_with(&service, &pass, &asked).await?;
        assert_eq!(
            (answered, answer["refusal"].as_str()),
            (status, Some(refusal)),
            "{answer}"
        );
        assert_eq!(answer["receipt"]["completed"], json!([]), "{answer}");
    }
    assert_eq!(people(&service, &ada).await?, before);
    assert_eq!(accounts_for(&rauthy, BEA)?, 0);
    Ok(())
}

#[tokio::test]
async fn a_person_without_organisation_admit_admits_nobody_through_a_product() -> TestResult {
    let (service, rauthy, ada) = table().await?;
    let pass = app_pass(&service, &ada).await?;
    let (status, answer) = admit_with(
        &service,
        &pass,
        &bea(
            &OperationId::generate()?.to_string(),
            member_of("fixture_product.workspace"),
        ),
    )
    .await?;
    assert_eq!(status, 200, "{answer}");
    let bea_cookie = service
        .sign_in(identity_contract::fake_issuer::Login {
            subject: answer["subject"].as_str().ok_or("a subject")?.to_owned(),
            email: BEA.to_owned(),
        })
        .await?;
    let bea_pass = app_pass(&service, &bea_cookie).await?;
    let before = people(&service, &ada).await?;
    let asked = json!({"operation": OperationId::generate()?.to_string(), "display_name": "Cy",
        "email": "cy@example.test", "grant": member_of("fixture_product.workspace")});
    let (status, refused) = admit_with(&service, &bea_pass, &asked).await?;
    assert_eq!(status, 403, "{refused}");
    assert_eq!(refused["receipt"]["completed"], json!([]), "{refused}");
    assert_eq!(people(&service, &ada).await?, before);
    assert_eq!(accounts_for(&rauthy, "cy@example.test")?, 0);
    Ok(())
}

#[tokio::test]
async fn the_handed_link_goes_on_to_the_products_own_origin() -> TestResult {
    let (service, _rauthy, ada) = table().await?;
    let pass = app_pass(&service, &ada).await?;
    let (status, answer) = admit_with(
        &service,
        &pass,
        &bea(
            &OperationId::generate()?.to_string(),
            member_of("fixture_product.workspace"),
        ),
    )
    .await?;
    assert_eq!(status, 200, "{answer}");
    let (status, proved) = prove(&service, &token_in(&answer)?).await?;
    assert_eq!(status, 200, "{proved}");
    assert_eq!(proved["continue_to"], ORIGIN, "{proved}");
    // The same act asked from Ada's own session goes on to nowhere.
    let asked = json!({"operation": OperationId::generate()?.to_string(), "display_name": "Cy",
        "email": "cy@example.test", "grant": member_of("fixture_product.workspace")});
    let (status, from_lys) = service.post("/people/admit", Some(&ada), &asked).await?;
    assert_eq!(status, 200, "{from_lys}");
    let (status, proved) = prove(&service, &token_in(&from_lys)?).await?;
    assert_eq!(status, 200, "{proved}");
    assert!(proved.get("continue_to").is_none(), "{proved}");
    // A body naming where to go is refused whole, nothing recorded.
    let mut named = bea(
        &OperationId::generate()?.to_string(),
        member_of("fixture_product.workspace"),
    );
    named["continue_to"] = json!("https://elsewhere.example.test");
    let (status, refused) = admit_with(&service, &pass, &named).await?;
    assert_eq!(
        (status, refused["refusal"].as_str()),
        (400, Some("RequestMalformed")),
        "{refused}"
    );
    Ok(())
}
