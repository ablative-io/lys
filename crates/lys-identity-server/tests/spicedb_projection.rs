#![cfg(test)]
//! R2: the schema written at start-up, and committed signed grant events
//! projected into `SpiceDB` relationships by the one projector.
//!
//! The expected relationships are written by hand below, from the grant
//! contract ADR-078 records (C5) and schema.zed, as text with named places
//! for the ids a run generates; the mapping under test produces none of
//! them. A log position counts events from 1. Every grant here has one
//! action, so each is exactly 3 relationships. Beside them the root
//! authority object holds its two wildcard relationships,
//! [`ROOT_AUTHORITY_ANCHOR`], written by every root issue and never deleted:
//! a permission is a set of subjects, and without them no walk could end at
//! the root authority for any holder.

mod spicedb_support;

use std::sync::{Arc, Mutex, PoisonError};

use identity_contract::harness::{Fault, GRANT_MODEL, Harness, Service};
use lys_core::Ed25519Identity;
use lys_identity::grants::{
    GrantError, GrantId, Grants, MemoryRelationships, ProjectionRefusal, Recorded, Relation,
    Resource, RootRequest, Route, Source, Window,
};
use lys_identity::{IdentityId, OperationId};
use lys_identity_server::spicedb::schema::{SCHEMA, Started, ensure};
use lys_identity_server::spicedb::wire::authzed::api::v1::WriteSchemaRequest;
use lys_identity_server::spicedb::wire::authzed::api::v1::check_permission_response::Permissionship;
use lys_identity_server::spicedb::wire::authzed::api::v1::relationship_update::Operation;
use lys_identity_server::spicedb::{
    ClientConfig, FilePosition, Projector, SchemaRead, SpiceDbApi, SpiceDbClient, SpiceDbSettings,
};
use spicedb_support::server::SpiceDb;
use spicedb_support::{
    Counting, Pause, T0, TestResult, World, belonging, check_directly, model, position, project,
    read, render, stored, writes,
};

/// RP's issued expiry.
const E0: u64 = T0 + 10_000;
/// G1's expiry, before E0.
const E1: u64 = T0 + 9_000;
/// G2's expiry, before E1.
const E2: u64 = T0 + 8_000;
/// G3's expiry, before E2.
const E3: u64 = T0 + 7_000;
/// G4's expiry, before E5.
const E4: u64 = T0 + 19_000;
/// RQ's issued expiry.
const E5: u64 = T0 + 20_000;
/// One hour.
const HOUR: u64 = 3_600;

/// The relationships after G4, written by hand from the recorded grant
/// contract and schema: for each of RP, RQ, G1, G2, G3 and G4 its resource
/// relationship, its holder relationship with its expiry caveat, and its
/// standing relationship, a root relationship for a root grant and a source
/// relationship for a delegated one.
const EXPECTED_RELATIONSHIPS_BEFORE_REVOKE: [&str; 18] = [
    "resource:project/x/read#granted@grant:{RP}",
    "grant:{RP}#holder@person:{P} [within_window ends_at={E0} starts_at={T0}]",
    "grant:{RP}#root@root_authority:directory",
    "resource:project/y/read#granted@grant:{RQ}",
    "grant:{RQ}#holder@person:{Q} [within_window ends_at={E5} starts_at={T0}]",
    "grant:{RQ}#root@root_authority:directory",
    "resource:project/x/read#granted@grant:{G1}",
    "grant:{G1}#holder@agent:{A} [within_window ends_at={E1} starts_at={T0}]",
    "grant:{G1}#source@grant:{RP}",
    "resource:project/x/read#granted@grant:{G2}",
    "grant:{G2}#holder@agent:{C} [within_window ends_at={E2} starts_at={T0}]",
    "grant:{G2}#source@grant:{G1}",
    "resource:project/x/read#granted@grant:{G3}",
    "grant:{G3}#holder@agent:{D} [within_window ends_at={E3} starts_at={T0}]",
    "grant:{G3}#source@grant:{G2}",
    "resource:project/y/read#granted@grant:{G4}",
    "grant:{G4}#holder@agent:{E} [within_window ends_at={E4} starts_at={T0}]",
    "grant:{G4}#source@grant:{RQ}",
];

/// The root authority object's two wildcard relationships.
const ROOT_AUTHORITY_ANCHOR: [&str; 2] = [
    "root_authority:directory#anyone@agent:*",
    "root_authority:directory#anyone@person:*",
];

