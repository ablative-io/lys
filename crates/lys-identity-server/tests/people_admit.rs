#![cfg(test)]

//! Admitting a person is one act (Scone's queued unit, Waffles 9 Oct 2026
//! 14:5x): register the person, make their sign-in account at the issuer by
//! email, bind that login, activate them and issue their first named root
//! grant, in that order. Each step runs under an operation derived from the
//! act's one operation, so asking again finishes what is left and makes
//! nothing twice; a failed step is answered by its name with the steps that
//! completed before it, and every answer carries one receipt.

#[path = "people_admit/admit_only.rs"]
mod admit_only;
#[path = "people_admit/table.rs"]
mod table;

use admit_only::{CY, STEPS, accounts_for, people, table};
use lys_identity::{OperationId, OrganisationId};
use serde_json::{Value, json};
use table::{BEA, TestResult, admission_files, admit, signed_in_as, table_with_mail};

#[tokio::test]
async fn one_act_registers_makes_the_account_binds_activates_and_grants() -> TestResult {
    let (service, rauthy, ada) = table().await?;
    let operation = OperationId::generate()?.to_string();
    let (status, answer) = service
        .post("/people/admit", Some(&ada), &admit(&operation, "alpha"))
        .await?;
    assert_eq!(status, 200, "{answer}");
    let person = answer["person"].as_str().ok_or("a person")?;
    let subject = answer["subject"].as_str().ok_or("an issuer subject")?;
    assert!(answer["grant"].as_str().is_some(), "{answer}");
    let receipt = &answer["receipt"];
    assert_eq!(receipt["operation"], operation);
    assert_eq!(receipt["completed"], json!(STEPS));
    assert!(receipt["failed"].is_null(), "{receipt}");
    // Every step that writes a log names its leaf: the directory's three and
    // the grant's. The issuer account is written at the issuer, not a log.
    let logged: Vec<(Value, Value)> = receipt["logged"]
        .as_array()
        .ok_or("the receipt names its log leaves")?
        .iter()
        .map(|leaf| (leaf["step"].clone(), leaf["log"].clone()))
        .collect();
    assert_eq!(
        logged,
        [
            (json!("register"), json!("directory")),
            (json!("bind"), json!("directory")),
            (json!("activate"), json!("directory")),
            (json!("grant"), json!("grants")),
        ],
        "{receipt}"
    );
    let directory: Vec<u64> = receipt["logged"]
        .as_array()
        .ok_or("leaves")?
        .iter()
        .filter(|leaf| leaf["log"] == "directory")
        .filter_map(|leaf| leaf["index"].as_u64())
        .collect();
    assert_eq!(directory.len(), 3, "{receipt}");
    assert!(
        directory.windows(2).all(|pair| pair[0] < pair[1]),
        "{receipt}"
    );
    assert_eq!(accounts_for(&rauthy, BEA)?, 1);
    let (status, read) = service
        .get(&format!("/identities/{person}"), Some(&ada))
        .await?;
    assert_eq!(status, 200, "{read}");
    assert_eq!(read["state"], "active", "{read}");
    let logins = read["logins"].to_string();
    assert!(logins.contains(subject), "the login is bound: {read}");
    Ok(())
}

#[tokio::test]
async fn asking_again_answers_the_same_person_and_makes_nothing_twice() -> TestResult {
    let (service, rauthy, ada) = table().await?;
    let before = people(&service, &ada).await?;
    let operation = OperationId::generate()?.to_string();
    let body = admit(&operation, "alpha");
    let (status, first) = service.post("/people/admit", Some(&ada), &body).await?;
    assert_eq!(status, 200, "{first}");
    let (status, again) = service.post("/people/admit", Some(&ada), &body).await?;
    assert_eq!(status, 200, "{again}");
    for member in ["person", "subject", "grant"] {
        assert_eq!(first[member], again[member], "{member}");
    }
    // The receipt is the logs' own: asking again answers the same leaves.
    assert_eq!(first["receipt"]["logged"], again["receipt"]["logged"]);
    assert_eq!(accounts_for(&rauthy, BEA)?, 1);
    assert_eq!(people(&service, &ada).await?, before + 1);
    Ok(())
}

