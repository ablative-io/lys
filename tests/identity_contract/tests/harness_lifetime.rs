//! Dropping a fixture releases every listener, including during unwinding.

use std::error::Error;
use std::net::TcpListener;

use identity_contract::harness::{GRANT_MODEL, Service};

type Outcome = Result<(), Box<dyn Error>>;

async fn fixture() -> Result<Service, Box<dyn Error>> {
    Ok(Service::start_adjusted(
        GRANT_MODEL,
        None,
        None,
        None,
        |config| {
            config.requests_dir = None;
            config.certificates_dir = None;
            config.network_file = None;
            config.roles_file = None;
            config.provisioning_file = None;
            config.homes_dir = None;
            config.runtime_dir = None;
            config.service_accounts_dir = None;
            config.teams_dir = None;
            config.stops_dir = None;
            config.budgets_dir = None;
            config.policies_dir = None;
            config.goals_dir = None;
            config.reviews_dir = None;
        },
        |_| Ok(()),
    )
    .await?
    .0)
}

fn listeners(service: &Service) -> Result<[String; 3], Box<dyn Error>> {
    let address = |url: &str| -> Result<String, Box<dyn Error>> {
        Ok(url
            .strip_prefix("http://")
            .ok_or("fixture is not HTTP")?
            .split('/')
            .next()
            .ok_or("fixture has no listener address")?
            .to_owned())
    };
    Ok([
        address(&service.base)?,
        address(service.issuer.loopback())?,
        address(service.issuer.provider_base())?,
    ])
}

fn released(addresses: &[String]) -> Outcome {
    let listeners = addresses
        .iter()
        .map(|address| {
            TcpListener::bind(address)
                .map_err(|error| format!("fixture_listener_not_released {address}: {error}"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    assert_eq!(listeners.len(), addresses.len());
    Ok(())
}

#[tokio::test]
async fn dropping_service_releases_all_three_listener_ports() -> Outcome {
    let service = fixture().await?;
    let addresses = listeners(&service)?;
    drop(service);
    released(&addresses)
}

#[tokio::test]
async fn forced_failure_releases_all_three_listener_ports() -> Outcome {
    let service = fixture().await?;
    let addresses = listeners(&service)?;
    let failed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
        let fixture = service;
        assert!(!fixture.base.is_empty());
        panic!("forced_fixture_failure");
    }));
    assert!(failed.is_err());
    released(&addresses)
}

#[tokio::test]
async fn restart_releases_the_previous_service_listener() -> Outcome {
    let mut service = fixture().await?;
    let addresses = listeners(&service)?;
    service.restart().await?;
    released(&addresses[..1])?;
    let current = listeners(&service)?;
    drop(service);
    released(&current)
}