/// How many relationships of G1, G2 and G3 remain once G1 is revoked: none.
/// The grant representation (C5) ADR-078 records deletes, on the committed
/// revoke, every relationship of the withdrawn grant and of every grant
/// derived from it through the source relation.
const RELATIONSHIPS_AFTER_ROOT_REVOKE: usize = 0;

/// The main fixture: RP and RQ from the root authority; G1 from P to A, G2
/// from A to C, G3 from C to D, all on X; G4 from Q to E on Y.
struct Fixture {
    world: World,
    names: Vec<(&'static str, String)>,
    a: IdentityId,
    c: IdentityId,
    d: IdentityId,
    e: IdentityId,
    p: IdentityId,
    rp: Recorded,
    rq: Recorded,
    g1: Recorded,
    g2: Recorded,
    g3: Recorded,
    g4: Recorded,
}

impl Fixture {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let mut world = World::new()?;
        let (on_x, on_y) = (project("x")?, project("y")?);
        let person_p = world.person("P")?;
        let person_q = world.person("Q")?;
        let agent_a = IdentityId::Agent(world.agent(person_p, "A")?);
        let agent_c = IdentityId::Agent(world.agent(person_p, "C")?);
        let agent_d = IdentityId::Agent(world.agent(person_p, "D")?);
        let agent_e = IdentityId::Agent(world.agent(person_q, "E")?);
        let (pi, qi) = (IdentityId::Person(person_p), IdentityId::Person(person_q));
        let rp = world.root_grant(person_p, &on_x, Some(E0))?;
        let rq = world.root_grant(person_q, &on_y, Some(E5))?;
        let g1 = world.delegate(rp.event.grant(), (pi, agent_a), &on_x, Some(E1))??;
        let g2 = world.delegate(g1.event.grant(), (agent_a, agent_c), &on_x, Some(E2))??;
        let g3 = world.delegate(g2.event.grant(), (agent_c, agent_d), &on_x, Some(E3))??;
        let g4 = world.delegate(rq.event.grant(), (qi, agent_e), &on_y, Some(E4))??;
        let names = vec![
            ("{RP}", rp.event.grant().to_string()),
            ("{RQ}", rq.event.grant().to_string()),
            ("{G1}", g1.event.grant().to_string()),
            ("{G2}", g2.event.grant().to_string()),
            ("{G3}", g3.event.grant().to_string()),
            ("{G4}", g4.event.grant().to_string()),
            ("{P}", pi.to_string()),
            ("{Q}", qi.to_string()),
            ("{A}", agent_a.to_string()),
            ("{C}", agent_c.to_string()),
            ("{D}", agent_d.to_string()),
            ("{E}", agent_e.to_string()),
            ("{E0}", E0.to_string()),
            ("{E1}", E1.to_string()),
            ("{E2}", E2.to_string()),
            ("{E3}", E3.to_string()),
            ("{E4}", E4.to_string()),
            ("{E5}", E5.to_string()),
            ("{T0}", T0.to_string()),
        ];
        Ok(Self {
            world,
            names,
            a: agent_a,
            c: agent_c,
            d: agent_d,
            e: agent_e,
            p: pi,
            rp,
            rq,
            g1,
            g2,
            g3,
            g4,
        })
    }

    /// A hand-written line with this run's ids in their places.
    fn fill(&self, line: &str) -> String {
        self.names
            .iter()
            .fold(line.to_owned(), |line, (place, value)| {
                line.replace(place, value)
            })
    }

    fn expected_before_revoke(&self) -> Vec<String> {
        let mut expected: Vec<String> = EXPECTED_RELATIONSHIPS_BEFORE_REVOKE
            .iter()
            .map(|line| self.fill(line))
            .collect();
        expected.extend(ROOT_AUTHORITY_ANCHOR.iter().map(|line| (*line).to_owned()));
        expected.sort();
        expected
    }

    fn ids(recorded: &[&Recorded]) -> Vec<GrantId> {
        recorded.iter().map(|one| one.event.grant()).collect()
    }

    fn revoke_g1(&mut self) -> Result<Recorded, Box<dyn std::error::Error>> {
        let (grant, by) = (self.g1.event.grant(), self.p);
        self.world.revoke(grant, by)
    }
}