#[tokio::test]
async fn adding_an_admitted_email_again_adds_the_grant_and_makes_no_second_person() -> TestResult {
    let (service, rauthy, ada) = table().await?;
    let before = people(&service, &ada).await?;
    let first_op = OperationId::generate()?.to_string();
    let (status, first) = service
        .post("/people/admit", Some(&ada), &admit(&first_op, "alpha"))
        .await?;
    assert_eq!(status, 200, "{first}");
    // Added again under a new operation with another first grant: the person
    // and their account stand, the steps that made them are already done,
    // and the grant asked for is added to what they hold.
    let again_op = OperationId::generate()?.to_string();
    let (status, again) = service
        .post("/people/admit", Some(&ada), &admit(&again_op, "beta"))
        .await?;
    assert_eq!(status, 200, "{again}");
    assert_eq!(again["person"], first["person"], "{again}");
    assert_eq!(again["subject"], first["subject"], "{again}");
    assert_ne!(again["grant"], first["grant"], "{again}");
    assert_eq!(again["receipt"]["completed"], json!(STEPS), "{again}");
    assert!(again["receipt"]["failed"].is_null(), "{again}");
    let logged: Vec<Value> = again["receipt"]["logged"]
        .as_array()
        .ok_or("leaves")?
        .iter()
        .map(|leaf| leaf["step"].clone())
        .collect();
    assert_eq!(logged, [json!("grant")], "{again}");
    assert_eq!(accounts_for(&rauthy, BEA)?, 1);
    assert_eq!(people(&service, &ada).await?, before + 1);
    for (answer, relation) in [(&first, "alpha"), (&again, "beta")] {
        let grant = answer["grant"].as_str().ok_or("a grant")?;
        let (status, held) = service.get(&format!("/grants/{grant}"), Some(&ada)).await?;
        assert_eq!(status, 200, "{held}");
        assert!(held.to_string().contains(relation), "{held}");
    }
    Ok(())
}

#[tokio::test]
async fn a_failed_step_is_named_with_the_steps_completed_before_it() -> TestResult {
    let (service, rauthy, ada) = table().await?;
    let before = people(&service, &ada).await?;
    let operation = OperationId::generate()?.to_string();
    let body = admit(&operation, "gamma");
    let (status, refused) = service.post("/people/admit", Some(&ada), &body).await?;
    assert_eq!(status, 403, "{refused}");
    assert_eq!(refused["refusal"], "RelationUnknown", "{refused}");
    let receipt = &refused["receipt"];
    assert_eq!(receipt["failed"], "grant", "{refused}");
    assert_eq!(receipt["completed"], json!(STEPS[..4]), "{refused}");
    let person = receipt["person"].as_str().ok_or("the registered person")?;
    // Asking again with the same words fails at the same step and repeats
    // none of the four before it.
    let (status, again) = service.post("/people/admit", Some(&ada), &body).await?;
    assert_eq!(status, 403, "{again}");
    assert_eq!(again["receipt"]["failed"], "grant");
    assert_eq!(again["receipt"]["person"], person);
    assert_eq!(accounts_for(&rauthy, BEA)?, 1);
    assert_eq!(people(&service, &ada).await?, before + 1);
    Ok(())
}

/// Reuse is caught at the first step whose words differ: here the grant, after
/// the four steps before it replay without writing. (A changed email is caught
/// at the bind, after the issuer has found or made that account: named in the
/// handback as a gap, not hidden.)
#[tokio::test]
async fn the_same_operation_with_other_grant_words_is_refused_at_the_grant() -> TestResult {
    let (service, rauthy, ada) = table().await?;
    let before = people(&service, &ada).await?;
    let operation = OperationId::generate()?.to_string();
    let (status, first) = service
        .post("/people/admit", Some(&ada), &admit(&operation, "alpha"))
        .await?;
    assert_eq!(status, 200, "{first}");
    let (status, refused) = service
        .post("/people/admit", Some(&ada), &admit(&operation, "beta"))
        .await?;
    assert_ne!(status, 200, "{refused}");
    assert_eq!(refused["receipt"]["failed"], "grant", "{refused}");
    assert_eq!(refused["receipt"]["completed"], json!(STEPS[..4]));
    assert_eq!(refused["receipt"]["person"], first["person"]);
    assert_eq!(accounts_for(&rauthy, BEA)?, 1);
    assert_eq!(people(&service, &ada).await?, before + 1);
    Ok(())
}

#[tokio::test]
async fn only_the_administrator_admits_and_a_refusal_writes_nothing() -> TestResult {
    let (service, rauthy, ada) = table().await?;
    let before = people(&service, &ada).await?;
    let operation = OperationId::generate()?.to_string();
    let (status, refused) = service
        .post("/people/admit", None, &admit(&operation, "alpha"))
        .await?;
    assert_eq!(status, 401, "{refused}");
    assert_eq!(refused["refusal"], "NotSignedIn", "{refused}");
    assert_eq!(accounts_for(&rauthy, BEA)?, 0);
    assert_eq!(people(&service, &ada).await?, before);
    Ok(())
}

