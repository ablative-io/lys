//! The screen routes as the served binary answers them over HTTP: a broker
//! made and seeded with the product's own commands, `serve` started on a
//! loopback port, and every request signed on a person's behalf by a screen
//! service the broker trusts, with the library's own signer.

mod support;

use reqwest::Method;
use serde_json::{Value, json};
use support::served::{
    HIDDEN, OTHER, OWNED, OWNER, Seeded, Served, TestResult, names, percent_encoded,
};

#[test]
fn the_listing_holds_only_what_the_person_may_discover() -> TestResult {
    let served = Served::start(Seeded::new()?)?;

    let (status, body) = served.ask(&Method::GET, "/_lys/secrets", b"", OWNER)?;
    assert_eq!(status, 200, "{body}");
    let listed = names(&serde_json::from_str(&body)?);
    assert_eq!(listed, vec![OWNED.to_owned()], "{body}");

    let (status, body) = served.ask(&Method::GET, "/_lys/secrets", b"", OTHER)?;
    assert_eq!(status, 200, "{body}");
    assert_eq!(
        names(&serde_json::from_str(&body)?),
        vec![HIDDEN.to_owned()]
    );
    Ok(())
}

const SCOPE_CHANGE: &str = "screen-scope-change-01";
const RECIPIENTS_CHANGE: &str = "screen-recipients-change-01";

#[test]
fn only_the_owner_changes_a_secret() -> TestResult {
    let served = Served::start(Seeded::new()?)?;

    let scope = serde_json::to_vec(&json!({
        "secret": OWNED, "scope": "personal:person-other", "operation": SCOPE_CHANGE
    }))?;
    let (status, body) = served.ask(&Method::POST, "/_lys/scope", &scope, OTHER)?;
    assert_eq!(status, 403, "{body}");
    assert!(body.starts_with("LendingNotPermitted:"), "{body}");

    let recipients = serde_json::to_vec(&json!({
        "secret": OWNED, "recipients": "people_only", "operation": RECIPIENTS_CHANGE
    }))?;
    let (status, body) = served.ask(&Method::POST, "/_lys/recipients", &recipients, OWNER)?;
    assert_eq!(status, 200, "{body}");
    let answered: Value = serde_json::from_str(&body)?;
    assert_eq!(
        answered,
        json!({
            "secret": OWNED, "recipients": "people_only",
            "operation": RECIPIENTS_CHANGE, "repeated": false
        })
    );
    Ok(())
}

#[test]
fn an_owner_change_is_made_once_per_operation_id() -> TestResult {
    let served = Served::start(Seeded::new()?)?;

    let unmarked = serde_json::to_vec(&json!({ "secret": OWNED, "recipients": "people_only" }))?;
    let (status, body) = served.ask(&Method::POST, "/_lys/recipients", &unmarked, OWNER)?;
    assert_eq!(status, 400, "{body}");
    assert!(body.starts_with("OperationMissing:"), "{body}");

    let misshapen = serde_json::to_vec(&json!({
        "secret": OWNED, "recipients": "people_only", "operation": "too short"
    }))?;
    let (status, body) = served.ask(&Method::POST, "/_lys/recipients", &misshapen, OWNER)?;
    assert_eq!(status, 400, "{body}");
    assert!(body.starts_with("OperationMissing:"), "{body}");

    let recipients = serde_json::to_vec(&json!({
        "secret": OWNED, "recipients": "people_only", "operation": RECIPIENTS_CHANGE
    }))?;
    let (status, body) = served.ask(&Method::POST, "/_lys/recipients", &recipients, OWNER)?;
    assert_eq!(status, 200, "{body}");
    let (status, body) = served.ask(&Method::POST, "/_lys/recipients", &recipients, OWNER)?;
    assert_eq!(status, 200, "{body}");
    let again: Value = serde_json::from_str(&body)?;
    assert_eq!(again["repeated"], json!(true), "{body}");

    let other = serde_json::to_vec(&json!({
        "secret": OWNED, "recipients": "anyone", "operation": RECIPIENTS_CHANGE
    }))?;
    let (status, body) = served.ask(&Method::POST, "/_lys/recipients", &other, OWNER)?;
    assert_eq!(status, 400, "{body}");
    assert!(body.starts_with("OperationReused:"), "{body}");
    assert!(body.contains(RECIPIENTS_CHANGE), "{body}");
    Ok(())
}