/// A disposable `SpiceDB` taking at most `cap` updates a write, its store
/// given the schema, and a counting client of it.
fn engine(cap: Option<u16>) -> Result<(SpiceDb, Arc<Counting>), Box<dyn std::error::Error>> {
    let spicedb = SpiceDb::start(cap)?;
    let client = spicedb.client()?;
    ensure(client.as_ref())?;
    Ok((spicedb, client))
}

#[test]
fn replaying_up_to_g4_leaves_exactly_the_hand_written_relationships() -> TestResult {
    let fixture = Fixture::new()?;
    assert_eq!(EXPECTED_RELATIONSHIPS_BEFORE_REVOKE.len(), 18);
    let (_spicedb, client) = engine(None)?;
    let mut projector = fixture.world.projector(&client, 1000)?;
    projector.catch_up(fixture.world.book(), position(&fixture.g4))?;
    let held = stored(client.as_ref())?;
    assert_eq!(held, fixture.expected_before_revoke());
    let root_of = |grant: &Recorded| {
        fixture.fill(&format!(
            "grant:{}#root@root_authority:directory",
            grant.event.grant()
        ))
    };
    assert!(held.contains(&root_of(&fixture.rp)));
    assert!(held.contains(&root_of(&fixture.rq)));
    for derived in [&fixture.g1, &fixture.g2, &fixture.g3, &fixture.g4] {
        assert!(!held.contains(&root_of(derived)));
    }
    for root in [&fixture.rp, &fixture.rq] {
        let source = format!("grant:{}#source@", root.event.grant());
        assert!(!held.iter().any(|line| line.starts_with(&source)));
    }
    Ok(())
}

#[test]
fn after_g4_a_c_and_d_may_read_x_and_e_may_read_y() -> TestResult {
    let fixture = Fixture::new()?;
    let (_spicedb, client) = engine(None)?;
    let mut projector = fixture.world.projector(&client, 1000)?;
    projector.catch_up(fixture.world.book(), position(&fixture.g4))?;
    let (x, y, now) = (project("x")?, project("y")?, fixture.world.now);
    let check =
        |who: IdentityId, on: &Resource| check_directly(client.as_ref(), who, on, &read()?, now);
    assert_eq!(check(fixture.a, &x)?, Permissionship::HasPermission);
    assert_eq!(check(fixture.c, &x)?, Permissionship::HasPermission);
    assert_eq!(check(fixture.d, &x)?, Permissionship::HasPermission);
    assert_eq!(check(fixture.e, &y)?, Permissionship::HasPermission);
    Ok(())
}

#[test]
fn revoking_g1_refuses_a_c_and_d_and_leaves_e_and_the_roots_standing() -> TestResult {
    let mut fixture = Fixture::new()?;
    let (_spicedb, client) = engine(None)?;
    let mut projector = fixture.world.projector(&client, 1000)?;
    projector.catch_up(fixture.world.book(), position(&fixture.g4))?;
    let chain = Fixture::ids(&[&fixture.g1, &fixture.g2, &fixture.g3]);
    let others = [&fixture.rp, &fixture.rq, &fixture.g4];
    let before = stored(client.as_ref())?;
    assert_eq!(belonging(&before, &chain), 9);
    for other in others {
        assert_eq!(belonging(&before, &Fixture::ids(&[other])), 3);
    }
    let revoke = fixture.revoke_g1()?;
    projector.catch_up(fixture.world.book(), fixture.world.grants.revision())?;
    let after = stored(client.as_ref())?;
    assert_eq!(belonging(&after, &chain), RELATIONSHIPS_AFTER_ROOT_REVOKE);
    for other in [&fixture.rp, &fixture.rq, &fixture.g4] {
        assert_eq!(belonging(&after, &Fixture::ids(&[other])), 3);
    }
    assert_eq!(projector.reached().position, position(&revoke));
    let (x, y, now) = (project("x")?, project("y")?, fixture.world.now);
    let check =
        |who: IdentityId, on: &Resource| check_directly(client.as_ref(), who, on, &read()?, now);
    assert_eq!(check(fixture.a, &x)?, Permissionship::NoPermission);
    assert_eq!(check(fixture.c, &x)?, Permissionship::NoPermission);
    assert_eq!(check(fixture.d, &x)?, Permissionship::NoPermission);
    assert_eq!(check(fixture.e, &y)?, Permissionship::HasPermission);
    Ok(())
}

