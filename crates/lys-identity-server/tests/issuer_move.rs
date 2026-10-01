#![cfg(test)]
//! An upgrade that moves the sign-in service to a new issuer hands the
//! service the earlier one. The service records the move once at start, and
//! the administrator and every bound login sign in under the new issuer as
//! the people they were; a token from the earlier issuer resolves to nobody.

use std::error::Error;
use std::os::unix::fs::PermissionsExt;

use identity_contract::apps::login;
use identity_contract::fake_issuer::FakeIssuer;
use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_identity::{Actor, AuthMethod, LoginBinding, OperationId, Profile, Provenance};
use lys_identity_server::config::ConfiguredLogin;
use lys_identity_server::routes::open_directory;

type TestResult = Result<(), Box<dyn Error>>;

/// A service whose administrator was set up under the fake issuer it
/// started with, signed in once as them.
async fn set_up() -> Result<Service, Box<dyn Error>> {
    let (service, ()) = Service::start_with(|config| {
        let mut directory = open_directory(config)?;
        let administrator = Actor::new(
            LoginBinding::new(&config.issuer, ADMINISTRATOR)?,
            Provenance::new(AuthMethod::Oidc, 1),
        );
        directory.setup_person(
            administrator,
            OperationId::generate()?,
            Profile::new("Tom")?,
            1,
        )?;
        Ok(())
    })
    .await?;
    let cookie = service.sign_in(login(ADMINISTRATOR)).await?;
    assert_eq!(
        service.get("/directory/people", Some(&cookie)).await?.0,
        200
    );
    Ok(service)
}

/// Move the service to a new issuer signing with the same key, as an upgrade
/// that changes the sign-in service's public address does, answering the
/// earlier issuer.
async fn move_issuer(service: &mut Service) -> Result<FakeIssuer, Box<dyn Error>> {
    let earlier_name = service.issuer.issuer().to_owned();
    let moved = FakeIssuer::start(&service.dir.path().join("issuer.key")).await?;
    let (name, api) = (moved.issuer().to_owned(), moved.api().to_owned());
    let earlier = std::mem::replace(&mut service.issuer, moved);
    service
        .restart_adjusted(|config| {
            config.issuer.clone_from(&name);
            config.sign_in_api = Some(api);
            config.administrator = Some(ConfiguredLogin {
                issuer: name.clone(),
                subject: ADMINISTRATOR.to_owned(),
            });
            config.link_audit_source.issuer.clone_from(&name);
            config.issuer_moved_from = Some(earlier_name);
        })
        .await?;
    Ok(earlier)
}

#[tokio::test]
async fn after_the_issuer_moves_the_administrator_is_still_the_administrator() -> TestResult {
    let mut service = set_up().await?;
    let before = service.log_size().await?;
    move_issuer(&mut service).await?;
    assert_eq!(
        service.log_size().await?,
        before + 1,
        "the move is one leaf"
    );
    let cookie = service.sign_in(login(ADMINISTRATOR)).await?;
    let (status, people) = service.get("/directory/people", Some(&cookie)).await?;
    assert_eq!(status, 200, "{people}");
    let (status, me) = service.get("/me", Some(&cookie)).await?;
    assert_eq!(
        status, 200,
        "the moved login resolves to their person: {me}"
    );
    assert!(me.to_string().contains("Tom"), "{me}");
    let (status, leaf) = service.get(&format!("/receipts/{before}"), None).await?;
    assert_eq!(status, 409, "{leaf}");
    assert_eq!(leaf["refusal"], "InstallEntry", "{leaf}");
    Ok(())
}

#[tokio::test]
async fn starting_again_after_the_move_records_nothing_more() -> TestResult {
    let mut service = set_up().await?;
    move_issuer(&mut service).await?;
    let after = service.log_size().await?;
    service.restart().await?;
    service.restart().await?;
    assert_eq!(service.log_size().await?, after);
    let cookie = service.sign_in(login(ADMINISTRATOR)).await?;
    assert_eq!(
        service.get("/directory/people", Some(&cookie)).await?.0,
        200
    );
    Ok(())
}

#[tokio::test]
async fn after_the_move_a_token_from_the_earlier_issuer_signs_nobody_in() -> TestResult {
    let mut service = set_up().await?;
    let earlier = move_issuer(&mut service).await?;
    service.issuer = earlier;
    assert!(
        service.sign_in(login(ADMINISTRATOR)).await.is_err(),
        "a token the earlier issuer signed is refused"
    );
    Ok(())
}

/// An install from 29 September holds the directory loader's service
/// account, created at start under the administrator's login at the earlier
/// issuer. After the move the service starts, the administrator is still
/// the administrator, and the account is still held, once.
#[tokio::test]
async fn an_install_holding_the_loaders_service_account_starts_after_the_move() -> TestResult {
    const MODEL: &str = r#"{"version":1,"relations":{"owner":["view","edit","grant"],"editor":["view","edit"],"viewer":["view"]}}"#;
    const ACCOUNT: &str = "op-91919191919191919191919191919191";
    let (mut service, ()) = Service::start_judging(MODEL, None, |config| {
        lys_identity_server::dev_seed::seed_configured(config, [ADMINISTRATOR, "second-person"])?;
        Ok(())
    })
    .await?;
    let credential = service.dir.path().join("loader.credential");
    std::fs::write(
        &credential,
        format!("lys-registrar.{ACCOUNT}.{}", "ab".repeat(32)),
    )?;
    std::fs::set_permissions(&credential, std::fs::Permissions::from_mode(0o600))?;
    service
        .restart_adjusted(|config| config.import_credential_file = Some(credential))
        .await?;
    let cookie = service.sign_in(login(ADMINISTRATOR)).await?;
    let (status, before) = service.get("/service-accounts", Some(&cookie)).await?;
    assert_eq!(status, 200, "{before}");
    assert!(before.to_string().contains(ACCOUNT), "{before}");
    move_issuer(&mut service).await?;
    service.restart().await?;
    let cookie = service.sign_in(login(ADMINISTRATOR)).await?;
    let (status, people) = service.get("/directory/people", Some(&cookie)).await?;
    assert_eq!(status, 200, "{people}");
    let (status, after) = service.get("/service-accounts", Some(&cookie)).await?;
    assert_eq!(status, 200, "{after}");
    assert_eq!(
        after.to_string().matches(ACCOUNT).count(),
        before.to_string().matches(ACCOUNT).count()
    );
    Ok(())
}
