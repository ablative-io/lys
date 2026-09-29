#![cfg(test)]
use super::resolve;
use axum::http::HeaderMap;
use std::error::Error;
use std::net::IpAddr;

type TestResult = Result<(), Box<dyn Error>>;

#[test]
fn only_explicitly_trusted_peers_can_supply_a_client_address() -> TestResult {
    let peer: IpAddr = "192.0.2.1".parse()?;
    let mut headers = HeaderMap::new();
    headers.insert("x-forwarded-for", "198.51.100.7".parse()?);
    assert_eq!(resolve(peer, &headers, &[])?, peer);
    assert_eq!(
        resolve(peer, &headers, &[peer])?,
        "198.51.100.7".parse::<IpAddr>()?
    );
    headers.insert("x-forwarded-for", "not-an-ip".parse()?);
    assert_eq!(resolve(peer, &headers, &[])?, peer);
    assert!(resolve(peer, &headers, &[peer]).is_err());
    assert!(resolve(peer, &HeaderMap::new(), &[peer]).is_err());
    Ok(())
}

#[test]
fn a_clients_spoofed_left_entries_cannot_override_the_address_a_proxy_appends() -> TestResult {
    let peer: IpAddr = "192.0.2.1".parse()?;
    let second: IpAddr = "192.0.2.2".parse()?;
    let mut headers = HeaderMap::new();
    headers.append("x-forwarded-for", "garbage, 203.0.113.99".parse()?);
    headers.append("x-forwarded-for", "198.51.100.7, 192.0.2.2".parse()?);
    assert_eq!(
        resolve(peer, &headers, &[peer, second])?,
        "198.51.100.7".parse::<IpAddr>()?
    );
    assert_eq!(resolve(peer, &headers, &[peer])?, second);
    Ok(())
}

#[test]
fn ipv6_resolution_is_grouped_by_prefix_after_the_trust_walk() -> TestResult {
    let peer: IpAddr = "::1".parse()?;
    let mut headers = HeaderMap::new();
    headers.insert("x-forwarded-for", "2001:db8:1:2::abcd".parse()?);
    assert_eq!(
        crate::sign_in_flights::address_key(resolve(peer, &headers, &[peer])?),
        "2001:db8:1:2::".parse::<IpAddr>()?
    );
    Ok(())
}