/// Each write's updates, rendered.
fn written(client: &Counting, from: usize) -> Vec<Vec<String>> {
    writes(client)[from..]
        .iter()
        .map(|write| {
            write
                .updates
                .iter()
                .filter_map(|update| update.relationship.as_ref())
                .map(render)
                .collect()
        })
        .collect()
}

/// Whether every update of every write from `from` on is a delete.
fn all_deletes(client: &Counting, from: usize) -> bool {
    let delete: i32 = Operation::Delete.into();
    writes(client)[from..]
        .iter()
        .flat_map(|write| write.updates.iter())
        .all(|update| update.operation == delete)
}

#[test]
fn at_the_default_cap_the_revoke_of_g1_is_two_writes() -> TestResult {
    let mut fixture = Fixture::new()?;
    let (_spicedb, client) = engine(None)?;
    let mut projector = fixture.world.projector(&client, 1000)?;
    projector.catch_up(fixture.world.book(), position(&fixture.g4))?;
    let from = writes(&client).len();
    fixture.revoke_g1()?;
    projector.catch_up(fixture.world.book(), fixture.world.grants.revision())?;
    let revoke = written(&client, from);
    assert_eq!(revoke.len(), 2);
    assert!(all_deletes(&client, from));
    assert_eq!(
        revoke[0],
        vec![fixture.fill("grant:{G1}#source@grant:{RP}")]
    );
    let mut second = revoke[1].clone();
    second.sort();
    let mut rest: Vec<String> = [
        "resource:project/x/read#granted@grant:{G1}",
        "grant:{G1}#holder@agent:{A} [within_window ends_at={E1} starts_at={T0}]",
        "resource:project/x/read#granted@grant:{G2}",
        "grant:{G2}#holder@agent:{C} [within_window ends_at={E2} starts_at={T0}]",
        "grant:{G2}#source@grant:{G1}",
        "resource:project/x/read#granted@grant:{G3}",
        "grant:{G3}#holder@agent:{D} [within_window ends_at={E3} starts_at={T0}]",
        "grant:{G3}#source@grant:{G2}",
    ]
    .iter()
    .map(|line| fixture.fill(line))
    .collect();
    rest.sort();
    assert_eq!(second.len(), 8);
    assert_eq!(second, rest);
    let others = Fixture::ids(&[&fixture.rp, &fixture.rq, &fixture.g4]);
    assert_eq!(belonging(&second, &others), 0);
    Ok(())
}

#[test]
fn at_a_cap_of_one_the_revoke_of_g1_is_nine_writes_and_the_first_refuses_at_once() -> TestResult {
    let mut fixture = Fixture::new()?;
    let (_spicedb, client) = engine(Some(1))?;
    let mut projector = fixture.world.projector(&client, 1)?;
    projector.catch_up(fixture.world.book(), position(&fixture.g4))?;
    let from = writes(&client).len();
    let revoke = fixture.revoke_g1()?;
    let chain = Fixture::ids(&[&fixture.g1, &fixture.g2, &fixture.g3]);
    let (told, reached) = std::sync::mpsc::channel();
    let (release, held) = std::sync::mpsc::channel();
    *client.pause.lock().unwrap_or_else(PoisonError::into_inner) = Some(Pause {
        after: from + 1,
        reached: told,
        resume: held,
    });
    let (x, y, now) = (project("x")?, project("y")?, fixture.world.now);
    let (book, committed) = (fixture.world.book(), fixture.world.grants.revision());
    let projected = std::thread::scope(|scope| -> TestResult {
        let running = scope.spawn(|| projector.catch_up(book, committed).cloned());
        reached.recv()?;
        let check = |who: IdentityId, on: &Resource| {
            check_directly(client.as_ref(), who, on, &read()?, now)
        };
        assert_eq!(check(fixture.a, &x)?, Permissionship::NoPermission);
        assert_eq!(check(fixture.c, &x)?, Permissionship::NoPermission);
        assert_eq!(check(fixture.d, &x)?, Permissionship::NoPermission);
        assert_eq!(check(fixture.e, &y)?, Permissionship::HasPermission);
        assert_eq!(belonging(&stored(client.as_ref())?, &chain), 8);
        release.send(())?;
        running
            .join()
            .map_err(|panic| format!("the projector stopped: {panic:?}"))??;
        Ok(())
    });
    projected?;
    let revoked = written(&client, from);
    assert_eq!(revoked.len(), 9);
    assert!(revoked.iter().all(|write| write.len() == 1));
    assert!(all_deletes(&client, from));
    assert_eq!(
        revoked[0],
        vec![fixture.fill("grant:{G1}#source@grant:{RP}")]
    );
    let after = stored(client.as_ref())?;
    assert_eq!(belonging(&after, &chain), RELATIONSHIPS_AFTER_ROOT_REVOKE);
    for other in [&fixture.rp, &fixture.rq, &fixture.g4] {
        assert_eq!(belonging(&after, &Fixture::ids(&[other])), 3);
    }
    assert_eq!(projector.reached().position, position(&revoke));
    Ok(())
}

