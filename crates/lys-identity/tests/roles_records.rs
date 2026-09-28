#![cfg(test)]
//! R1: the role, version and holding records and their signed events
//! (`ROLE_RECORDS`, `ROLE_RECORDS_VALUES`, `ROLE_RECORDS_EVENTS` and
//! `ROLE_RECORDS_TEMPLATE`).

use std::collections::BTreeSet;
use std::error::Error;

use lys_identity::grants::{PassOn, Relation, Resource};
use lys_identity::roles::test_support::{E, RoleWorld, T0};
use lys_identity::roles::{
    Capacity, ChangeDefault, ChangePolicy, ChangeTitle, Holding, HoldingId, MovePolicy, Renew,
    RoleBook, RoleChange, RoleError, RoleEvent, decode_event_body, decode_holding,
    decode_template, encode_event_body, encode_holding, encode_template, verify_role_event,
};
use lys_identity::{IdentityId, OperationId};

type TestResult = Result<(), Box<dyn Error>>;

fn world() -> Result<(tempfile::TempDir, RoleWorld), Box<dyn Error>> {
    let dir = tempfile::TempDir::new()?;
    let world = RoleWorld::new(dir.path())?;
    Ok((dir, world))
}

/// `bytes` with the first `needle` replaced by `with`.
fn replaced(bytes: &[u8], needle: &[u8], with: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
    let at = bytes
        .windows(needle.len())
        .position(|window| window == needle)
        .ok_or("the bytes do not carry the needle")?;
    let mut out = bytes[..at].to_vec();
    out.extend_from_slice(with);
    out.extend_from_slice(&bytes[at + needle.len()..]);
    Ok(out)
}

/// The CBOR text head and bytes of `word`, for a word under 24 bytes.
fn cbor_text(word: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    let len = u8::try_from(word.len())?;
    assert!(len < 24, "a short text head holds under 24 bytes");
    let mut out = vec![0x60 + len];
    out.extend_from_slice(word.as_bytes());
    Ok(out)
}

#[test]
fn role_records_round_trip_and_refuse_a_gap_or_a_replacement() -> TestResult {
    let (_dir, mut world) = world()?;
    world.make_version_2()?;
    let key = world.roles.service_key();
    let mut replayed = RoleBook::default();
    let mut versions = Vec::new();
    for signed in world.roles.events() {
        let verified = verify_role_event(signed.bytes(), &key)?;
        assert_eq!(verified.event(), signed.event());
        let body = encode_event_body(signed.event());
        assert_eq!(&decode_event_body(&body)?, signed.event());
        replayed.apply(verified.event())?;
        if let RoleChange::VersionMade { version, .. } = verified.event().change() {
            versions.push(*version);
        }
    }
    assert_eq!(versions, [1, 2]);
    assert_eq!(
        replayed.role(world.builder),
        world.roles.book().role(world.builder),
        "a role with versions 1 and 2 round-trips through its event encoding unchanged"
    );

    let mut first = RoleBook::default();
    let opening = world
        .roles
        .events()
        .first()
        .ok_or("builder's version 1 is committed")?;
    first.apply(opening.event())?;
    let mut outcomes = 0;

    let gap = RoleEvent::new(
        OperationId::generate()?,
        IdentityId::Person(world.o1),
        Capacity::ProjectOwner,
        world.now,
        RoleChange::VersionMade {
            role: world.builder,
            project: world.p1.clone(),
            version: 3,
            templates: vec![world.template("reader", "read")?],
            opening: None,
        },
    )?;
    let refused = first.check(&gap).err();
    assert!(
        matches!(refused, Some(RoleError::VersionGap { last: 1, version: 3, .. })),
        "{refused:?}"
    );
    outcomes += 1;

    let RoleChange::VersionMade {
        opening: Some(opened),
        ..
    } = opening.event().change()
    else {
        return Err("builder's first event opens it".into());
    };
    let replace = RoleEvent::new(
        OperationId::generate()?,
        IdentityId::Person(world.o1),
        Capacity::ProjectOwner,
        world.now,
        RoleChange::VersionMade {
            role: world.builder,
            project: world.p1.clone(),
            version: 1,
            templates: vec![world.template("pusher", "push")?],
            opening: Some(opened.clone()),
        },
    )?;
    let refused = first.check(&replace).err();
    assert!(
        matches!(refused, Some(RoleError::VersionImmutable { version: 1, .. })),
        "{refused:?}"
    );
    outcomes += 1;

    let unchanged = first
        .role(world.builder)
        .ok_or("builder is in the book")?;
    assert_eq!(unchanged.current().number(), 1);
    assert_eq!(unchanged.current().templates().len(), 2);
    outcomes += 1;
    assert_eq!(outcomes, 3);
    Ok(())
}

