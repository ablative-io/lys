#![cfg(test)]
//! A configured front proxy's clients have independent admission buckets;
//! leftmost values supplied by a client cannot override the appended peer.
use identity_contract::fake_issuer::{Account, Login};
use identity_contract::harness::{GRANT_MODEL, Service};
use std::error::Error;

#[tokio::test]
async fn a_trusted_proxy_does_not_put_all_password_clients_in_one_bucket()
-> Result<(), Box<dyn Error>> {
    let (service, ()) = Service::start_adjusted(
        GRANT_MODEL,
        None,
        None,
        None,
        |config| config.trusted_proxies = vec![std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST)],
        |_| Ok(()),
    )
    .await?;
    service.issuer.hold(Account {
        login: Login {
            subject: "person".into(),
            email: "person@example.test".into(),
        },
        password: "Correct-Password-For-Test1".into(),
        second_factor: false,
    });
    let client = reqwest::Client::new();
    let request = |forwarded: &str| {
        client
            .post(format!("{}/sign-in", service.base))
            .header("x-forwarded-for", forwarded)
            .header("content-type", "application/json")
            .body(r#"{"email":"person@example.test","password":"Correct-Password-For-Test1"}"#)
    };
    for _ in 0..10 {
        assert_eq!(request("192.0.2.1").send().await?.status(), 200);
    }
    assert_eq!(
        request("203.0.113.99, 192.0.2.1").send().await?.status(),
        429,
        "spoofed left entry cannot escape the noisy client's limit"
    );
    assert_eq!(
        request("192.0.2.2").send().await?.status(),
        200,
        "another client behind the same proxy still signs in"
    );
    Ok(())
}
