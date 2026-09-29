#![cfg(test)]
//! Explicit instants exercise recovery and independence without sleeping.
use super::{Attempts, WINDOW};
use std::error::Error;
use std::net::IpAddr;
use std::time::{Duration, Instant};

#[test]
fn the_address_recovers_at_the_boundary_and_blocked_attempts_do_not_extend_it()
-> Result<(), Box<dyn Error>> {
    let attempts = Attempts::default();
    let now = Instant::now();
    let address: IpAddr = "192.0.2.1".parse()?;
    for _ in 0..5 {
        attempts.at(address, 5, now)?;
    }
    assert!(
        attempts
            .at(
                address,
                5,
                (now + WINDOW)
                    .checked_sub(Duration::from_nanos(1))
                    .ok_or("instant underflow")?
            )
            .is_err()
    );
    attempts.at("192.0.2.2".parse()?, 5, now)?;
    attempts.at(address, 5, now + WINDOW)?;
    Ok(())
}

#[test]
fn concurrent_attempts_cannot_overrun_the_allowance() -> Result<(), Box<dyn Error>> {
    let attempts = std::sync::Arc::new(Attempts::default());
    let address: IpAddr = "192.0.2.1".parse()?;
    let now = Instant::now();
    let workers: Vec<_> = (0..20)
        .map(|_| {
            let attempts = std::sync::Arc::clone(&attempts);
            std::thread::spawn(move || attempts.at(address, 5, now).is_ok())
        })
        .collect();
    let mut admitted = 0;
    for worker in workers {
        match worker.join() {
            Ok(true) => admitted += 1,
            Ok(false) => {}
            Err(payload) => return Err(format!("worker panicked: {payload:?}").into()),
        }
    }
    assert_eq!(admitted, 5);
    Ok(())
}

#[test]
fn attempts_from_ipv6_privacy_addresses_share_the_prefix_allowance() -> Result<(), Box<dyn Error>> {
    let attempts = Attempts::default();
    let now = Instant::now();
    for n in 0..5u16 {
        attempts.at(
            IpAddr::V6(std::net::Ipv6Addr::new(0x2001, 0xdb8, 1, 2, 0, 0, 0, n)),
            5,
            now,
        )?;
    }
    assert!(
        attempts
            .at("2001:db8:1:2:abcd::1".parse()?, 5, now)
            .is_err()
    );
    attempts.at("2001:db8:1:3::1".parse()?, 5, now)?;
    Ok(())
}
