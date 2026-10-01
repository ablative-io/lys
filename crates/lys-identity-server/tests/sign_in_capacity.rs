#![cfg(test)]
//! Abandoning provider consent must not exhaust every client's sign-in
//! capacity. Sixteen outstanding flows are one client's whole share.
use std::error::Error;
use std::net::IpAddr;

use identity_contract::fake_issuer::{Account, Login};
use identity_contract::harness::Service;
use lys_identity_server::oidc::Oidc;
use lys_identity_server::sign_in::{Attempt, IssuerSignIn};

#[tokio::test]
async fn abandoned_provider_flows_are_bounded_per_address_and_leave_other_clients_room()
-> Result<(), Box<dyn Error>> {
    let (service, config) = Service::start_with(|config| Ok(config.clone())).await?;
    service.issuer.hold(Account {
        login: Login {
            subject: "quiet-person".to_owned(),
            email: "quiet@example.test".to_owned(),
        },
        password: "Correct-Person-Password1".to_owned(),
        second_factor: false,
    })?;
    let oidc = Oidc::discover(&config).await?;
    let issuer = IssuerSignIn::configured(&config)?;
    let noisy: IpAddr = "192.0.2.10".parse()?;
    for _ in 0..16 {
        issuer
            .begin_provider(&oidc, "google-provider", noisy, [7; 32])
            .await?;
    }
    let refused = issuer
        .begin_provider(&oidc, "google-provider", noisy, [7; 32])
        .await
        .err()
        .map(|error| error.name());
    assert_eq!(refused.as_deref(), Some("SignInThrottled"));
    let actor = issuer
        .password(
            &oidc,
            &Attempt {
                email: "quiet@example.test",
                password: "Correct-Person-Password1",
                address: "198.51.100.10".parse()?,
            },
        )
        .await?;
    assert_eq!(actor.binding().subject(), "quiet-person");
    Ok(())
}

#[tokio::test]
async fn a_full_provider_pool_cannot_refuse_a_password_sign_in() -> Result<(), Box<dyn Error>> {
    let (service, config) = Service::start_with(|config| Ok(config.clone())).await?;
    service.issuer.hold(Account {
        login: Login {
            subject: "quiet-person".into(),
            email: "quiet@example.test".into(),
        },
        password: "Correct-Person-Password1".into(),
        second_factor: false,
    })?;
    let oidc = Oidc::discover(&config).await?;
    let issuer = IssuerSignIn::configured(&config)?;
    for address in 0..64u8 {
        for _ in 0..16 {
            oidc.begin_provider(IpAddr::V4(std::net::Ipv4Addr::new(192, 0, 2, address)))?;
        }
    }
    assert!(oidc.begin_provider("203.0.113.1".parse()?).is_err());
    let actor = issuer
        .password(
            &oidc,
            &Attempt {
                email: "quiet@example.test",
                password: "Correct-Person-Password1",
                address: "203.0.113.1".parse()?,
            },
        )
        .await?;
    assert_eq!(actor.binding().subject(), "quiet-person");
    Ok(())
}
