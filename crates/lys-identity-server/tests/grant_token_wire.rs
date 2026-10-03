//! The personal administration routes enforce the same responsible-person boundary.
use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_identity::OperationId;
use lys_identity_server::dev_seed::seed_configured;
use serde_json::{Value, json};
use std::error::Error;

type TestResult = Result<(), Box<dyn Error>>;

fn login(subject: &str) -> Login {
    Login {
        subject: subject.to_owned(),
        email: "shared@example.test".to_owned(),
    }
}

fn root(holder: &str) -> Result<Value, Box<dyn Error>> {
    Ok(
        json!({"operation":OperationId::generate()?.to_string(),"route":"api","holder":holder,"resource":{"kind":"doc","id":"one"},"relation":"alpha","pass_on":{"kind":"use_only"},"window":{"starts_at":0,"ends_at":null}}),
    )
}

fn refused(answer: &(u16, Value), status: u16, name: &str) {
    assert_eq!(answer.0, status);
    assert_eq!(answer.1["refusal"], name);
}

#[tokio::test]
async fn grant_token_routes_refuse_other_administrators_and_durable_failure() -> TestResult {
    let (service, (seeded, file)) = Service::start_with(|config| {
        Ok((
            seed_configured(config, [ADMINISTRATOR, "owner"])?,
            config.log_dir.with_file_name("grant-tokens.json"),
        ))
    })
    .await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    let owner = service.sign_in(login("owner")).await?;
    let answer = service
        .post(
            "/grants/roots",
            Some(&admin),
            &root(&seeded.people[1].id.to_string())?,
        )
        .await?;
    assert_eq!(answer.0, 200);
    let grant = answer.1["grant"].as_str().ok_or("missing grant")?;
    let path = format!("/grants/{grant}/tokens");
    let expiry = lys_identity_server::session::now() + 300;
    refused(
        &service
            .post(&path, Some(&admin), &json!({"expires_at":expiry}))
            .await?,
        403,
        "GrantTokenResponsibleRequired",
    );
    refused(
        &service
            .post(&path, Some(&owner), &json!({"expires_at":0}))
            .await?,
        400,
        "GrantTokenExpiryInvalid",
    );
    let issued = service
        .post(&path, Some(&owner), &json!({"expires_at":expiry}))
        .await?;
    assert_eq!(issued.0, 200);
    let id = issued.1["id"].as_str().ok_or("missing token id")?;
    let revoke = format!("{path}/{id}/revoke");
    refused(
        &service.post(&revoke, Some(&admin), &json!({})).await?,
        403,
        "GrantTokenResponsibleRequired",
    );
    assert_eq!(
        service.post(&revoke, Some(&owner), &json!({})).await?.0,
        200
    );
    refused(
        &service
            .post(&format!("{path}/unknown/revoke"), Some(&owner), &json!({}))
            .await?,
        401,
        "GrantTokenUnknown",
    );
    std::fs::remove_file(&file)?;
    std::fs::create_dir(&file)?;
    refused(
        &service
            .post(&path, Some(&owner), &json!({"expires_at":expiry}))
            .await?,
        503,
        "GrantTokenUnavailable",
    );
    Ok(())
}

#[tokio::test]
async fn grant_token_routes_issue_over_two_thousand_retained_records() -> TestResult {
    let (service, seeded) = Service::start_with(|config| {
        let seeded = seed_configured(config, [ADMINISTRATOR, "owner"])?;
        let grant = lys_identity::grants::GrantId::from_bytes([1; 16]).to_string();
        // Revoked but not yet expired, so opening the table keeps every one.
        let ends = lys_identity_server::session::now() + 86_400;
        let entries: serde_json::Map<String, Value> = (0..2000u32)
            .map(|n| {
                (
                    format!("{n:064x}"),
                    json!({"grant":grant,"expires_at":ends,"revoked":true}),
                )
            })
            .collect();
        std::fs::write(
            config.log_dir.with_file_name("grant-tokens.json"),
            serde_json::to_vec(&json!({"format":"lys-grant-tokens/v1","tokens":entries}))?,
        )?;
        Ok(seeded)
    })
    .await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    let answer = service
        .post(
            "/grants/roots",
            Some(&admin),
            &root(&seeded.people[0].id.to_string())?,
        )
        .await?;
    assert_eq!(answer.0, 200);
    let grant = answer.1["grant"].as_str().ok_or("missing grant")?;
    let issued = service
        .post(
            &format!("/grants/{grant}/tokens"),
            Some(&admin),
            &json!({"expires_at":lys_identity_server::session::now()+300}),
        )
        .await?;
    assert_eq!(issued.0, 200, "{}", issued.1);
    assert!(issued.1["id"].as_str().is_some(), "{}", issued.1);
    assert!(issued.1["token"].as_str().is_some(), "{}", issued.1);
    Ok(())
}
