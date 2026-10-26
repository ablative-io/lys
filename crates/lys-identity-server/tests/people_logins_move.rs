#![cfg(test)]
//! DIRECTORY-122: repair is a signed batch; replay each independent hole.
#[path = "people_admit/table.rs"]
mod table;

use identity_contract::fake_issuer::Login;
use identity_contract::fake_rauthy::{API_KEY, FakeRauthy};
use identity_contract::harness::Service;
use lys_identity::OperationId;
use serde_json::{Value, json};
use table::{ADMIN_EMAIL, BEA, PASSWORD, TestResult, admit, signed_in_as, table_with_mail};

async fn add(
    service: &Service,
    ada: &str,
    name: &str,
    email: &str,
) -> Result<Value, Box<dyn std::error::Error>> {
    let mut body = admit(&OperationId::generate()?.to_string(), "alpha");
    body["display_name"] = json!(name);
    body["email"] = json!(email);
    let (status, answer) = service.post("/people/admit", Some(ada), &body).await?;
    assert_eq!(status, 200, "{answer}");
    Ok(answer)
}

async fn historical_email(
    rauthy: &FakeRauthy,
    subject: &str,
    email: &str,
    verified: bool,
) -> TestResult {
    let mut user = rauthy
        .users()?
        .into_iter()
        .find(|u| u["id"] == subject)
        .ok_or("the historical account")?;
    user["email"] = json!(email);
    user["email_verified"] = json!(verified);
    reqwest::Client::new()
        .put(format!("{}/users/{subject}", rauthy.api()))
        .header(reqwest::header::AUTHORIZATION, format!("API-Key {API_KEY}"))
        .json(&user)
        .send()
        .await?
        .error_for_status()?;
    Ok(())
}

fn move_body(
    service: &Service,
    added: &Value,
    target: &Value,
) -> Result<Value, Box<dyn std::error::Error>> {
    Ok(
        json!({"operation":OperationId::generate()?.to_string(), "issuer":service.issuer.issuer(), "subject":added["subject"], "target":target}),
    )
}