#[test]
fn a_projector_stopped_between_the_revokes_writes_resumes_it_on_restart() -> TestResult {
    let mut fixture = Fixture::new()?;
    let (_spicedb, client) = engine(Some(1))?;
    let mut projector = fixture.world.projector(&client, 1)?;
    projector.catch_up(fixture.world.book(), position(&fixture.g4))?;
    let from = writes(&client).len();
    let revoke = fixture.revoke_g1()?;
    *client
        .fail_write
        .lock()
        .unwrap_or_else(PoisonError::into_inner) = Some(from + 2);
    let crashed = projector.catch_up(fixture.world.book(), fixture.world.grants.revision());
    assert!(crashed.is_err(), "{crashed:?}");
    let chain = Fixture::ids(&[&fixture.g1, &fixture.g2, &fixture.g3]);
    assert_eq!(belonging(&stored(client.as_ref())?, &chain), 8);
    assert_eq!(projector.reached().position, position(&fixture.g4));
    assert!(projector.reached().position < position(&revoke));

    *client
        .fail_write
        .lock()
        .unwrap_or_else(PoisonError::into_inner) = None;
    let mut restarted = fixture.world.projector(&client, 1)?;
    assert_eq!(restarted.reached().position, position(&fixture.g4));
    restarted.catch_up(fixture.world.book(), fixture.world.grants.revision())?;
    let after = stored(client.as_ref())?;
    assert_eq!(belonging(&after, &chain), RELATIONSHIPS_AFTER_ROOT_REVOKE);
    for other in [&fixture.rp, &fixture.rq, &fixture.g4] {
        assert_eq!(belonging(&after, &Fixture::ids(&[other])), 3);
    }
    assert_eq!(restarted.reached().position, position(&revoke));
    Ok(())
}

#[test]
fn a_delegation_outliving_its_source_is_refused_by_name_and_commits_nothing() -> TestResult {
    let mut fixture = Fixture::new()?;
    let (_spicedb, client) = engine(None)?;
    let mut projector = fixture.world.projector(&client, 1000)?;
    projector.catch_up(fixture.world.book(), fixture.world.grants.revision())?;
    let (events, held) = (fixture.world.grants.revision(), stored(client.as_ref())?);
    let requested = E1 + HOUR;
    let (g1, a, c) = (fixture.g1.event.grant(), fixture.a, fixture.c);
    let refused = fixture
        .world
        .delegate(g1, (a, c), &project("x")?, Some(requested))?;
    let Err(refusal) = refused else {
        return Err(format!("a delegation past its source was answered: {refused:?}").into());
    };
    assert!(
        matches!(refusal, GrantError::DelegationOutlivesSource { .. }),
        "{refusal:?}"
    );
    let words = refusal.to_string();
    assert!(words.starts_with("delegation_outlives_source"), "{words}");
    for part in [g1.to_string(), E1.to_string(), requested.to_string()] {
        assert!(words.contains(&part), "{words} names no {part}");
    }
    assert_eq!(fixture.world.grants.revision() - events, 0);
    projector.catch_up(fixture.world.book(), fixture.world.grants.revision())?;
    assert_eq!(stored(client.as_ref())?, held);
    Ok(())
}

