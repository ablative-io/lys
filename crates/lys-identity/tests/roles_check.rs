#![cfg(test)]
//! R2: project ownership is the explicit owner relation, and every
//! role-version check is answered through one seam (`ROLE_OWNER_GIVE`,
//! `ROLE_OWNER_NOT_LABEL`, `ROLE_CHECK_SEAM` and `ROLE_CHECK_UNAVAILABLE`).

use std::error::Error;
use std::path::Path;

use lys_identity::grants::{GrantChange, PassOn, Relation};
use lys_identity::roles::test_support::{E, RoleWorld};
use lys_identity::roles::{Capacity, Check, RoleError};
use lys_identity::IdentityId;

type TestResult = Result<(), Box<dyn Error>>;

fn world() -> Result<(tempfile::TempDir, RoleWorld), Box<dyn Error>> {
    let dir = tempfile::TempDir::new()?;
    let world = RoleWorld::new(dir.path())?;
    Ok((dir, world))
}

#[test]
fn role_owner_give_is_admitted_only_for_the_administrator_or_an_owner() -> TestResult {
    let (_dir, mut world) = world()?;
    let (d1, o1, o2, x1) = (
        IdentityId::Person(world.d1),
        IdentityId::Person(world.o1),
        IdentityId::Person(world.o2),
        IdentityId::Person(world.x1),
    );
    assert_eq!(
        world
            .grants
            .book()
            .held_by(d1)
            .filter(|record| record.grant().resource() == &world.p1)
            .count(),
        0,
        "D1 holds no grant on P1"
    );
    let before = world.grants.revision();
    let mut cases = 0;

    let recorded = world.give_owner(d1, o1)?;
    assert_eq!(recorded.event.caller(), d1);
    let GrantChange::Issue(grant) = recorded.event.change() else {
        return Err("giving the owner relation issues a grant".into());
    };
    assert_eq!(grant.holder(), o1);
    assert_eq!(grant.responsible(), world.o1);
    assert_eq!(grant.parts().relation, Relation::owner());
    assert_eq!(grant.pass_on(), &PassOn::UseOnly);
    cases += 1;

    world.give_owner(o1, o2)?;
    cases += 1;

    let refused = world.give_owner(x1, x1).err();
    assert!(
        matches!(refused, Some(RoleError::NotPermitted { check: Check::GiveOwner, .. })),
        "{refused:?}"
    );
    cases += 1;

    let refused = world.give_owner(d1, IdentityId::Agent(world.a1)).err();
    assert!(
        matches!(refused, Some(RoleError::NotPermitted { check: Check::GiveOwner, .. })),
        "{refused:?}"
    );
    cases += 1;

    assert_eq!(cases, 4);
    assert_eq!(world.grants.revision() - before, 2, "grant events added: 2");
    Ok(())
}

#[test]
fn role_owner_not_label_ownership_is_never_read_from_a_name() -> TestResult {
    let (_dir, mut world) = world()?;
    world.assign(world.h1, world.a1, Some(E))?;
    let shown = world
        .directory
        .record(IdentityId::Person(world.x1))?
        .ok_or("X1 is in the directory")?;
    assert_eq!(shown.profile().display_name(), "Owner of P1");
    let x1_holds = world
        .grants
        .book()
        .held_by(IdentityId::Person(world.x1))
        .map(|record| record.grant().pass_on().clone())
        .collect::<Vec<_>>();
    assert_eq!(x1_holds, [PassOn::UseOnly], "X1 holds a use-only read grant on P1");

    assert_eq!(world.capacity(world.x1, None, Check::EditTemplates)?, None);
    assert_eq!(
        world.capacity(world.o1, None, Check::EditTemplates)?,
        Some(Capacity::ProjectOwner)
    );
    assert_eq!(
        world.capacity(world.h1, Some(world.a1), Check::Move)?,
        Some(Capacity::ResponsiblePerson)
    );
    assert_eq!(world.capacity(world.h1, None, Check::EditTemplates)?, None);
    Ok(())
}

#[test]
fn role_check_seam_is_the_only_roles_file_naming_the_permission_module() -> TestResult {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let roles = root.join("src").join("roles");
    let mut naming = Vec::new();
    let mut read = 0;
    for entry in std::fs::read_dir(&roles)? {
        let path = entry?.path();
        if path.is_file() {
            read += 1;
            if std::fs::read_to_string(&path)?.contains("grants::permission") {
                naming.push(path.strip_prefix(root)?.display().to_string());
            }
        }
    }
    assert!(read > 1, "the roles module's files are read");
    assert_eq!(naming, ["src/roles/check.rs"]);
    Ok(())
}

#[test]
fn role_check_unavailable_refuses_by_the_check_name_and_commits_nothing() -> TestResult {
    let (_dir, mut world) = world()?;
    world.assign(world.h1, world.a1, Some(E))?;
    world.make_version_2()?;
    let before = world.roles.events().len();
    world.engine.set_down(true);
    let refused = world.move_to(world.o1, world.a1, 2, None).err();
    world.engine.set_down(false);
    assert!(
        matches!(refused, Some(RoleError::CheckUnavailable { check: Check::Move, .. })),
        "{refused:?}"
    );
    assert_eq!(world.roles.events().len(), before);
    assert_eq!(world.holding(world.a1)?.version, 1);
    Ok(())
}