#[tokio::test]
async fn moving_a_historical_mix_up_records_one_batch_keeps_grants_and_ends_the_moved_session()
-> TestResult {
    let (service, rauthy, ada) = table_with_mail(false).await?;
    let dean = add(&service, &ada, "Dean Whiting", "dean@example.test").await?;
    // Historical pre-R1 bearer proof: a signed-out browser becomes Dean.
    let token = dean["letter"]["handed"]
        .as_str()
        .and_then(|s| s.split("/#/added/").nth(1))
        .ok_or("Dean's link")?;
    let response = reqwest::Client::new()
        .post(format!("{}/organisations/found/prove", service.base))
        .json(&json!({"proof":token}))
        .send()
        .await?;
    let mixed = identity_contract::harness::session_cookie(response).await?;
    let second_session = signed_in_as(&service, &dean, "dean@example.test").await?;
    let person = dean["person"].as_str().ok_or("Dean")?;
    let subject = dean["subject"].as_str().ok_or("Dean's subject")?;
    historical_email(&rauthy, subject, "annabel@example.test", true).await?;
    let before_grants = service.get("/grants", Some(&ada)).await?.1;
    let (_, me) = service.get("/me", Some(&ada)).await?;
    let path = format!("/people/{person}/logins/move");
    let (status, moved) = service
        .post(
            &path,
            Some(&ada),
            &move_body(&service, &dean, &json!({"display_name":"Annabel"}))?,
        )
        .await?;
    assert_eq!(status, 200, "{moved}");
    let target = moved["target"].as_str().ok_or("Annabel's new record")?;
    assert_ne!(target, person);
    assert_eq!(moved["source_grants"], json!([dean["grant"]]));
    assert_eq!(moved["target_grants"], json!([]));
    assert_eq!(service.get("/grants", Some(&ada)).await?.1, before_grants);
    let (_, source) = service
        .get(&format!("/identities/{person}"), Some(&ada))
        .await?;
    // Dean keeps the address held for him; the repair after the move gives
    // his record a new account under it (DIRECTORY-124 R2).
    let renewed = source["logins"].as_array().ok_or("Dean's logins")?;
    assert_eq!(renewed.len(), 1, "{source}");
    assert_ne!(renewed[0]["subject"], json!(subject));
    assert!(
        rauthy
            .users()?
            .iter()
            .any(|user| user["id"] == renewed[0]["subject"] && user["email"] == "dean@example.test"),
        "Dean's new account is under his own address"
    );
    let (_, target_record) = service
        .get(&format!("/identities/{target}"), Some(&ada))
        .await?;
    assert_eq!(target_record["display_name"], "Annabel");
    assert_eq!(target_record["state"], "active");
    let receipts = moved["receipts"].as_array().ok_or("one batch's receipts")?;
    assert_eq!(
        receipts.len(),
        4,
        "unbind, register, bind, activate: {moved}"
    );
    let mut indices = Vec::new();
    for receipt in receipts {
        assert_eq!(receipt["actor"]["subject"], me["signed_in"]["subject"]);
        let index = receipt["log"]["index"].as_u64().ok_or("the batch's leaf")?;
        indices.push(index);
        let (_, signed) = service
            .get(&format!("/receipts/{index}"), Some(&ada))
            .await?;
        assert_eq!(signed["receipt"], *receipt);
    }
    assert!(indices.windows(2).all(|pair| pair[1] == pair[0] + 1));
    let source_events = source["events"].as_array().ok_or("Dean's events")?;
    assert!(source_events.contains(&json!(indices[0])), "{source}");
    assert_eq!(target_record["events"], json!(&indices[1..]));
    assert_eq!(service.get("/me", Some(&mixed)).await?.0, 401);
    assert_eq!(service.get("/me", Some(&second_session)).await?.0, 401);
    assert_eq!(service.get("/me", Some(&ada)).await?.0, 200);
    let (back, binding) = service
        .issuer_answer(Login {
            subject: subject.to_owned(),
            email: "annabel@example.test".to_owned(),
        })
        .await?;
    let (_, location, cookie) = service.get_page(&back, &binding).await?;
    assert_eq!(location.as_deref(), Some("/#/me"));
    assert!(
        cookie,
        "the repaired login may now begin a provider session"
    );
    let held: Value =
        serde_json::from_slice(&std::fs::read(service.dir.path().join("foundings.json"))?)?;
    assert_eq!(held["admitted_email"][person], "dean@example.test");
    assert_eq!(held["admitted_email"][target], "annabel@example.test");
    let annabel = service
        .sign_in(Login {
            subject: subject.to_owned(),
            email: "annabel@example.test".to_owned(),
        })
        .await?;
    let (_, own) = service.get("/me", Some(&annabel)).await?;
    assert_eq!(own["person"]["id"], target);
    Ok(())
}

#[tokio::test]
async fn move_refusals_are_named_and_leave_both_records_and_sessions_alone() -> TestResult {
    let (service, _rauthy, ada) = table_with_mail(false).await?;
    let dean = add(&service, &ada, "Dean Whiting", "dean@example.test").await?;
    let annabel = add(&service, &ada, "Annabel", BEA).await?;
    let person = dean["person"].as_str().ok_or("Dean")?;
    let path = format!("/people/{person}/logins/move");
    let before = service
        .get(&format!("/identities/{person}"), Some(&ada))
        .await?;
    let target = json!({"person":annabel["person"]});
    let body = move_body(&service, &dean, &target)?;
    let mut missing = body.clone();
    missing["subject"] = json!("not-bound");
    let (_, refused) = service.post(&path, Some(&ada), &missing).await?;
    assert_eq!(refused["refusal"], "LoginNotBound");
    let bea = signed_in_as(&service, &annabel, BEA).await?;
    let (status, refused) = service.post(&path, Some(&bea), &body).await?;
    assert_eq!(status, 403, "{refused}");
    assert_eq!(refused["refusal"], "NotAdmitted");
    assert_eq!(service.post(&path, None, &body).await?.0, 401);
    assert_eq!(
        service
            .get(&format!("/identities/{person}"), Some(&ada))
            .await?,
        before
    );
    assert_eq!(service.get("/me", Some(&bea)).await?.0, 200);
    let target_id = annabel["person"].as_str().ok_or("Annabel's record")?;
    let target_before = service
        .get(&format!("/identities/{target_id}"), Some(&ada))
        .await?
        .1;
    let dean_session = signed_in_as(&service, &dean, "dean@example.test").await?;
    let grants_before = service.get("/grants", Some(&ada)).await?;
    let size = service.log_size().await?;
    let (status, moved) = service.post(&path, Some(&ada), &body).await?;
    assert_eq!(status, 200, "{moved}");
    let source = service
        .get(&format!("/identities/{person}"), Some(&ada))
        .await?
        .1;
    let target = service
        .get(&format!("/identities/{target_id}"), Some(&ada))
        .await?
        .1;
    assert_eq!(source["logins"], json!([]));
    let logins = target["logins"].as_array().ok_or("both logins")?;
    assert_eq!(logins.len(), 2);
    for added in [&dean, &annabel] {
        assert!(
            logins
                .iter()
                .any(|login| login["issuer"] == service.issuer.issuer()
                    && login["subject"] == added["subject"])
        );
    }
    assert_eq!(target["display_name"], target_before["display_name"]);
    assert_eq!(target["state"], target_before["state"]);
    assert_eq!(moved["source_grants"], json!([dean["grant"]]));
    assert_eq!(moved["target_grants"], json!([annabel["grant"]]));
    assert_eq!(service.get("/grants", Some(&ada)).await?, grants_before);
    assert_eq!(service.log_size().await?, size + 2);
    assert_eq!(service.get("/me", Some(&dean_session)).await?.0, 401);
    assert_eq!(
        service.get("/me", Some(&bea)).await?.1["person"]["id"],
        target_id
    );
    for (added, email) in [(&dean, "dean@example.test"), (&annabel, BEA)] {
        let session = signed_in_as(&service, added, email).await?;
        let (status, own) = service.get("/me", Some(&session)).await?;
        assert_eq!(status, 200, "{own}");
        assert_eq!(own["person"]["id"], target_id);
        assert_eq!(own["signed_in"]["subject"], added["subject"]);
    }
    Ok(())
}

