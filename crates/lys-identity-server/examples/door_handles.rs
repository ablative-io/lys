//! Read an agent's handle records from a running door through the start
//! route's own client, and print the count of credentials and each id with
//! its validity. Never a value: the client takes ids and states only.
//!
//! `cargo run -p lys-identity-server --example door_handles -- <door address> <agent>`
//!
//! This is the verification a person runs with the door started by name
//! from its own tree at the commit where SECRETS-002 R1 lands, because the
//! door does not run inside this workspace's tests.

use lys_identity_server::routes::door_handles::{DoorHandles, report};

fn main() -> Result<(), String> {
    let mut arguments = std::env::args().skip(1);
    let usage = || "usage: door_handles <door address> <agent>".to_owned();
    let address = arguments.next().ok_or_else(usage)?;
    let agent = arguments.next().ok_or_else(usage)?;
    let handles = DoorHandles::at(&address)?;
    println!("{}", report(&handles, &agent));
    Ok(())
}
