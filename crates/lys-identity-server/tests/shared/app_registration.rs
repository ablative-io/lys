//! Registrar authority and registration-selected service accounts.

use super::*;

#[tokio::test]
async fn a_registrar_registers_apps_and_a_person_or_a_wrong_credential_does_not() -> TestResult {
    let (service, seeded) = identity_contract::harness::Service::start_judging(
        r#"{"version":1,"relations":{"editor":["view","edit"],"viewer":["view"]}}"#,
        None,
        |config| {
            Ok(lys_identity_server::dev_seed::seed_configured(
                config,
                [ADMINISTRATOR, BEA],
            )?)
        },
    )
    .await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    let account = op()?;
    let made = json!({"operation": account, "name": "registrar fixture"});
    ok(post(&service, "/service-accounts", Auth::Cookie(&admin), &made).await?)?;
    let registrar = json!({"operation": op()?, "service_account": account});
    let issued = ok(post(
        &service,
        "/apps/registrars",
        Auth::Cookie(&admin),
        &registrar,
    )
    .await?)?;
    let credential = issued["credential"]
        .as_str()
        .ok_or("no credential")?
        .to_owned();

    let probe = registration(NOTES, &workspace_schema(NOTES))?;
    let missing = post(&service, "/apps", Auth::Bearer(&credential), &probe).await?;
    assert_eq!(missing.1["refusal"], "NotHeld", "{}", missing.1);
    let owner = seeded.people[0].id.to_string();
    let root = ok(post(&service, "/grants/roots", Auth::Cookie(&admin), &json!({
        "operation":op()?,"route":"api","holder":owner,"resource":{"kind":"directory","id":"apps"},"relation":"editor",
        "pass_on":{"kind":"to","actions":["view","edit"],"recipients":["service_account"]},"window":{"starts_at":0,"ends_at":null}
    })).await?)?;
    ok(post(&service, "/grants", Auth::Cookie(&admin), &json!({
        "operation":op()?,"route":"api","source":root["grant"],"recipient":account,"responsible":owner,
        "resource":{"kind":"directory","id":"apps"},"relation":"editor","pass_on":{"kind":"use_only"},"window":{"starts_at":0,"ends_at":null}
    })).await?)?;

    let body = registration(NOTES, &workspace_schema(NOTES))?;
    let pending = ok(post(&service, "/apps", Auth::Bearer(&credential), &body).await?)?;
    assert_eq!(pending["state"], "pending");
    assert_eq!(
        pending["registered_by"]["kind"], "service_account",
        "{pending}"
    );
    assert_eq!(pending["service_account"], account.as_str());

    let mut refusals = 0;
    let bea = service.sign_in(login(BEA)).await?;
    let other = registration(FILES, &workspace_schema(FILES))?;
    refused(
        &post(&service, "/apps", Auth::Cookie(&bea), &other).await?,
        403,
        "NotAdmitted",
    )?;
    refusals += 1;
    let wrong = format!("lys-registrar.{account}.{}", "f".repeat(64));
    refused(
        &post(&service, "/apps", Auth::Bearer(&wrong), &other).await?,
        401,
        "credential_refused",
    )?;
    refusals += 1;
    let listed = ok(get(&service, "/apps", Auth::Cookie(&bea)).await?)?;
    let names: Vec<&str> = listed["apps"]
        .as_array()
        .ok_or("no apps")?
        .iter()
        .filter_map(|app| app["id"].as_str())
        .collect();
    assert_eq!(names, vec!["lys"], "a person sees approved apps only");
    refused(
        &get(&service, &format!("/apps/{NOTES}"), Auth::Cookie(&bea)).await?,
        404,
        "app_unknown",
    )?;
    refusals += 1;
    assert_eq!(refusals, 3);
    Ok(())
}

#[tokio::test]
async fn approval_binds_the_service_account_the_registration_names() -> TestResult {
    let (service, _) = seeded().await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    let account = op()?;
    let made = json!({"operation": account, "name": "app fixture"});
    ok(post(&service, "/service-accounts", Auth::Cookie(&admin), &made).await?)?;
    let mut body = registration(NOTES, &workspace_schema(NOTES))?;
    body["service_account"] = json!(account);
    ok(post(&service, "/apps", Auth::Cookie(&admin), &body).await?)?;
    let approval = ok(post(
        &service,
        &format!("/apps/{NOTES}/approve"),
        Auth::Cookie(&admin),
        &json!({"operation": op()?}),
    )
    .await?)?;
    assert_eq!(
        approval["app"]["service_account"],
        account.as_str(),
        "{approval}"
    );
    assert!(approval["client"].is_null());
    let credential = identity_contract::app_custody::credential(NOTES);
    let me = ok(get(&service, "/apps/me", Auth::Bearer(&credential)).await?)?;
    assert_eq!(me["service_account"], account.as_str());
    Ok(())
}