#[tokio::test]
async fn moving_a_login_to_its_current_person_is_same_target_and_writes_nothing() -> TestResult {
    let (service, rauthy, ada) = table_with_mail(false).await?;
    let dean = add(&service, &ada, "Dean Whiting", "dean@example.test").await?;
    let annabel = add(&service, &ada, "Annabel", BEA).await?;
    let person = dean["person"].as_str().ok_or("Dean")?;
    let dean_session = signed_in_as(&service, &dean, "dean@example.test").await?;
    let annabel_session = signed_in_as(&service, &annabel, BEA).await?;
    let directory_before = service.get("/identities", Some(&ada)).await?;
    let grants_before = service.get("/grants", Some(&ada)).await?;
    let accounts_before = rauthy.users()?;
    let size = service.log_size().await?;
    let sessions_before = std::fs::read(service.dir.path().join("sessions.json"))?;
    let admissions_before = table::admission_files(service.dir.path())?;
    let (status, refused) = service
        .post(
            &format!("/people/{person}/logins/move"),
            Some(&ada),
            &move_body(&service, &dean, &json!({"person":person}))?,
        )
        .await?;
    assert_eq!(status, 409, "{refused}");
    assert_eq!(refused["refusal"], "SameTarget");
    assert_eq!(service.log_size().await?, size);
    assert_eq!(
        service.get("/identities", Some(&ada)).await?,
        directory_before
    );
    assert_eq!(service.get("/grants", Some(&ada)).await?, grants_before);
    assert_eq!(rauthy.users()?, accounts_before);
    assert_eq!(
        std::fs::read(service.dir.path().join("sessions.json"))?,
        sessions_before
    );
    assert_eq!(
        table::admission_files(service.dir.path())?,
        admissions_before
    );
    for (session, expected) in [
        (&dean_session, &dean["person"]),
        (&annabel_session, &annabel["person"]),
    ] {
        let (status, own) = service.get("/me", Some(session)).await?;
        assert_eq!(status, 200, "{own}");
        assert_eq!(&own["person"]["id"], expected);
    }
    assert_eq!(service.get("/me", Some(&ada)).await?.0, 200);
    Ok(())
}