#[tokio::test]
async fn a_malformed_email_or_missing_member_is_refused_before_any_step() -> TestResult {
    let (service, rauthy, ada) = table().await?;
    let before = people(&service, &ada).await?;
    let operation = OperationId::generate()?.to_string();
    let mut bad_email = admit(&operation, "alpha");
    bad_email["email"] = json!("not-an-email");
    let mut invalid = vec![bad_email];
    for member in ["operation", "display_name", "email"] {
        let mut body = admit(&OperationId::generate()?.to_string(), "alpha");
        body.as_object_mut().ok_or("an object")?.remove(member);
        invalid.push(body);
    }
    for body in invalid {
        let (status, refused) = service.post("/people/admit", Some(&ada), &body).await?;
        assert_eq!(status, 400, "{refused}");
        assert_eq!(refused["refusal"], "RequestMalformed", "{refused}");
        assert!(
            refused["receipt"]["completed"]
                .as_array()
                .is_none_or(Vec::is_empty)
        );
    }
    assert_eq!(accounts_for(&rauthy, BEA)?, 0);
    assert_eq!(people(&service, &ada).await?, before);
    let mut no_grant = admit(&OperationId::generate()?.to_string(), "alpha");
    no_grant.as_object_mut().ok_or("an object")?.remove("grant");
    let (status, admitted) = service.post("/people/admit", Some(&ada), &no_grant).await?;
    assert_eq!(status, 200, "{admitted}");
    assert!(admitted["grant"].is_null(), "{admitted}");
    assert_eq!(
        admitted["receipt"]["completed"],
        json!(["register", "account", "bind", "activate", "letter"])
    );
    // Asked from the person's own session, the act names no app
    // (DIRECTORY-113 Amendment 2).
    let actor = &admitted["receipt"]["actor"];
    assert!(actor["subject"].is_string(), "{admitted}");
    assert!(actor.get("through_app").is_none(), "{admitted}");
    assert_eq!(accounts_for(&rauthy, BEA)?, 1);
    assert_eq!(people(&service, &ada).await?, before + 1);
    Ok(())
}

/// ACCESS-009 R1: admitting is the organisation.admit grant on the
/// caller's own organisation, on both routes that register a person.
#[tokio::test]
async fn a_person_without_organisation_admit_is_refused_by_name_and_writes_nothing() -> TestResult {
    let (service, rauthy, ada) = table().await?;
    let (status, admitted) = service
        .post(
            "/people/admit",
            Some(&ada),
            &admit(&OperationId::generate()?.to_string(), "alpha"),
        )
        .await?;
    assert_eq!(status, 200, "{admitted}");
    let bea = signed_in_as(&service, &admitted, BEA).await?;
    let before = people(&service, &ada).await?;
    let body = json!({
        "operation": OperationId::generate()?.to_string(),
        "display_name": "Cy",
        "email": CY,
    });
    let (status, refused) = service.post("/people/admit", Some(&bea), &body).await?;
    assert_eq!(status, 403, "{refused}");
    assert_eq!(refused["refusal"], "NotAdmitted", "{refused}");
    assert!(
        refused["reason"]
            .as_str()
            .is_some_and(|reason| reason.contains("organisation.admit")),
        "the refusal names the act: {refused}"
    );
    assert!(refused["receipt"]["failed"].is_null(), "{refused}");
    assert_eq!(refused["receipt"]["completed"], json!([]), "{refused}");
    let body = json!({
        "operation": OperationId::generate()?.to_string(),
        "display_name": "Cy",
    });
    let (status, refused) = service.post("/people", Some(&bea), &body).await?;
    assert_eq!(status, 403, "{refused}");
    assert_eq!(refused["refusal"], "NotAdmitted", "{refused}");
    assert_eq!(people(&service, &ada).await?, before);
    assert_eq!(accounts_for(&rauthy, CY)?, 0);
    Ok(())
}

