use std::collections::BTreeMap;
use std::error::Error;

use super::{identity_kind, placed_within};
use crate::grants::Resource;
use crate::{AgentId, ConnectorId, IdentityId, PersonId, ServiceAccountId};

fn placements(
    pairs: &[(&str, &str, &str, &str)],
) -> Result<BTreeMap<Resource, Resource>, Box<dyn Error>> {
    let mut placed = BTreeMap::new();
    for (child_kind, child_id, parent_kind, parent_id) in pairs {
        placed.insert(
            Resource::new(child_kind, child_id)?,
            Resource::new(parent_kind, parent_id)?,
        );
    }
    Ok(placed)
}

#[test]
fn each_identity_names_its_own_kind() {
    assert_eq!(
        identity_kind(&IdentityId::Person(PersonId::from_bytes([1; 16]))),
        "person"
    );
    assert_eq!(
        identity_kind(&IdentityId::Agent(AgentId::from_bytes([2; 16]))),
        "agent"
    );
    assert_eq!(
        identity_kind(&IdentityId::ServiceAccount(ServiceAccountId::from_bytes(
            [3; 16]
        ))),
        "service_account"
    );
    assert_eq!(
        identity_kind(&IdentityId::Connector(ConnectorId::from_bytes([4; 16]))),
        "connector"
    );
}

#[test]
fn a_channel_is_within_the_workspace_it_is_placed_under() -> Result<(), Box<dyn Error>> {
    let placed = placements(&[
        ("sample.channel", "ward-a", "sample.workspace", "ward"),
        ("sample.thread", "t1", "sample.channel", "ward-a"),
        (
            "sample.channel",
            "elsewhere-a",
            "sample.workspace",
            "elsewhere",
        ),
    ])?;
    let parent = |child: &Resource| placed.get(child).cloned();
    let ward = Resource::new("sample.workspace", "ward")?;
    assert!(placed_within(
        &Resource::new("sample.channel", "ward-a")?,
        &ward,
        parent
    ));
    assert!(placed_within(
        &Resource::new("sample.thread", "t1")?,
        &ward,
        parent
    ));
    assert!(placed_within(&ward, &ward, parent));
    assert!(!placed_within(
        &Resource::new("sample.channel", "elsewhere-a")?,
        &ward,
        parent
    ));
    Ok(())
}

#[test]
fn a_guessed_or_unplaced_id_is_within_no_workspace() -> Result<(), Box<dyn Error>> {
    let placed = placements(&[("sample.channel", "ward-a", "sample.workspace", "ward")])?;
    let parent = |child: &Resource| placed.get(child).cloned();
    let ward = Resource::new("sample.workspace", "ward")?;
    assert!(!placed_within(
        &Resource::new("sample.channel", "ward-z")?,
        &ward,
        parent
    ));
    assert!(!placed_within(
        &Resource::new("sample.workspace", "ward-a")?,
        &ward,
        parent
    ));
    Ok(())
}

#[test]
fn a_placement_cycle_ends_the_walk_without_reaching() -> Result<(), Box<dyn Error>> {
    let placed = placements(&[
        ("sample.channel", "a", "sample.channel", "b"),
        ("sample.channel", "b", "sample.channel", "a"),
    ])?;
    let mut asked = 0_usize;
    let parent = |child: &Resource| {
        asked += 1;
        placed.get(child).cloned()
    };
    let ward = Resource::new("sample.workspace", "ward")?;
    assert!(!placed_within(
        &Resource::new("sample.channel", "a")?,
        &ward,
        parent
    ));
    assert_eq!(asked, 2, "each placement is read once");
    Ok(())
}