#[tokio::test]
async fn an_existing_person_in_the_same_organisation_receives_only_the_login() -> TestResult {
    let (service, _rauthy, ada) = table_with_mail(false).await?;
    let dean = add(&service, &ada, "Dean Whiting", "dean@example.test").await?;
    let (status, target) = service
        .post(
            "/people",
            Some(&ada),
            &json!({"operation":OperationId::generate()?.to_string(),"display_name":"Annabel"}),
        )
        .await?;
    assert_eq!(status, 200, "{target}");
    let id = target["person"].as_str().ok_or("Annabel")?;
    let (status, active) = service
        .post(
            &format!("/identities/{id}/transitions"),
            Some(&ada),
            &json!({"operation":OperationId::generate()?.to_string(),"transition":"activate"}),
        )
        .await?;
    assert_eq!(status, 200, "{active}");
    let (status, grant) = service
        .post(
            "/grants/roots",
            Some(&ada),
            &json!({
                "operation":OperationId::generate()?.to_string(),"route":"api","holder":id,
                "resource":{"kind":"doc","id":"alpha"},"relation":"beta",
                "pass_on":{"kind":"use_only"},"window":{"starts_at":0,"ends_at":null},
            }),
        )
        .await?;
    assert_eq!(status, 200, "{grant}");
    let grants_before = service.get("/grants", Some(&ada)).await?;

    let path = format!(
        "/people/{}/logins/move",
        dean["person"].as_str().ok_or("Dean")?
    );
    let (status, moved) = service
        .post(
            &path,
            Some(&ada),
            &move_body(&service, &dean, &json!({"person":id}))?,
        )
        .await?;
    assert_eq!(status, 200, "{moved}");
    assert_eq!(moved["target"], id);
    assert_eq!(moved["receipts"].as_array().map(Vec::len), Some(2));
    assert_eq!(moved["target_grants"], json!([grant["grant"]]));
    assert_eq!(service.get("/grants", Some(&ada)).await?, grants_before);
    Ok(())
}

#[path = "people_logins_move/incident.rs"]
mod incident;
#[path = "people_logins_move/reconcile.rs"]
mod reconcile;

#[tokio::test]
async fn moving_to_a_person_in_another_organisation_is_refused_before_any_write() -> TestResult {
    let (service, _rauthy, ada) = table_with_mail(false).await?;
    let dean = add(&service, &ada, "Dean Whiting", "dean@example.test").await?;
    let (status, founded) = service.post("/organisations/found", None, &json!({
        "operation":OperationId::generate()?.to_string(),"organisation":"Another organisation",
        "display_name":"Annabel","email":"annabel@example.test",
    })).await?;
    assert_eq!(status, 200, "{founded}");
    let proof = founded["letter"]["handed"]
        .as_str()
        .and_then(|s| s.split("/#/found/").nth(1))
        .ok_or("the other organisation's proof")?;
    let (status, other) = service
        .post("/organisations/found/prove", None, &json!({"proof":proof}))
        .await?;
    assert_eq!(status, 200, "{other}");
    let source = dean["person"].as_str().ok_or("Dean")?;
    let before = service
        .get(&format!("/identities/{source}"), Some(&ada))
        .await?;
    let (status, refused) = service
        .post(
            &format!("/people/{source}/logins/move"),
            Some(&ada),
            &move_body(&service, &dean, &json!({"person":other["person"]}))?,
        )
        .await?;
    assert_eq!(status, 409, "{refused}");
    assert_eq!(refused["refusal"], "NotSameOrganisation");
    assert_eq!(
        service
            .get(&format!("/identities/{source}"), Some(&ada))
            .await?,
        before
    );
    Ok(())
}

#[tokio::test]
async fn the_configured_administrators_login_is_never_moved_to_another_person() -> TestResult {
    let (service, _rauthy, ada) = table_with_mail(false).await?;
    let (_, me) = service.get("/me", Some(&ada)).await?;
    let person = me["person"]["id"].as_str().ok_or("the administrator")?;
    let (_, before) = service.get("/directory/people", Some(&ada)).await?;
    let (status, refused) = service
        .post(
            &format!("/people/{person}/logins/move"),
            Some(&ada),
            &json!({
                "operation":OperationId::generate()?.to_string(),
                "issuer":service.issuer.issuer(), "subject":me["signed_in"]["subject"],
                "target":{"display_name":"Annabel"},
            }),
        )
        .await?;
    assert_eq!(status, 403, "{refused}");
    assert_eq!(refused["refusal"], "NotAdmitted");
    assert!(
        refused["reason"]
            .as_str()
            .ok_or("the reason")?
            .contains("administrator's own sign-in"),
        "{refused}"
    );
    let (status, after) = service.get("/directory/people", Some(&ada)).await?;
    assert_eq!(status, 200, "{after}");
    assert_eq!(after, before);
    let (_, own) = service.get("/me", Some(&ada)).await?;
    assert_eq!(own["person"]["id"], person);
    Ok(())
}