#[test]
fn a_committed_delegation_outliving_its_source_is_refused_by_the_projector() -> TestResult {
    let mut world = World::new()?;
    let (on_x, on_y) = (project("x")?, project("y")?);
    let person_p = world.person("P")?;
    let person_q = world.person("Q")?;
    let agent_a = IdentityId::Agent(world.agent(person_p, "A")?);
    let agent_c = IdentityId::Agent(world.agent(person_p, "C")?);
    let agent_e = IdentityId::Agent(world.agent(person_q, "E")?);
    let (pi, qi) = (IdentityId::Person(person_p), IdentityId::Person(person_q));
    let rp = world.root_grant(person_p, &on_x, Some(E0))?;
    let g1 = world.delegate(rp.event.grant(), (pi, agent_a), &on_x, Some(E1))??;
    let past = E1 + HOUR;
    let g2 = World::parts(
        (agent_a, agent_c, person_p),
        &on_x,
        Source::Grant(g1.event.grant()),
        Some(past),
    )?;
    let (g2_id, g2_position) = (g2.id, position(&g1) + 1);
    world.append_raw(g2)?;
    let rq = world.root_grant(person_q, &on_y, Some(E5))?;
    let g4 = world.delegate(rq.event.grant(), (qi, agent_e), &on_y, Some(E4))??;

    let (_spicedb, client) = engine(None)?;
    let mut projector = world.projector(&client, 1000)?;
    let refused = projector.catch_up(world.book(), world.grants.revision());
    let Err(refusal) = refused.cloned() else {
        return Err("the projector applied a delegation past its source".into());
    };
    assert!(
        matches!(
            &refusal,
            GrantError::ProjectionRefused {
                position,
                refusal: ProjectionRefusal::ExpiryPastSource { .. },
            } if *position == g2_position
        ),
        "{refusal:?}"
    );
    let words = refusal.to_string();
    assert!(words.contains("projection_expiry_past_source"), "{words}");
    for part in [
        g2_position.to_string(),
        g1.event.grant().to_string(),
        E1.to_string(),
        past.to_string(),
    ] {
        assert!(words.contains(&part), "{words} names no {part}");
    }
    let held = stored(client.as_ref())?;
    assert_eq!(belonging(&held, &[rp.event.grant()]), 3);
    assert_eq!(belonging(&held, &[g1.event.grant()]), 3);
    let unapplied = [g2_id, rq.event.grant(), g4.event.grant()];
    assert_eq!(belonging(&held, &unapplied), 0);
    assert_eq!(projector.reached().position, position(&g1));
    Ok(())
}

#[test]
fn a_grant_standing_on_nothing_is_refused_by_the_projector() -> TestResult {
    let mut world = World::new()?;
    let on_x = project("x")?;
    let person_p = world.person("P")?;
    let person_q = world.person("Q")?;
    let agent_a = IdentityId::Agent(world.agent(person_p, "A")?);
    let rp = world.root_grant(person_p, &on_x, None)?;
    // The row names G7 as a grant to agent A. DIRECTORY-006's grant contract
    // refuses that grant before any leaf holds it: Grant::new, and
    // decode_grant reading a leaf back, refuse a grant that names no source
    // unless its holder is its responsible person. So G7 is P's, issued by
    // agent A: it names no source grant and was not issued by the
    // directory's root authority, the case the projector alone can refuse.
    let holder = IdentityId::Person(person_p);
    let g7 = World::parts((agent_a, holder, person_p), &on_x, Source::Root, None)?;
    let (g7_id, g7_position) = (g7.id, position(&rp) + 1);
    world.append_raw(g7)?;
    let rq = world.root_grant(person_q, &project("y")?, None)?;

    let (_spicedb, client) = engine(None)?;
    let mut projector = world.projector(&client, 1000)?;
    let refused = projector.catch_up(world.book(), world.grants.revision());
    let Err(refusal) = refused.cloned() else {
        return Err("the projector applied a grant standing on nothing".into());
    };
    let at_g7 = matches!(
        &refusal,
        GrantError::ProjectionRefused { position, .. } if *position == g7_position
    );
    assert!(at_g7, "{refusal:?}");
    assert!(refusal.to_string().contains(&g7_position.to_string()));
    let held = stored(client.as_ref())?;
    assert_eq!(belonging(&held, &[rp.event.grant()]), 3);
    assert_eq!(belonging(&held, &[g7_id, rq.event.grant()]), 0);
    for holder in held.iter().filter(|line| line.contains("#holder@")) {
        let grant = holder.split('#').next().unwrap_or_default();
        let stands = held.iter().any(|line| {
            line.starts_with(&format!("{grant}#root@"))
                || line.starts_with(&format!("{grant}#source@"))
        });
        assert!(stands, "{grant} has no standing relationship");
    }
    assert_eq!(projector.reached().position, position(&rp));
    Ok(())
}