/// ACCESS-009 R1: a body naming another organisation is refused before any
/// step; naming the caller's own names nothing new.
#[tokio::test]
async fn a_body_naming_another_organisation_is_refused_before_any_step() -> TestResult {
    let (service, rauthy, ada) = table().await?;
    let before = people(&service, &ada).await?;
    let mut body = admit(&OperationId::generate()?.to_string(), "alpha");
    body["organisation"] = json!(OrganisationId::generate()?.to_string());
    let (status, refused) = service.post("/people/admit", Some(&ada), &body).await?;
    assert_eq!(status, 403, "{refused}");
    assert_eq!(refused["refusal"], "organisation_not_own", "{refused}");
    assert!(refused["receipt"]["failed"].is_null(), "{refused}");
    assert_eq!(refused["receipt"]["completed"], json!([]), "{refused}");
    assert_eq!(people(&service, &ada).await?, before);
    assert_eq!(accounts_for(&rauthy, BEA)?, 0);

    let (status, me) = service.get("/me", Some(&ada)).await?;
    assert_eq!(status, 200, "{me}");
    let own = me["organisation"]["id"]
        .as_str()
        .ok_or("the caller's organisation")?
        .to_owned();
    let mut body = admit(&OperationId::generate()?.to_string(), "alpha");
    body["organisation"] = json!(own);
    let (status, admitted) = service.post("/people/admit", Some(&ada), &body).await?;
    assert_eq!(status, 200, "{admitted}");
    assert_eq!(people(&service, &ada).await?, before + 1);
    Ok(())
}

/// ACCESS-009 R1: the administrator admits through the founding grant and
/// through nothing else; with it revoked they are refused like anyone.
#[tokio::test]
async fn the_administrator_admits_by_the_founding_grant_alone() -> TestResult {
    let (service, rauthy, ada) = table().await?;
    let (status, grants) = service.get("/grants", Some(&ada)).await?;
    assert_eq!(status, 200, "{grants}");
    let founding = grants["grants"]
        .as_array()
        .ok_or("the grants are listed")?
        .iter()
        .find(|grant| {
            grant["resource"]["kind"] == "organisation" && grant["relation"] == "administrator"
        })
        .ok_or("the founding grant is listed")?;
    let id = founding["id"].as_str().ok_or("a grant id")?;
    let (status, revoked) = service
        .post(
            &format!("/grants/{id}/revoke"),
            Some(&ada),
            &json!({
                "operation": OperationId::generate()?.to_string(),
                "route": "browser",
                "reason": "the founder steps back",
            }),
        )
        .await?;
    assert_eq!(status, 200, "{revoked}");
    let before = people(&service, &ada).await?;
    let (status, refused) = service
        .post(
            "/people/admit",
            Some(&ada),
            &admit(&OperationId::generate()?.to_string(), "alpha"),
        )
        .await?;
    assert_eq!(status, 403, "{refused}");
    assert_eq!(refused["refusal"], "NotAdmitted", "{refused}");
    assert_eq!(people(&service, &ada).await?, before);
    assert_eq!(accounts_for(&rauthy, BEA)?, 0);
    Ok(())
}

/// DIRECTORY-121 R1: an email is never silently admitted as another name.
#[tokio::test]
async fn an_email_held_under_another_name_is_refused_without_writing() -> TestResult {
    for mail in [false, true] {
        let (service, rauthy, ada) = table_with_mail(mail).await?;
        let mut body = admit(&OperationId::generate()?.to_string(), "alpha");
        body["display_name"] = json!("Dean Whiting");
        body["email"] = json!("a@example.test");
        let (status, first) = service.post("/people/admit", Some(&ada), &body).await?;
        assert_eq!(status, 200, "{first}");
        let person = first["person"].as_str().ok_or("the existing person")?;
        let before_people = people(&service, &ada).await?;
        let before_grants = service.get("/grants", Some(&ada)).await?;
        let before_record = service
            .get(&format!("/identities/{person}"), Some(&ada))
            .await?;
        let before_files = admission_files(service.dir.path())?;
        body["operation"] = json!(OperationId::generate()?.to_string());
        body["display_name"] = json!("Annabel");
        body["grant"]["relation"] = json!("beta");
        for explicit in [
            None,
            Some(
                OperationId::generate()?
                    .to_string()
                    .replace("op-", "person-"),
            ),
        ] {
            if let Some(other) = explicit {
                body["person"] = json!(other);
            }
            let (status, refused) = service.post("/people/admit", Some(&ada), &body).await?;
            assert_eq!(status, 409, "{refused}");
            assert_eq!(refused["refusal"], "EmailHeldByAnother", "{refused}");
            assert_eq!(refused["person"], person, "{refused}");
            assert_eq!(refused["display_name"], "Dean Whiting", "{refused}");
            assert_eq!(refused["email"], "a@example.test", "{refused}");
            assert_eq!(refused["receipt"]["completed"], json!([]), "{refused}");
            assert_eq!(service.get("/grants", Some(&ada)).await?, before_grants);
            assert_eq!(
                service
                    .get(&format!("/identities/{person}"), Some(&ada))
                    .await?,
                before_record
            );
            assert_eq!(admission_files(service.dir.path())?, before_files);
        }
        assert_eq!(people(&service, &ada).await?, before_people);
        assert_eq!(accounts_for(&rauthy, "a@example.test")?, 1);
    }
    Ok(())
}

