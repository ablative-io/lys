#![cfg(test)]
//! What is read of a request body is decided by who sent it. A caller
//! judged in full before its body, a signed-in person, has the body read
//! whole, however long. A caller whose credential is judged over the body
//! has no more than 2 MiB read before it is verified, and a longer body is
//! refused `BodyTooLarge` at 413 before the credential is looked at, valid
//! or not. A short body with a credential is served as it always was. A
//! caller refused before its body, signed out or bearing a refused pass,
//! receives its refusal however long the body it sends.

use identity_contract::apps::{
    Auth, BEA, NOTES, TestResult, check, login, ok, post, refused, registered, root, seeded,
};
use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_identity::OperationId;
use lys_identity_server::dev_seed::seed_configured;
use serde_json::json;

/// Well past the 2 MiB read of an unverified caller's body.
const LONG: usize = 3 * 1024 * 1024;

#[tokio::test]
async fn a_session_saves_a_long_role_text_and_reads_it_back_whole() -> TestResult {
    let (service, _seeded) =
        Service::start_with(|config| Ok(seed_configured(config, [ADMINISTRATOR, BEA])?)).await?;
    let ada = service.sign_in(login(ADMINISTRATOR)).await?;
    let id = OperationId::generate()?.to_string();
    let responsibilities = "r".repeat(LONG);
    let body = json!({
        "operation": id,
        "name": "Archivist",
        "responsibilities": responsibilities,
        "goals": "Keeps every word.",
        "practice": "Reads the whole text.",
        "profile": "archivist.md",
        "grant_templates": [],
        "note": "A long text, kept whole.",
    });
    let (status, made) = service.post("/roles", Some(&ada), &body).await?;
    assert_eq!(
        status, 200,
        "a long role text is taken: {}",
        made["refusal"]
    );
    assert!(
        made["versions"][0]["responsibilities"] == responsibilities,
        "the role made holds the whole text"
    );
    let (status, read) = service.get(&format!("/roles/{id}"), Some(&ada)).await?;
    assert_eq!(status, 200, "the role reads back: {}", read["refusal"]);
    let kept = read["versions"][0]["responsibilities"]
        .as_str()
        .ok_or("the role read back has no responsibilities")?;
    assert_eq!(kept.len(), LONG, "every byte is kept");
    assert!(
        kept == responsibilities,
        "the text read back is the text sent"
    );
    Ok(())
}

#[tokio::test]
async fn a_long_body_with_an_unjudged_credential_is_refused_by_length() -> TestResult {
    let (service, seeded) = seeded().await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    let bea = seeded.people[1].id.to_string();
    let credential = registered(&service, &admin, NOTES).await?;
    let doc = format!("{NOTES}.doc");
    let guessed = format!("lys-app.{NOTES}.{}", "0".repeat(64));
    let long = json!({"checks": [check(&bea, &doc, &"1".repeat(LONG), "read")]});
    // A credential that would be refused, and one that would be taken: the
    // body is refused by its length first, since neither is judged before
    // the body is read.
    for bearer in [guessed.as_str(), credential.as_str()] {
        refused(
            &post(&service, "/grants/check/batch", Auth::Bearer(bearer), &long).await?,
            413,
            "BodyTooLarge",
        )?;
    }
    let short = json!({"checks": [check(&bea, &doc, "1", "read")]});
    refused(
        &post(
            &service,
            "/grants/check/batch",
            Auth::Bearer(&guessed),
            &short,
        )
        .await?,
        401,
        "credential_refused",
    )?;
    Ok(())
}

#[tokio::test]
async fn a_short_body_with_a_credential_is_served_as_before() -> TestResult {
    let (service, seeded) = seeded().await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    let bea = seeded.people[1].id.to_string();
    let credential = registered(&service, &admin, NOTES).await?;
    let doc = format!("{NOTES}.doc");
    ok(root(&service, &admin, &bea, (&doc, "1"), "reader").await?)?;
    let short = json!({"checks": [
        check(&bea, &doc, "1", "read"),
        check(&bea, &doc, "1", "write"),
    ]});
    let answer = ok(post(
        &service,
        "/grants/check/batch",
        Auth::Bearer(&credential),
        &short,
    )
    .await?)?;
    assert_eq!(answer["results"][0]["allowed"], json!(true), "{answer}");
    assert_eq!(answer["results"][1]["allowed"], json!(false), "{answer}");
    assert_eq!(answer["results"][1]["refusal"], "NotHeld", "{answer}");
    Ok(())
}

#[tokio::test]
async fn a_signed_out_caller_sending_a_long_body_receives_the_sign_in_refusal() -> TestResult {
    let service = Service::start().await?;
    let long = json!({"responsibilities": "r".repeat(LONG)});
    // Refused before the body is read; the body is read to its end and
    // dropped first, so the refusal arrives in place of a reset.
    refused(
        &post(&service, "/roles", Auth::Nobody, &long).await?,
        401,
        "NotSignedIn",
    )?;
    Ok(())
}

#[tokio::test]
async fn a_refused_run_pass_sending_a_long_body_receives_its_refusal() -> TestResult {
    let service = Service::start().await?;
    let long = json!({"responsibilities": "r".repeat(LONG)}).to_string();
    // A run pass beside a session cookie is refused before the body is read.
    let (status, answer) = service
        .post_carrying(
            "/roles",
            &[
                ("lys-agent-pass", "a-pass"),
                ("cookie", "lys_session=a-session"),
            ],
            long.into_bytes(),
        )
        .await?;
    assert_eq!(status, 401, "{answer}");
    assert_eq!(answer["refusal"], "AgentPassRefused", "{answer}");
    Ok(())
}
