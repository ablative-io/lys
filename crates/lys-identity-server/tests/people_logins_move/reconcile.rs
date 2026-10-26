//! DIRECTORY-124: sign-in accounts follow the directory. The issuer is
//! wiped and Lys started again; the 26 October records are replayed whole.
use super::*;

/// Each person's sign-in as the administrator's People read judges it.
async fn health(
    service: &Service,
    ada: &str,
) -> Result<Vec<(String, Value)>, Box<dyn std::error::Error>> {
    let (status, people) = service.get("/directory/people", Some(ada)).await?;
    assert_eq!(status, 200, "{people}");
    Ok(people["people"]
        .as_array()
        .ok_or("the people")?
        .iter()
        .map(|person| {
            (
                person["display_name"]
                    .as_str()
                    .unwrap_or_default()
                    .to_owned(),
                person["sign_in"].clone(),
            )
        })
        .collect())
}

fn state_of(health: &[(String, Value)], name: &str) -> Value {
    health
        .iter()
        .find(|(shown, _)| shown == name)
        .map_or(Value::Null, |(_, health)| health.clone())
}

async fn logins(
    service: &Service,
    ada: &str,
    person: &Value,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let id = person["person"].as_str().ok_or("a person")?;
    let (_, record) = service.get(&format!("/identities/{id}"), Some(ada)).await?;
    Ok(record["logins"]
        .as_array()
        .ok_or("their logins")?
        .iter()
        .filter_map(|login| login["subject"].as_str().map(str::to_owned))
        .collect())
}

#[tokio::test]
async fn wiping_the_issuer_and_starting_again_gives_everyone_an_account_under_their_own_address()
-> TestResult {
    let (mut service, rauthy, ada) = table_with_mail(false).await?;
    let bea = add(&service, &ada, "Bea", "bea@example.test").await?;
    let cal = add(&service, &ada, "Cal", "cal@example.test").await?;
    let before = [
        logins(&service, &ada, &bea).await?,
        logins(&service, &ada, &cal).await?,
    ];
    for person in [&bea, &cal] {
        rauthy.forget(person["subject"].as_str().ok_or("an account")?)?;
    }
    let judged = health(&service, &ada).await?;
    assert_eq!(state_of(&judged, "Bea"), json!({"state": "repairable"}));
    assert_eq!(state_of(&judged, "Cal"), json!({"state": "repairable"}));

    service.restart().await?;

    for ((person, email), old) in [(&bea, "bea@example.test"), (&cal, "cal@example.test")]
        .into_iter()
        .zip(before)
    {
        let now = logins(&service, &ada, person).await?;
        assert_eq!(
            now.len(),
            1,
            "the dead login is replaced, not kept beside the new one: {now:?}"
        );
        assert!(
            !old.contains(&now[0]),
            "a new account, not the forgotten one"
        );
        assert!(
            rauthy
                .users()?
                .iter()
                .any(|user| user["id"] == now[0] && user["email"] == email),
            "the new account is under {email}"
        );
        let session = service
            .sign_in(Login {
                subject: now[0].clone(),
                email: email.to_owned(),
            })
            .await?;
        let (status, own) = service.get("/me", Some(&session)).await?;
        assert_eq!(status, 200, "{own}");
        assert_eq!(own["person"]["id"], person["person"]);
    }
    let judged = health(&service, &ada).await?;
    assert_eq!(state_of(&judged, "Bea"), json!({"state": "ready"}));
    assert_eq!(state_of(&judged, "Cal"), json!({"state": "ready"}));
    // A second start finds nothing to do.
    let accounts = rauthy.users()?;
    service.restart().await?;
    assert_eq!(rauthy.users()?, accounts);
    Ok(())
}

/// Move `person` through `transition` as the administrator.
async fn transition(service: &Service, ada: &str, person: &Value, transition: &str) -> TestResult {
    let id = person["person"].as_str().ok_or("a person")?;
    let (status, answer) = service
        .post(
            &format!("/identities/{id}/transitions"),
            Some(ada),
            &json!({"operation": OperationId::generate()?.to_string(), "transition": transition}),
        )
        .await?;
    assert_eq!(status, 200, "{answer}");
    Ok(())
}

#[tokio::test]
async fn a_suspended_person_is_given_no_account_until_they_are_reinstated() -> TestResult {
    let (mut service, rauthy, ada) = table_with_mail(false).await?;
    let cal = add(&service, &ada, "Cal", "cal@example.test").await?;
    let old = logins(&service, &ada, &cal).await?;
    transition(&service, &ada, &cal, "suspend").await?;
    rauthy.forget(cal["subject"].as_str().ok_or("an account")?)?;
    let accounts = rauthy.users()?;

    service.restart().await?;

    assert_eq!(
        rauthy.users()?,
        accounts,
        "no account is made for a suspended person"
    );
    assert_eq!(
        logins(&service, &ada, &cal).await?,
        old,
        "their dead login is kept until a repair replaces it"
    );
    transition(&service, &ada, &cal, "reinstate").await?;
    let now = logins(&service, &ada, &cal).await?;
    assert_eq!(now.len(), 1, "{now:?}");
    assert!(!old.contains(&now[0]), "reinstating makes their account");
    assert!(
        rauthy
            .users()?
            .iter()
            .any(|user| user["id"] == now[0] && user["email"] == "cal@example.test")
    );
    Ok(())
}

