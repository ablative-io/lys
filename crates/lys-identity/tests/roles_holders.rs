#![cfg(test)]
//! R5: who holds a role, and on which version, answered as a query over
//! holdings (`ROLE_HOLDERS` and `ROLE_HOLDERS_QUERY`).

use std::error::Error;

use lys_identity::AgentId;
use lys_identity::roles::VersionHolders;
use lys_identity::roles::test_support::{RoleWorld, T0};

type TestResult = Result<(), Box<dyn Error>>;

fn world() -> Result<(tempfile::TempDir, RoleWorld), Box<dyn Error>> {
    let dir = tempfile::TempDir::new()?;
    let world = RoleWorld::new(dir.path())?;
    Ok((dir, world))
}

/// Each version with its holders' agents and whether it is current.
fn listed(answer: &[VersionHolders]) -> Vec<(u64, Vec<AgentId>, bool)> {
    answer
        .iter()
        .map(|version| {
            (
                version.version,
                version.holders.iter().map(|holder| holder.agent).collect(),
                version.current,
            )
        })
        .collect()
}

#[test]
fn role_holders_groups_by_version_and_leaves_out_a_lapsed_holding() -> TestResult {
    let (_dir, mut world) = world()?;
    world.assign(world.h1, world.a1, None)?;
    world.assign(world.h1, world.a3, Some(T0 + 100))?;
    world.make_version_2()?;
    world.assign(world.h1, world.a2, None)?;
    world.now = T0 + 200;
    let answer = world.roles.holders(world.builder, world.now)?;
    assert_eq!(
        listed(&answer),
        [
            (1, vec![world.a1], false),
            (2, vec![world.a2], true),
        ]
    );
    assert!(
        answer
            .iter()
            .all(|version| version.holders.iter().all(|holder| holder.agent != world.a3)),
        "A3's lapsed holding is not listed"
    );
    Ok(())
}

#[test]
fn role_holders_query_reads_a_new_holding_with_no_other_write() -> TestResult {
    let (_dir, mut world) = world()?;
    world.assign(world.h1, world.a1, None)?;
    let first = world.roles.holders(world.builder, world.now)?;
    assert_eq!(listed(&first), [(1, vec![world.a1], true)]);
    let before = world.roles.events().len();
    world.assign(world.h1, world.a4, None)?;
    assert_eq!(world.roles.events().len() - before, 1, "one event between the answers");
    let next = world.roles.holders(world.builder, world.now)?;
    let mut expected = vec![world.a1, world.a4];
    expected.sort();
    let mut got = listed(&next)
        .into_iter()
        .flat_map(|(_, agents, _)| agents)
        .collect::<Vec<_>>();
    got.sort();
    assert_eq!(got, expected);
    Ok(())
}
