//! Registrar authority and registration-selected service accounts.

use super::*;

#[tokio::test]
async fn a_registrar_registers_apps_and_a_person_or_a_wrong_credential_does_not() -> TestResult {
    let (service, owner) = authority_table(
        r#"{"version":1,"relations":{"editor":["view","edit"],"viewer":["view"]}}"#,
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
    ok(post(
        &service,
        &format!("/service-accounts/{account}/retire"),
        Auth::Cookie(&admin),
        &json!({"operation": op()?}),
    )
    .await?)?;
    refused(
        &get(&service, "/apps", Auth::Bearer(&credential)).await?,
        401,
        "credential_refused",
    )?;
    Ok(())
}

#[tokio::test]
async fn approval_binds_the_service_account_the_registration_names() -> TestResult {
    let (service, _) = authority_table(identity_contract::harness::GRANT_MODEL).await?;
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
        &json!({"operation": op()?, "redirects": body["redirects"], "profile": false}),
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
    ok(post(
        &service,
        &format!("/service-accounts/{account}/retire"),
        Auth::Cookie(&admin),
        &json!({"operation": op()?}),
    )
    .await?)?;
    refused(
        &get(&service, "/apps/me", Auth::Bearer(&credential)).await?,
        401,
        "credential_refused",
    )?;
    Ok(())
}

async fn authority_table(
    model: &str,
) -> Result<(identity_contract::harness::Service, String), Box<dyn Error>> {
    use lys_core::Ed25519Identity;
    use lys_identity::{
        Actor, AuthMethod, IdentityId, LoginBinding, OperationId, Profile, Provenance, Transition,
    };
    use lys_identity_server::routes::open_directory;
    use std::sync::Arc;
    let broker = identity_contract::app_custody::start().await?;
    identity_contract::harness::Service::start_adjusted(
        model,
        None,
        None,
        None,
        move |config| {
            config.requests_dir = None;
            config.certificates_dir = None;
            config.network_file = None;
            config.roles_file = None;
            config.provisioning_file = None;
            config.runtime_dir = None;
            config.teams_dir = None;
            config.stops_dir = None;
            config.budgets_dir = None;
            config.policies_dir = None;
            config.goals_dir = None;
            config.reviews_dir = None;
            config.secrets = Some(lys_identity_server::secrets_api::SecretsSettings {
                broker,
                service: "identity".to_owned(),
                service_key_file: config.event_key_file.clone(),
            });
        },
        |config| {
            let key = Arc::new(Ed25519Identity::load(&config.event_key_file)?);
            let actor = Actor::new(
                LoginBinding::new(&config.issuer, ADMINISTRATOR)?,
                Provenance::new(AuthMethod::Oidc, 1),
            );
            let mut directory = open_directory(config)?;
            let (owner, _) = directory.setup_person(
                actor.clone(),
                OperationId::generate()?,
                Profile::new("Owner")?,
                1,
            )?;
            let (bea, _) = directory.register_person(
                actor.clone(),
                OperationId::generate()?,
                Profile::new("Person")?,
                2,
            )?;
            directory.bind_login(
                actor.clone(),
                OperationId::generate()?,
                bea,
                LoginBinding::new(&config.issuer, BEA)?,
                3,
            )?;
            directory.transition(
                actor,
                OperationId::generate()?,
                IdentityId::Person(bea),
                Transition::Activate,
                "",
                4,
            )?;
            drop(
                lys_identity_server::service_accounts_store::ServiceAccountStore::open(
                    config
                        .service_accounts_dir
                        .as_deref()
                        .ok_or("accounts directory missing")?,
                    Arc::clone(&key),
                )?,
            );
            drop(
                lys_identity_server::configuration_store::ConfigurationStore::open(
                    &config.log_dir.with_file_name("organisation"),
                    Arc::clone(&key),
                )?,
            );
            drop(lys_identity_server::runner_acts::ActStore::open(
                &config.log_dir.with_file_name("runner-acts"),
                Arc::clone(&key),
            )?);
            drop(lys_identity::start::LaunchRecords::open(
                &config.log_dir.with_file_name("launch-records"),
                Ed25519Identity::load(&config.event_key_file)?,
            )?);
            drop(lys_identity_server::apps_api::opened(
                config,
                Arc::clone(&key),
                &|_| {},
            )?);
            Ok(owner.to_string())
        },
    )
    .await
}