#[tokio::test]
async fn an_account_a_retired_person_still_holds_is_never_taken_by_the_repair() -> TestResult {
    let (mut service, rauthy, ada) = table_with_mail(false).await?;
    let bea = add(&service, &ada, "Bea", "bea@example.test").await?;
    let cal = add(&service, &ada, "Cal", "cal@example.test").await?;
    let bea_logins = logins(&service, &ada, &bea).await?;
    let cal_logins = logins(&service, &ada, &cal).await?;
    transition(&service, &ada, &bea, "retire").await?;
    // Cal's account is gone, and the address Lys holds for Cal is the one
    // whose account the retired Bea still holds.
    rauthy.forget(cal["subject"].as_str().ok_or("Cal's account")?)?;
    let foundings = service.dir.path().join("foundings.json");
    let cal_id = cal["person"].as_str().ok_or("Cal")?.to_owned();
    service
        .restart_between(None, move |_config| {
            let mut held: Value = serde_json::from_slice(&std::fs::read(&foundings)?)?;
            for row in held["foundings"]
                .as_object_mut()
                .ok_or("the foundings")?
                .values_mut()
            {
                if row["founder"] == cal_id.as_str() {
                    row["email"] = json!("bea@example.test");
                }
            }
            std::fs::write(&foundings, serde_json::to_vec(&held)?)?;
            Ok(())
        })
        .await?;

    assert_eq!(
        logins(&service, &ada, &bea).await?,
        bea_logins,
        "the retired Bea keeps her login"
    );
    assert_eq!(
        logins(&service, &ada, &cal).await?,
        cal_logins,
        "Cal waits for an administrator"
    );
    Ok(())
}

#[tokio::test]
async fn the_administrator_with_two_addresses_held_still_signs_in() -> TestResult {
    let (mut service, _rauthy, ada) = table_with_mail(false).await?;
    let (status, me) = service.get("/me", Some(&ada)).await?;
    assert_eq!(status, 200, "{me}");
    let administrator = me["person"]["id"]
        .as_str()
        .ok_or("the administrator")?
        .to_owned();
    let (_, record) = service
        .get(&format!("/identities/{administrator}"), Some(&ada))
        .await?;
    let subject = record["logins"][0]["subject"]
        .as_str()
        .ok_or("their login")?
        .to_owned();
    let foundings = service.dir.path().join("foundings.json");
    let founder = administrator.clone();
    service
        .restart_between(None, move |_config| {
            let mut held: Value = serde_json::from_slice(&std::fs::read(&foundings)?)?;
            let rows = held["foundings"].as_object_mut().ok_or("the foundings")?;
            let template = rows.values().next().cloned().ok_or("a founding row")?;
            for (key, email) in [
                ("first", "ada@example.test"),
                ("second", "ada.other@example.test"),
            ] {
                let mut row = template.clone();
                row["founder"] = json!(founder);
                row["email"] = json!(email);
                row["withdrawn"] = json!(false);
                rows.insert(format!("administrator-{key}"), row);
            }
            std::fs::write(&foundings, serde_json::to_vec(&held)?)?;
            Ok(())
        })
        .await?;

    let name = me["person"]["display_name"].as_str().ok_or("their name")?;
    assert_eq!(
        state_of(&health(&service, &ada).await?, name)["state"],
        "several_addresses",
        "the administrator's record holds two addresses"
    );
    let session = service
        .sign_in(Login {
            subject,
            email: "ada@example.test".to_owned(),
        })
        .await?;
    assert_eq!(
        service.get("/me", Some(&session)).await?.1["person"]["id"],
        json!(administrator)
    );
    Ok(())
}