#[test]
fn two_grants_of_one_holder_action_and_resource_stand_apart() -> TestResult {
    let mut world = World::new()?;
    let on_x = project("x")?;
    let person_p = world.person("P")?;
    let agent_a = IdentityId::Agent(world.agent(person_p, "A")?);
    let rp = world.root_grant(person_p, &on_x, None)?;
    let pi = IdentityId::Person(person_p);
    let g5 = world.delegate(rp.event.grant(), (pi, agent_a), &on_x, None)??;
    let g6 = world.delegate(rp.event.grant(), (pi, agent_a), &on_x, None)??;
    let (_spicedb, client) = engine(None)?;
    let mut projector = world.projector(&client, 1000)?;
    projector.catch_up(world.book(), position(&g6))?;
    let held = stored(client.as_ref())?;
    let grants = [rp.event.grant(), g5.event.grant(), g6.event.grant()];
    assert_eq!(belonging(&held, &grants), 9);
    assert_eq!(held.len(), 9 + ROOT_AUTHORITY_ANCHOR.len());

    world.revoke(g5.event.grant(), pi)?;
    projector.catch_up(world.book(), world.grants.revision())?;
    assert_eq!(
        check_directly(client.as_ref(), agent_a, &on_x, &read()?, world.now)?,
        Permissionship::HasPermission
    );
    let after = stored(client.as_ref())?;
    assert_eq!(belonging(&after, &[g5.event.grant()]), 0);
    assert_eq!(belonging(&after, &[rp.event.grant()]), 3);
    assert_eq!(belonging(&after, &[g6.event.grant()]), 3);
    Ok(())
}

#[test]
fn replaying_the_same_log_again_changes_nothing() -> TestResult {
    let mut fixture = Fixture::new()?;
    fixture.revoke_g1()?;
    let (_spicedb, client) = engine(None)?;
    let mut projector = fixture.world.projector(&client, 1000)?;
    projector.catch_up(fixture.world.book(), fixture.world.grants.revision())?;
    let once = stored(client.as_ref())?;
    std::fs::remove_file(fixture.world.dir.path().join("projection.json"))?;
    let mut again = fixture.world.projector(&client, 1000)?;
    assert_eq!(again.reached().position, 0);
    again.catch_up(fixture.world.book(), fixture.world.grants.revision())?;
    assert_eq!(stored(client.as_ref())?, once);
    assert_eq!(again.reached().position, fixture.world.grants.revision());
    Ok(())
}

#[test]
fn an_appended_but_uncommitted_grant_event_projects_nothing() -> TestResult {
    let mut world = World::new()?;
    let person_p = world.person("P")?;
    let harness = Harness::new(11)?;
    let leaves = Mutex::new(harness.leaves());
    let mut grants = Grants::open(
        Box::new(move || (leaves.lock().unwrap_or_else(PoisonError::into_inner))()),
        Ed25519Identity::load(&harness.dir.path().join("service.key"))?,
        MemoryRelationships::default(),
        model()?,
        world.root,
    )?;
    let request = |id: &str| -> Result<RootRequest, Box<dyn std::error::Error>> {
        Ok(RootRequest {
            operation: OperationId::generate()?,
            caller: IdentityId::Person(world.root),
            route: Route::Api,
            holder: person_p,
            resource: project(id)?,
            relation: Relation::new("reader")?,
            pass_on: lys_identity::grants::PassOn::UseOnly,
            window: Window::new(T0, None)?,
        })
    };
    let (committed, uncommitted) = (request("x")?, request("z")?);
    let directory = world.directory.projection()?;
    let first = grants.issue_root(directory, &committed, world.now)?;
    harness.fail(Fault::AfterLeafUnreadable);
    let held_back = grants.issue_root(directory, &uncommitted, world.now);
    assert!(held_back.is_err(), "{held_back:?}");
    let uncertain = grants
        .ledger()
        .uncertain()
        .ok_or("the append is not held uncertain")?
        .grant;

    let (_spicedb, client) = engine(None)?;
    let position_file = world.dir.path().join("uncommitted.json");
    let engaged: Arc<Counting> = Arc::clone(&client);
    let mut projector = Projector::open(
        engaged,
        Box::new(FilePosition::new(position_file)),
        1000,
        world.root,
    )?;
    projector.catch_up(grants.book(), grants.revision())?;
    let held = stored(client.as_ref())?;
    assert_eq!(belonging(&held, &[uncertain]), 0);
    assert_eq!(belonging(&held, &[first.event.grant()]), 3);
    assert_eq!(projector.reached().position, position(&first));
    Ok(())
}