#[tokio::test]
async fn naming_the_held_person_or_their_exact_trimmed_name_records_existing() -> TestResult {
    let (service, _rauthy, ada) = table().await?;
    let mut body = admit(&OperationId::generate()?.to_string(), "alpha");
    body["display_name"] = json!("Dean Whiting");
    body["email"] = json!("a@example.test");
    let (status, first) = service.post("/people/admit", Some(&ada), &body).await?;
    assert_eq!(status, 200, "{first}");
    let person = first["person"].as_str().ok_or("the existing person")?;
    for name in ["Annabel", "Dean Whiting", "  Dean Whiting  "] {
        body["operation"] = json!(OperationId::generate()?.to_string());
        body["display_name"] = json!(name);
        body["grant"]["relation"] = json!("beta");
        if name == "Annabel" {
            body["person"] = json!(person);
        } else {
            body.as_object_mut().ok_or("an object")?.remove("person");
        }
        let (status, answer) = service.post("/people/admit", Some(&ada), &body).await?;
        assert_eq!(status, 200, "{answer}");
        assert_eq!(answer["person"], person);
        assert_eq!(
            answer["receipt"]["existing"],
            json!({"person": person, "display_name": "Dean Whiting"})
        );
        assert_eq!(answer["display_name"], "Dean Whiting");
        let (status, again) = service.post("/people/admit", Some(&ada), &body).await?;
        assert_eq!(status, 200, "{again}");
        assert_eq!(again["receipt"]["existing"], answer["receipt"]["existing"]);
    }
    body["operation"] = json!(OperationId::generate()?.to_string());
    body["display_name"] = json!("dean whiting");
    let (status, refused) = service.post("/people/admit", Some(&ada), &body).await?;
    assert_eq!(status, 409, "{refused}");
    assert_eq!(refused["refusal"], "EmailHeldByAnother");
    Ok(())
}

#[tokio::test]
async fn only_an_administrators_people_read_carries_sign_in_and_admitted_email() -> TestResult {
    let (service, _rauthy, ada) = table_with_mail(false).await?;
    let (status, added) = service
        .post(
            "/people/admit",
            Some(&ada),
            &admit(&OperationId::generate()?.to_string(), "alpha"),
        )
        .await?;
    assert_eq!(status, 200, "{added}");
    let person = added["person"].as_str().ok_or("Bea")?;
    let bea = signed_in_as(&service, &added, BEA).await?;
    let (status, changed) = service
        .post(
            &format!("/directory/people/{person}/account/email"),
            Some(&ada),
            &json!({"email":"annabel@example.test"}),
        )
        .await?;
    assert_eq!(status, 200, "{changed}");
    for path in ["/directory/people", "/directory/people?limit=50"] {
        let (status, list) = service.get(path, Some(&ada)).await?;
        assert_eq!(status, 200, "{list}");
        let row = list["people"]
            .as_array()
            .ok_or("people")?
            .iter()
            .find(|row| row["id"] == person)
            .ok_or("Bea's row")?;
        assert_eq!(row["sign_in_email"], "annabel@example.test");
        assert_eq!(row["admitted_email"], BEA);
    }
    let (status, list) = service.get("/people", Some(&bea)).await?;
    assert_eq!(status, 200, "{list}");
    for row in list["people"].as_array().ok_or("people")? {
        assert!(
            row.get("sign_in_email").is_none()
                && row.get("admitted_email").is_none()
                && row.get("email_recorded_at_sign_in").is_none(),
            "{row}"
        );
    }
    assert_eq!(service.get("/directory/people", Some(&bea)).await?.0, 403);
    Ok(())
}

#[tokio::test]
async fn a_mailed_admission_holds_the_original_address_before_any_provider_sign_in() -> TestResult {
    let (service, _rauthy, ada) = table().await?;
    let (status, added) = service
        .post(
            "/people/admit",
            Some(&ada),
            &admit(&OperationId::generate()?.to_string(), "alpha"),
        )
        .await?;
    assert_eq!(status, 200, "{added}");
    let person = added["person"].as_str().ok_or("Bea")?;
    let stored: Value =
        serde_json::from_slice(&std::fs::read(service.dir.path().join("foundings.json"))?)?;
    assert_eq!(stored["admitted_email"][person], BEA);
    Ok(())
}