#[test]
fn the_settings_route_reads_what_a_change_left() -> TestResult {
    let served = Served::start(Seeded::new()?)?;
    let target = format!("/_lys/settings?secret={OWNED}");

    let (status, body) = served.ask(&Method::GET, &target, b"", OWNER)?;
    assert_eq!(status, 200, "{body}");
    let before: Value = serde_json::from_str(&body)?;
    assert_eq!(
        before,
        json!({ "secret": OWNED, "scope": null, "recipients": "anyone", "last_operation": null })
    );

    let scope = serde_json::to_vec(&json!({
        "secret": OWNED, "scope": "personal:person-owner", "operation": SCOPE_CHANGE
    }))?;
    let (status, body) = served.ask(&Method::POST, "/_lys/scope", &scope, OWNER)?;
    assert_eq!(status, 200, "{body}");
    let recipients = serde_json::to_vec(&json!({
        "secret": OWNED, "recipients": "people_only", "operation": RECIPIENTS_CHANGE
    }))?;
    let (status, body) = served.ask(&Method::POST, "/_lys/recipients", &recipients, OWNER)?;
    assert_eq!(status, 200, "{body}");

    let (status, body) = served.ask(&Method::GET, &target, b"", OWNER)?;
    assert_eq!(status, 200, "{body}");
    let after: Value = serde_json::from_str(&body)?;
    assert_eq!(
        after,
        json!({
            "secret": OWNED, "scope": "person/person-owner",
            "recipients": "people_only", "last_operation": RECIPIENTS_CHANGE
        })
    );

    let (status, body) = served.ask(&Method::GET, &target, b"", OTHER)?;
    assert_eq!(status, 404, "{body}");
    assert!(body.starts_with("SecretUnknown:"), "{body}");

    let (status, body) = served.ask(&Method::GET, "/_lys/settings?secret=", b"", OWNER)?;
    assert_eq!(status, 400, "{body}");
    assert!(body.starts_with("Encoding:"), "{body}");
    Ok(())
}

#[test]
fn the_handles_route_lists_what_a_holder_holds_and_never_the_handle() -> TestResult {
    let served = Served::start(Seeded::new()?)?;
    let target = format!("/_lys/handles?holder={OWNER}");

    let (status, body) = served.ask(&Method::GET, &target, b"", OWNER)?;
    assert_eq!(status, 200, "{body}");
    let answered: Value = serde_json::from_str(&body)?;
    assert_eq!(answered["holder"], OWNER);
    let handles = answered["handles"].as_array().ok_or("no handles")?;
    assert_eq!(handles.len(), 1, "{body}");
    assert_eq!(handles[0]["id"], served.seeded.owned_handle);
    assert_eq!(handles[0]["secret"], OWNED);
    assert_eq!(handles[0]["used"], 0);
    assert_eq!(handles[0]["dropped"], false);
    let shown: Vec<&String> = handles[0]
        .as_object()
        .ok_or("not an object")?
        .keys()
        .collect();
    assert_eq!(
        shown,
        [
            "dropped",
            "id",
            "max_uses",
            "not_after_ms",
            "parent",
            "secret",
            "settled",
            "spend_cap",
            "used"
        ],
        "no token, digest or key is shown"
    );

    let (status, body) = served.ask(&Method::GET, &target, b"", OTHER)?;
    assert_eq!(status, 200, "{body}");
    let hidden: Value = serde_json::from_str(&body)?;
    assert_eq!(
        hidden,
        json!({ "holder": OWNER, "handles": [] }),
        "a handle on a secret the asker may not discover is left out"
    );

    let (status, body) = served.ask(&Method::GET, "/_lys/handles?holder=", b"", OWNER)?;
    assert_eq!(status, 400, "{body}");
    assert!(body.starts_with("Encoding:"), "{body}");
    Ok(())
}

#[test]
fn the_revocation_route_reads_a_percent_encoded_handle() -> TestResult {
    let served = Served::start(Seeded::new()?)?;
    let owned = served.seeded.owned_handle.clone();
    let hidden = served.seeded.hidden_handle.clone();

    let target = format!("/_lys/revocation?handle={}", percent_encoded(&owned));
    let request = served.signed(&Method::GET, &target, b"", OWNER)?;
    assert!(
        request
            .url()
            .query()
            .is_some_and(|query| query.contains('%')),
        "the handle must travel percent-encoded"
    );
    let (status, body) = served.send(request)?;
    assert_eq!(status, 200, "{body}");
    let answered: Value = serde_json::from_str(&body)?;
    assert_eq!(
        answered,
        json!({
            "handle": owned,
            "stopped_here": false,
            "upstream": "not_asked",
            "upstream_reason": null,
        })
    );

    let target = format!("/_lys/revocation?handle={}", percent_encoded(&hidden));
    let (status, body) = served.ask(&Method::GET, &target, b"", OWNER)?;
    assert_eq!(status, 404, "{body}");
    assert!(body.starts_with("HandleUnknown:"), "{body}");

    let target = format!("/_lys/revocation?handle={owned}&extra=1");
    let (status, body) = served.ask(&Method::GET, &target, b"", OWNER)?;
    assert_eq!(status, 400, "{body}");
    assert!(body.starts_with("Encoding:"), "{body}");
    Ok(())
}

#[test]
fn a_replayed_service_request_is_refused_by_name() -> TestResult {
    let served = Served::start(Seeded::new()?)?;

    let first = served.signed(&Method::GET, "/_lys/secrets", b"", OWNER)?;
    let again = first
        .try_clone()
        .ok_or("the signed request did not clone")?;
    let (status, body) = served.send(first)?;
    assert_eq!(status, 200, "{body}");
    let (status, body) = served.send(again)?;
    assert_eq!(status, 403, "{body}");
    assert!(body.starts_with("ServiceReplayed:"), "{body}");
    Ok(())
}