#[tokio::test]
async fn october_26_annabel_inside_dean_is_refused_then_one_move_and_one_address_put_everyone_right()
-> TestResult {
    let (mut service, rauthy, ada) = table_with_mail(false).await?;
    let dean = add(&service, &ada, "Dean Whiting", "dean@example.test").await?;
    let annabel = add(&service, &ada, "Annabel", "annabel@example.test").await?;
    let dean_subject = dean["subject"].as_str().ok_or("Dean's account")?.to_owned();
    let dean_id = dean["person"].as_str().ok_or("Dean")?.to_owned();
    let annabel_id = annabel["person"].as_str().ok_or("Annabel")?.to_owned();
    // The live records of 26 October: Annabel's own account gone from the
    // issuer, Dean's account rewritten to her address, and Lys holding both
    // addresses for Dean's record after the second admission.
    rauthy.forget(annabel["subject"].as_str().ok_or("Annabel's account")?)?;
    historical_email(&rauthy, &dean_subject, "annabel@example.test", true).await?;
    let foundings = service.dir.path().join("foundings.json");
    let dean_for_both = dean_id.clone();
    service
        .restart_between(None, move |_config| {
            let mut held: Value = serde_json::from_slice(&std::fs::read(&foundings)?)?;
            let rows = held["foundings"].as_object_mut().ok_or("the foundings")?;
            let mut second = rows
                .values()
                .find(|row| row["founder"] == dean_for_both.as_str())
                .cloned()
                .ok_or("Dean's admission")?;
            second["email"] = json!("annabel@example.test");
            rows.insert("second-admission-of-annabel-to-dean".to_owned(), second);
            std::fs::write(&foundings, serde_json::to_vec(&held)?)?;
            Ok(())
        })
        .await?;

    let judged = health(&service, &ada).await?;
    assert_eq!(
        state_of(&judged, "Dean Whiting"),
        json!({"state": "several_addresses", "addresses": ["annabel@example.test", "dean@example.test"]})
    );
    assert_eq!(
        state_of(&judged, "Annabel"),
        json!({"state": "address_taken", "holder": "Dean Whiting"})
    );
    // Nothing was handed from one person to another at start.
    assert_eq!(
        logins(&service, &ada, &dean).await?,
        vec![dean_subject.clone()]
    );
    // She can no longer become Dean.
    let refused = service
        .sign_in(Login {
            subject: dean_subject.clone(),
            email: "annabel@example.test".to_owned(),
        })
        .await
        .err()
        .ok_or("signing in as Dean's record must be refused")?;
    assert!(refused.to_string().contains("SignInNotTheirs"), "{refused}");

    // One move gives her account to her own record.
    let (status, moved) = service
        .post(
            &format!("/people/{dean_id}/logins/move"),
            Some(&ada),
            &json!({"operation": OperationId::generate()?.to_string(), "issuer": service.issuer.issuer(), "subject": dean_subject, "target": {"person": annabel_id}}),
        )
        .await?;
    assert_eq!(status, 200, "{moved}");
    let session = service
        .sign_in(Login {
            subject: dean_subject.clone(),
            email: "annabel@example.test".to_owned(),
        })
        .await?;
    assert_eq!(
        service.get("/me", Some(&session)).await?.1["person"]["id"],
        json!(annabel_id)
    );
    assert_eq!(
        state_of(&health(&service, &ada).await?, "Annabel"),
        json!({"state": "ready"})
    );

    // One address set for Dean: Lys makes his account under it.
    let (status, set) = service
        .post(
            &format!("/directory/people/{dean_id}/account/email"),
            Some(&ada),
            &json!({"email": "dean@example.test"}),
        )
        .await?;
    assert_eq!(status, 200, "{set}");
    assert_eq!(set["email"], "dean@example.test", "{set}");
    let renewed = logins(&service, &ada, &dean).await?;
    assert_eq!(renewed.len(), 1, "{renewed:?}");
    assert_ne!(renewed[0], dean_subject);
    assert!(
        rauthy
            .users()?
            .iter()
            .any(|user| user["id"] == renewed[0] && user["email"] == "dean@example.test")
    );
    let judged = health(&service, &ada).await?;
    assert_eq!(state_of(&judged, "Dean Whiting"), json!({"state": "ready"}));
    assert_eq!(state_of(&judged, "Annabel"), json!({"state": "ready"}));
    Ok(())
}

#[tokio::test]
async fn an_address_another_person_holds_is_refused_for_someone_with_no_account() -> TestResult {
    let (service, rauthy, ada) = table_with_mail(false).await?;
    let bea = add(&service, &ada, "Bea", "bea@example.test").await?;
    let cal = add(&service, &ada, "Cal", "cal@example.test").await?;
    rauthy.forget(cal["subject"].as_str().ok_or("Cal's account")?)?;
    let cal_id = cal["person"].as_str().ok_or("Cal")?;
    let accounts = rauthy.users()?;
    let size = service.log_size().await?;
    let (status, refused) = service
        .post(
            &format!("/directory/people/{cal_id}/account/email"),
            Some(&ada),
            &json!({"email": "BEA@example.test"}),
        )
        .await?;
    assert_eq!(status, 409, "{refused}");
    assert_eq!(refused["refusal"], "EmailHeldByAnother");
    assert_eq!(refused["person"], bea["person"]);
    assert_eq!(rauthy.users()?, accounts);
    assert_eq!(service.log_size().await?, size);
    Ok(())
}
