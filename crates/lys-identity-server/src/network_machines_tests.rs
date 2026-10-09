//! The machines the spent codes made, read from the codes' own file: a code
//! spent before machines were identities still reads and makes none, and a
//! later join of the same computer replaces the earlier machine.

use std::error::Error;
use std::sync::Arc;

use lys_identity::projection::Projection;
use lys_identity::{IdentityId, MachineId};
use serde_json::json;

use super::{joined, retired_with, with_machines};
use crate::network_join::{JoinStanding, JoinStore};

const ISSUER: &str = "person-d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1";
const FIRST: &str = "machine-5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a";
const SECOND: &str = "machine-6b6b6b6b6b6b6b6b6b6b6b6b6b6b6b6b";

fn kept(codes: &serde_json::Value) -> Result<(tempfile::TempDir, JoinStore), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("network.joins.json");
    std::fs::write(
        &path,
        serde_json::to_vec(&json!({ "format": "lys-runner-joins/v1", "codes": codes }))?,
    )?;
    let store = JoinStore::open(&path)?;
    Ok((dir, store))
}

fn used(operation: &str, machine: &str, at: u64, identity: Option<&str>) -> serde_json::Value {
    let mut standing = json!({ "state": "used", "at": at, "key": "ab".repeat(32) });
    if let Some(identity) = identity {
        standing["identity"] = json!(identity);
    }
    json!({
        "operation": operation,
        "machine": machine,
        "issued_by": ISSUER,
        "issued_at": at - 1,
        "standing": standing,
    })
}

#[test]
fn a_code_spent_before_machines_still_reads_and_makes_no_machine() -> Result<(), Box<dyn Error>> {
    let (_dir, store) = kept(&json!([used("op-one", "ward", 10, None)]))?;
    let [record] = store.records() else {
        return Err("one code is kept".into());
    };
    assert_eq!(
        record.standing,
        JoinStanding::Used {
            at: 10,
            key: "ab".repeat(32),
            identity: None,
        }
    );
    assert!(
        joined(&store)?.is_empty(),
        "no machine is made after its join"
    );
    Ok(())
}

#[test]
fn a_later_join_of_the_same_computer_replaces_the_earlier_machine() -> Result<(), Box<dyn Error>> {
    let (_dir, store) = kept(&json!([
        used("op-one", "ward", 10, Some(FIRST)),
        used("op-two", "ward", 20, Some(SECOND)),
    ]))?;
    let machines = joined(&store)?;
    let [first, second] = machines.as_slice() else {
        return Err(format!("two machines: {machines:?}").into());
    };
    assert_eq!(first.identity.to_string(), FIRST);
    assert!(first.replaced);
    assert_eq!(second.identity.to_string(), SECOND);
    assert!(!second.replaced);
    assert_eq!(second.issuer.to_string(), ISSUER);
    assert_eq!(second.machine, "ward");
    assert_eq!(second.at, 20);
    Ok(())
}

#[test]
fn a_kept_file_naming_one_machine_twice_or_a_malformed_one_is_refused() -> Result<(), Box<dyn Error>>
{
    let twice = kept(&json!([
        used("op-one", "ward", 10, Some(FIRST)),
        used("op-two", "desk", 20, Some(FIRST)),
    ]));
    assert!(twice.is_err(), "one machine for two joins");
    let malformed = kept(&json!([used("op-one", "ward", 10, Some("agent-00"))]));
    assert!(malformed.is_err(), "a machine identity that is not one");
    Ok(())
}

#[test]
fn a_machine_whose_issuer_the_directory_does_not_hold_answers_for_nothing()
-> Result<(), Box<dyn Error>> {
    let (_dir, store) = kept(&json!([used("op-one", "ward", 10, Some(FIRST))]))?;
    let directory = Projection::new();
    let accounts = with_machines(Arc::default(), &joined(&store)?, &directory)?;
    let view = directory.with_accounts(accounts);
    let machine = IdentityId::Machine(FIRST.parse::<MachineId>()?);
    assert!(view.record(machine).is_none());
    Ok(())
}

#[test]
fn a_retired_computers_machine_is_retired_and_its_neighbours_are_not() -> Result<(), Box<dyn Error>>
{
    let (_dir, store) = kept(&json!([
        used("op-one", "ward", 10, Some(FIRST)),
        used("op-two", "desk", 20, Some(SECOND)),
    ]))?;
    let fresh = joined(&store)?;
    assert!(
        fresh.iter().all(|machine| !machine.is_retired()),
        "{fresh:?}"
    );
    let retired = std::collections::BTreeSet::from(["ward".to_owned()]);
    let machines = retired_with(fresh, &retired);
    let [ward, desk] = machines.as_slice() else {
        return Err(format!("two machines: {machines:?}").into());
    };
    assert!(ward.retired && !ward.replaced && ward.is_retired());
    assert!(!desk.retired && !desk.is_retired());
    Ok(())
}