/// The server's `SpiceDB` settings for `spicedb`, its key in a file of the
/// test's own.
fn settings(
    spicedb: &SpiceDb,
    dir: &tempfile::TempDir,
    key: &str,
) -> Result<SpiceDbSettings, Box<dyn std::error::Error>> {
    let key_file = dir.path().join("spicedb.key");
    std::fs::write(&key_file, key)?;
    Ok(SpiceDbSettings {
        endpoint: "unused.invalid:8443".to_owned(),
        key_file,
        mirror: "grants".to_owned(),
        grpc: Some(spicedb.address.clone()),
        max_updates_per_write: 1000,
    })
}

#[tokio::test]
async fn the_server_writes_the_schema_once_and_starts_again_without_writing() -> TestResult {
    let spicedb = SpiceDb::start(None)?;
    let dir = tempfile::TempDir::new()?;
    let key = spicedb_support::server::unique("spicedb-test-key");
    let first = Service::start_judging(GRANT_MODEL, Some(settings(&spicedb, &dir, &key)?), |_| {
        Ok(())
    })
    .await;
    assert!(first.is_ok(), "{:?}", first.err());
    let own = SpiceDbClient::connect(&ClientConfig::new(&spicedb.address, &key))?;
    assert_eq!(own.read_schema()?, SchemaRead::Schema(SCHEMA.to_owned()));

    // The second start is the server's own, through service_engaged, over a
    // client of the same store that counts every call the start makes.
    let counted = Counting::around(Arc::new(SpiceDbClient::connect(&ClientConfig::new(
        &spicedb.address,
        &key,
    ))?));
    let engaging: Arc<Counting> = Arc::clone(&counted);
    let said: Arc<Mutex<Option<Started>>> = Arc::new(Mutex::new(None));
    let saying = Arc::clone(&said);
    let second = Service::start_engaged(
        GRANT_MODEL,
        move |config| {
            let clock: Arc<dyn lys_identity_server::spicedb::Clock> =
                Arc::new(lys_identity_server::spicedb::ServiceClock);
            let (engine, started) =
                lys_identity_server::Engine::engage(engaging, clock, 1000, &config.grant_log_dir)?;
            *saying.lock().unwrap_or_else(PoisonError::into_inner) = Some(started);
            Ok(engine)
        },
        |_| Ok(()),
    )
    .await;
    assert!(second.is_ok(), "{:?}", second.err());
    assert_eq!(
        *said.lock().unwrap_or_else(PoisonError::into_inner),
        Some(Started::Unchanged)
    );
    let (schema_reads, schema_writes, _) = spicedb_support::others(&counted);
    let relationship_writes = writes(&counted).len();
    assert_eq!(schema_reads, 1, "the second start reads the schema once");
    assert_eq!(schema_writes, 0, "the second start wrote the schema");
    assert_eq!(
        relationship_writes, 0,
        "the second start wrote relationships"
    );
    assert_eq!(own.read_schema()?, SchemaRead::Schema(SCHEMA.to_owned()));
    Ok(())
}

#[tokio::test]
async fn a_store_holding_another_schema_stops_the_start_and_keeps_its_schema() -> TestResult {
    let spicedb = SpiceDb::start(None)?;
    let key = spicedb_support::server::unique("spicedb-test-key");
    let own = SpiceDbClient::connect(&ClientConfig::new(&spicedb.address, &key))?;
    let differing = SCHEMA.replace(
        "definition person {}",
        "definition person {\n\trelation spare: agent\n}",
    );
    assert_ne!(differing, SCHEMA);
    own.write_schema(WriteSchemaRequest { schema: differing })?;
    let before = own.read_schema()?;
    assert_ne!(before, SchemaRead::Schema(SCHEMA.to_owned()));
    let dir = tempfile::TempDir::new()?;
    let started =
        Service::start_judging(GRANT_MODEL, Some(settings(&spicedb, &dir, &key)?), |_| {
            Ok(())
        })
        .await;
    let Err(refusal) = started else {
        return Err("a store holding another schema was started on".into());
    };
    assert!(
        refusal.to_string().contains("spicedb_schema_mismatch"),
        "{refusal}"
    );
    assert_eq!(own.read_schema()?, before);
    Ok(())
}