#[test]
fn role_records_values_refuse_every_value_outside_its_set() -> TestResult {
    let (_dir, mut world) = world()?;
    world.assign(world.h1, world.a1, Some(E))?;
    let holding = world.holding(world.a1)?;
    let encoded = encode_holding(&holding);
    assert_eq!(decode_holding(&encoded)?, holding);
    let mut refusals = 0;

    let mut policy_tail = vec![0x08];
    policy_tail.extend(cbor_text(holding.policy.as_str())?);
    assert!(encoded.ends_with(&policy_tail));
    assert_eq!(encoded.first(), Some(&0xa8));
    let stem = &encoded[1..encoded.len() - policy_tail.len()];

    let mut absent = vec![0xa7];
    absent.extend_from_slice(stem);
    let refused = decode_holding(&absent).err();
    assert!(matches!(refused, Some(RoleError::MissingPolicy)), "{refused:?}");
    refusals += 1;

    let mut unknown = vec![0xa8];
    unknown.extend_from_slice(stem);
    unknown.push(0x08);
    unknown.extend(cbor_text("renewal_or_manual")?);
    let refused = decode_holding(&unknown).err();
    assert!(
        matches!(&refused, Some(RoleError::UnknownPolicy { text }) if text == "renewal_or_manual"),
        "{refused:?}"
    );
    refusals += 1;

    let elsewhere = Holding {
        id: HoldingId::generate()?,
        project: Resource::new("project", "p2")?,
        ..holding
    };
    let granted = RoleEvent::new(
        OperationId::generate()?,
        IdentityId::Person(world.h1),
        Capacity::ResponsiblePerson,
        world.now,
        RoleChange::HoldingGranted(Box::new(elsewhere)),
    )?;
    let refused = world.roles.book().check(&granted).err();
    assert!(
        matches!(refused, Some(RoleError::ProjectMismatch { .. })),
        "{refused:?}"
    );
    refusals += 1;

    let committed = world
        .roles
        .events()
        .iter()
        .find(|signed| signed.event().change().kind() == 4)
        .ok_or("the holding granted event is committed")?;
    let body = encode_event_body(committed.event());
    let mut named = vec![0x04];
    named.extend(cbor_text("responsible_person")?);
    let mut label = vec![0x04];
    label.extend(cbor_text("owner_label")?);
    let refused = decode_event_body(&replaced(&body, &named, &label)?).err();
    assert!(
        matches!(&refused, Some(RoleError::UnknownCapacity { text }) if text == "owner_label"),
        "{refused:?}"
    );
    refusals += 1;

    assert_eq!(refusals, 4, "none of the four yields a record");
    Ok(())
}

#[test]
fn role_records_events_one_of_each_kind_reads_back_with_its_actor() -> TestResult {
    let (_dir, mut world) = world()?;
    let h1 = IdentityId::Person(world.h1);
    world.give_owner(IdentityId::Person(world.d1), h1)?;
    world.assign(world.h1, world.a1, Some(E))?;
    world.make_version_2()?;
    world.move_to(world.h1, world.a1, 2, None)?;
    let holding = world.holding(world.a1)?.id;
    let renew = Renew {
        operation: OperationId::generate()?,
        actor: h1,
        holding,
        ends_at: E + 1,
    };
    let policy = ChangePolicy {
        operation: OperationId::generate()?,
        actor: h1,
        holding,
        policy: MovePolicy::DeliberateOnly,
    };
    let title = ChangeTitle {
        operation: OperationId::generate()?,
        actor: h1,
        role: world.builder,
        title: "Code builder".to_owned(),
    };
    let default = ChangeDefault {
        operation: OperationId::generate()?,
        actor: h1,
        role: world.builder,
        policy: MovePolicy::DeliberateOnly,
    };
    {
        let (roles, mut acting) = world.acting()?;
        roles.renew(&mut acting, &renew)?;
        roles.change_policy(&mut acting, &policy)?;
        roles.change_title(&mut acting, &title)?;
        roles.change_default(&mut acting, &default)?;
    }
    let reviewer = world.template("reader", "read")?;
    let make = lys_identity::roles::MakeRole {
        operation: OperationId::generate()?,
        actor: h1,
        project: world.p1.clone(),
        title: "Reviewer".to_owned(),
        templates: vec![reviewer],
    };
    {
        let (roles, mut acting) = world.acting()?;
        roles.make_role(&mut acting, &make)?;
    }

    let key = world.roles.service_key();
    let mut kinds = BTreeSet::new();
    let mut read_back = 0;
    for signed in world.roles.events() {
        let event = verify_role_event(signed.bytes(), &key)?;
        if event.event().actor() == world.h1 {
            read_back += 1;
            kinds.insert(event.event().change().kind());
        }
    }
    assert_eq!(read_back, 7);
    assert_eq!(kinds, (1..=7).collect::<BTreeSet<u64>>());
    assert!(T0 < world.now);
    Ok(())
}

#[test]
fn role_records_template_round_trips_and_refuses_a_holder_or_a_source() -> TestResult {
    let (_dir, world) = world()?;
    let template = world.template("reader", "read")?;
    let encoded = encode_template(&template);
    let mut outcomes = 0;

    let read = decode_template(&encoded)?;
    assert_eq!(read, template);
    assert_eq!(read.relation(), &Relation::new("reader")?);
    assert_eq!(read.resource(), &Resource::new("project", "p1")?);
    assert_eq!(
        read.actions().iter().map(ToString::to_string).collect::<Vec<_>>(),
        ["read"]
    );
    assert_eq!(read.pass_on(), &PassOn::UseOnly);
    assert_eq!(read.window().ends_at(), None);
    assert_eq!(read.responsible(), world.h1);
    outcomes += 1;

    assert_eq!(encoded.first(), Some(&0xa6));
    let mut with_holder = vec![0xa7, 0x03, 0x00];
    with_holder.extend_from_slice(&encoded[1..]);
    let refused = decode_template(&with_holder).err();
    assert!(
        matches!(refused, Some(RoleError::TemplateHasHolder)),
        "{refused:?}"
    );
    outcomes += 1;

    let window_at = encoded.len() - 10;
    assert_eq!(encoded[window_at], 0x0a, "the window is the last member");
    let mut with_source = vec![0xa7];
    with_source.extend_from_slice(&encoded[1..window_at]);
    with_source.extend_from_slice(&[0x09, 0x00]);
    with_source.extend_from_slice(&encoded[window_at..]);
    let refused = decode_template(&with_source).err();
    assert!(
        matches!(refused, Some(RoleError::TemplateHasSource)),
        "{refused:?}"
    );
    outcomes += 1;
    assert_eq!(outcomes, 3);
    Ok(())
}
