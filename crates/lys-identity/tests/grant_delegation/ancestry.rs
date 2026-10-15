use super::{BOTH, GrantError, GrantId, IdentityId, PassOn, Resource, World, pass};
use lys_identity::grants::DelegateRequest;
use std::error::Error;

type Refusals = Vec<(DelegateRequest, GrantError)>;

pub(super) fn refusals(
    world: &mut World,
    root: GrantId,
    middle: GrantId,
    ends: u64,
) -> Result<Refusals, Box<dyn Error>> {
    let (dana, tom, tom_agent) = (
        IdentityId::Person(world.dana),
        IdentityId::Person(world.tom),
        IdentityId::Agent(world.tom_agent),
    );
    let mut refusals = Vec::new();
    let mut broader_resource =
        world.request(tom, middle, tom_agent, "tern", PassOn::UseOnly, Some(ends))?;
    broader_resource.resource = Resource::new("project", "beta")?;
    refusals.push((
        broader_resource,
        GrantError::ResourceOutside {
            requested: "project:beta".to_owned(),
            source_grant: middle.to_string(),
        },
    ));
    refusals.push((
        world.request(tom, middle, tom_agent, "heron", PassOn::UseOnly, Some(ends))?,
        GrantError::ActionsOutside {
            relation: "heron".to_owned(),
            outside: "write".to_owned(),
            model_version: 1,
        },
    ));
    let forged = GrantId::from_bytes([0xf0; 16]);
    refusals.push((
        world.request(tom, forged, tom_agent, "tern", PassOn::UseOnly, Some(ends))?,
        GrantError::SourceUnknown {
            grant: forged.to_string(),
        },
    ));
    refusals.push((
        world.request(tom, root, tom_agent, "tern", PassOn::UseOnly, Some(ends))?,
        GrantError::NotHolder {
            caller: tom.to_string(),
            grant: root.to_string(),
        },
    ));
    let mut changed_responsible =
        world.request(tom, middle, tom_agent, "tern", PassOn::UseOnly, Some(ends))?;
    changed_responsible.responsible = world.dana;
    refusals.push((
        changed_responsible,
        GrantError::ResponsibleMismatch {
            identity: tom_agent.to_string(),
            named: dana.to_string(),
            recorded: tom.to_string(),
        },
    ));
    refusals.extend(authority_bounds(world, middle, ends)?);
    Ok(refusals)
}

fn authority_bounds(
    world: &mut World,
    middle: GrantId,
    ends: u64,
) -> Result<Refusals, Box<dyn Error>> {
    let (tom, tom_agent) = (
        IdentityId::Person(world.tom),
        IdentityId::Agent(world.tom_agent),
    );
    Ok(vec![
        (
            world.request(
                tom,
                middle,
                tom_agent,
                "tern",
                PassOn::UseOnly,
                Some(ends + 1),
            )?,
            GrantError::ExpiryBeyondSource {
                requested: (ends + 1).to_string(),
                source_grant: middle.to_string(),
                source_ends: ends,
            },
        ),
        (
            world.request(tom, middle, tom_agent, "tern", PassOn::UseOnly, None)?,
            GrantError::ExpiryBeyondSource {
                requested: "no end".to_owned(),
                source_grant: middle.to_string(),
                source_ends: ends,
            },
        ),
        (
            world.request(
                tom,
                middle,
                tom_agent,
                "tern",
                pass(&["read"], &BOTH)?,
                Some(ends),
            )?,
            GrantError::PassOnBeyondSource {
                source_grant: middle.to_string(),
            },
        ),
    ])
}
