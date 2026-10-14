//! Kept changes are carried in the content a caller can read.
use super::test_support::{certified_agent, change_call, leaves, nobody, post_signed, signed};
use reqwest::StatusCode;
use serde_json::{Value, json};
use std::error::Error;

/// A change the route refuses is kept once as an ask, never answered as a
/// receipt; the same refused ask made again, under a fresh operation id,
/// writes no second leaf; and once the directory has moved it is kept afresh,
/// as a later leaf.
#[tokio::test]
async fn a_refused_ask_is_kept_once_and_answered_as_asked_not_as_a_receipt()
-> Result<(), Box<dyn Error>> {
    let (service, cookie, agent, key) = certified_agent().await?;
    let before = leaves(&service, &cookie).await?;
    let mut first = None;
    for id in [11_u64, 12] {
        let bytes = change_call(id, "/people", &nobody()?)?;
        let signature = signed(&agent, &key, &bytes)?;
        let response = post_signed(&service.base, bytes, &signature).await?;
        assert_eq!(response.status(), StatusCode::OK);
        let answer: Value = response.json().await?;
        assert_eq!(answer["result"]["isError"], true, "{answer}");
        assert_eq!(
            crate::mcp_test_answer::body(&answer["result"])["status"],
            403,
            "an agent holding no grant to register people is refused after the ask is kept: {answer}"
        );
        assert_eq!(
            crate::mcp_test_answer::body(&answer["result"])["body"]["refusal"],
            "NotHeld"
        );
        assert!(
            crate::mcp_test_answer::body(&answer["result"])
                .get("receipt")
                .is_none(),
            "a refused change is never answered as a receipt: {answer}"
        );
        let asked = super::contract_tests::kept(&answer["result"], "asked")?;
        assert!(asked["operation"].is_string(), "the ask is named: {answer}");
        match &first {
            None => first = Some(asked),
            Some(first) => assert_eq!(&asked, first, "the same ask, the same leaf"),
        }
        assert_eq!(
            leaves(&service, &cookie).await?,
            before + 1,
            "one leaf for the ask, whatever the number of attempts or operation ids"
        );
    }
    let first = first.ok_or("the first ask")?;
    let other = json!({
        "operation": lys_identity::OperationId::generate()?.to_string(),
        "display_name": "Somebody"
    });
    let bytes = change_call(13, "/people", &other)?;
    let signature = signed(&agent, &key, &bytes)?;
    let answer: Value = post_signed(&service.base, bytes, &signature)
        .await?
        .json()
        .await?;
    assert_eq!(answer["result"]["isError"], true, "{answer}");
    assert_ne!(
        super::contract_tests::kept(&answer["result"], "asked")?,
        first,
        "a different ask is its own leaf"
    );
    assert_eq!(leaves(&service, &cookie).await?, before + 2);

    let (status, made) = service
        .post(
            "/people",
            Some(&cookie),
            &json!({
                "operation": lys_identity::OperationId::generate()?.to_string(),
                "display_name": "Somebody Real"
            }),
        )
        .await?;
    assert_eq!(status, 200, "the administrator moves the directory: {made}");
    let moved = leaves(&service, &cookie).await?;
    assert!(moved > before + 2, "the directory log moved");
    let bytes = change_call(14, "/people", &nobody()?)?;
    let signature = signed(&agent, &key, &bytes)?;
    let answer: Value = post_signed(&service.base, bytes, &signature)
        .await?
        .json()
        .await?;
    assert_eq!(answer["result"]["isError"], true, "{answer}");
    let again = super::contract_tests::kept(&answer["result"], "asked")?;
    assert_ne!(
        again, first,
        "the judgment it was refused under may have changed, so the ask is kept afresh"
    );
    let index = |asked: &Value| asked["log"]["index"].as_u64().ok_or("the leaf's index");
    assert!(
        index(&again)? > index(&first)?,
        "the fresh leaf is later than the one asked before: {again} after {first}"
    );
    assert_eq!(leaves(&service, &cookie).await?, moved + 1);
    Ok(())
}
