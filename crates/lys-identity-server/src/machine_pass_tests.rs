//! Which machine asks for a computer's pass: the one its latest join made,
//! never one a later join replaced, and none for a computer no join made a
//! machine of.

use std::error::Error;
use std::str::FromStr;

use lys_identity::{MachineId, PersonId};

use super::current;
use crate::network_machines::Joined;

const ISSUER: &str = "person-d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1";
const FIRST: &str = "machine-5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a";
const SECOND: &str = "machine-6b6b6b6b6b6b6b6b6b6b6b6b6b6b6b6b";
const OTHER: &str = "machine-7c7c7c7c7c7c7c7c7c7c7c7c7c7c7c7c";

fn joined(
    identity: &str,
    computer: &str,
    key: &str,
    replaced: bool,
) -> Result<Joined, Box<dyn Error>> {
    Ok(Joined {
        identity: MachineId::from_str(identity)?,
        machine: computer.to_owned(),
        key: key.repeat(32),
        issuer: PersonId::from_str(ISSUER)?,
        at: 10,
        replaced,
        retired: false,
    })
}

#[test]
fn the_latest_join_of_the_computer_is_the_machine_that_asks() -> Result<(), Box<dyn Error>> {
    let machines = [
        joined(FIRST, "ward", "ab", true)?,
        joined(OTHER, "desk", "cd", false)?,
        joined(SECOND, "ward", "ef", false)?,
    ];
    let asking = current(&machines, "ward")?;
    assert_eq!(asking.identity.to_string(), SECOND);
    assert_eq!(asking.key, "ef".repeat(32), "the latest join's key");
    assert_eq!(current(&machines, "desk")?.identity.to_string(), OTHER);
    Ok(())
}

#[test]
fn a_computer_no_join_made_a_machine_of_is_refused_by_name() -> Result<(), Box<dyn Error>> {
    let machines = [joined(FIRST, "ward", "ab", true)?];
    for computer in ["ward", "desk"] {
        let refused = current(&machines, computer)
            .err()
            .ok_or("a computer with no live machine was answered one")?;
        assert_eq!(refused.name(), "runner_dial_refused");
        assert!(refused.to_string().contains(computer), "{refused}");
    }
    assert!(current(&[], "ward").is_err(), "no join at all");
    Ok(())
}
