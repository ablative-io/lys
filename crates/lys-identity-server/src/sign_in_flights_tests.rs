#![cfg(test)]
//! Use explicit instants, never sleeping, to check boundary expiry and bounded
//! admission independently of either issuer's network or rate policy.
use super::{Flights, MAX_AGE, PER_ADDRESS};
use std::error::Error;
use std::net::IpAddr;
use std::time::{Duration, Instant};

type TestResult = Result<(), Box<dyn Error>>;

#[test]
fn expired_capacity_is_reclaimed_and_expired_callbacks_are_refused() -> TestResult {
    let address: IpAddr = "192.0.2.1".parse()?;
    let now = Instant::now();
    let mut flights = Flights::default();
    for n in 0..PER_ADDRESS {
        flights.insert(n.to_string(), n, address, now)?;
    }
    assert!(flights.insert("overflow".into(), 99, address, now).is_err());
    let expired = now + MAX_AGE;
    assert!(flights.take("0", expired).is_err());
    flights.insert("new".into(), 100, address, expired)?;
    assert!(flights.take("1", expired).is_err());
    assert_eq!(flights.take("new", expired)?, 100);
    assert!(flights.take("new", expired).is_err());
    Ok(())
}

#[test]
fn unexpired_flows_are_not_evicted_and_other_addresses_keep_their_share() -> TestResult {
    let now = Instant::now();
    let address: IpAddr = "192.0.2.1".parse()?;
    let mut flights = Flights::default();
    for n in 0..PER_ADDRESS {
        flights.insert(n.to_string(), n, address, now)?;
    }
    let before = now + MAX_AGE - Duration::from_nanos(1);
    assert!(
        flights
            .insert("overflow".into(), 99, address, before)
            .is_err()
    );
    flights.insert("quiet".into(), 100, "192.0.2.2".parse()?, before)?;
    assert_eq!(flights.take("0", before)?, 0);
    assert_eq!(flights.take("quiet", before)?, 100);
    Ok(())
}

#[test]
fn many_clients_cannot_exceed_the_global_cap() -> TestResult {
    let now = Instant::now();
    let mut flights = Flights::default();
    for n in 0..1024u16 {
        let address = IpAddr::V4(std::net::Ipv4Addr::new(
            192,
            0,
            n.to_be_bytes()[0],
            n.to_be_bytes()[1],
        ));
        flights.insert(n.to_string(), n, address, now)?;
    }
    assert!(
        flights
            .insert("overflow".into(), 1024, "198.51.100.1".parse()?, now)
            .is_err()
    );
    Ok(())
}
