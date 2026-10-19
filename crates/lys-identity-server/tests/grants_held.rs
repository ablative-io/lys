#![cfg(test)]
//! DIRECTORY-108: POST /grants/held answers an approved app whether a resource
//! of its own kind has any holder of an action, from the whole book, naming no
//! one; other kinds, other callers and no credential are refused by name.

use identity_contract::apps::{
    Auth, BEA, FILES, NOTES, TestResult, login, ok, post, refused, registered, root, seeded,
};
use identity_contract::harness::{ADMINISTRATOR, Service};
use serde_json::{Value, json};

fn question(kind: &str, action: &str) -> Value {
    json!({"route": "api", "resource": {"kind": kind, "id": "1"}, "action": action})
}

async fn held(
    service: &Service,
    credential: &str,
    kind: &str,
    action: &str,
) -> Result<Value, Box<dyn std::error::Error>> {
    ok(post(
        service,
        "/grants/held",
        Auth::Bearer(credential),
        &question(kind, action),
    )
    .await?)
}

#[tokio::test]
async fn an_app_without_a_service_account_reads_none_then_held_and_no_holder_is_named() -> TestResult
{
    let (service, seeded) = seeded().await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    let bea = seeded.people[1].id.to_string();
    let notes = registered(&service, &admin, NOTES).await?;
    let doc = format!("{NOTES}.doc");

    let none = held(&service, &notes, &doc, "read").await?;
    assert_eq!(none["held"], false, "{none}");
    let again = held(&service, &notes, &doc, "read").await?;
    assert_eq!(
        again["revision"], none["revision"],
        "asking wrote: {none} {again}"
    );

    ok(root(&service, &admin, &bea, (&doc, "1"), "reader").await?)?;
    let answer = held(&service, &notes, &doc, "read").await?;
    assert_eq!(answer["held"], true, "{answer}");
    let members: Vec<&String> = answer.as_object().ok_or("an object")?.keys().collect();
    assert_eq!(members, ["held", "revision"], "{answer}");
    assert!(
        !answer.to_string().contains(&bea),
        "the answer named the holder: {answer}"
    );
    let write = held(&service, &notes, &doc, "write").await?;
    assert_eq!(write["held"], false, "a reader holds no write: {write}");
    Ok(())
}

#[tokio::test]
async fn the_answer_does_not_depend_on_what_the_asking_app_sees() -> TestResult {
    let (service, seeded) = seeded().await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    let bea = seeded.people[1].id.to_string();
    let notes = registered(&service, &admin, NOTES).await?;
    let doc = format!("{NOTES}.doc");
    ok(root(&service, &admin, &bea, (&doc, "1"), "reader").await?)?;
    let who = json!({"route": "api", "resource": {"kind": doc, "id": "1"}, "action": "read", "page_size": 10, "after": null});
    let page = post(&service, "/grants/who", Auth::Bearer(&notes), &who).await?;
    let seen = page.0 == 200
        && page.1["holders"]
            .as_array()
            .is_some_and(|holders| !holders.is_empty());
    assert!(
        !seen,
        "the app sees the grant through who, so this test proves nothing: {page:?}"
    );
    assert_eq!(held(&service, &notes, &doc, "read").await?["held"], true);
    Ok(())
}

#[tokio::test]
async fn another_apps_kind_another_caller_and_no_credential_are_refused_and_answer_nothing()
-> TestResult {
    let (service, _seeded) = seeded().await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    let notes = registered(&service, &admin, NOTES).await?;
    registered(&service, &admin, FILES).await?;
    let files_doc = question(&format!("{FILES}.doc"), "read");
    let other = post(&service, "/grants/held", Auth::Bearer(&notes), &files_doc).await?;
    refused(&other, 403, "not_your_app")?;
    assert!(other.1.get("held").is_none(), "{other:?}");

    let bea = service.sign_in(login(BEA)).await?;
    let notes_doc = question(&format!("{NOTES}.doc"), "read");
    let person = post(&service, "/grants/held", Auth::Cookie(&bea), &notes_doc).await?;
    refused(&person, 403, "NotAdmitted")?;
    let nobody = post(&service, "/grants/held", Auth::Nobody, &notes_doc).await?;
    assert_ne!(nobody.0, 200, "{nobody:?}");
    assert!(nobody.1.get("held").is_none(), "{nobody:?}");

    let by_admin = ok(post(&service, "/grants/held", Auth::Cookie(&admin), &notes_doc).await?)?;
    assert_eq!(by_admin["held"], false, "{by_admin}");
    Ok(())
}
