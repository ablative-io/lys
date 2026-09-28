#![cfg(test)]
//! R4: a call after a committed revoke is never admitted. While the
//! projector is behind the revoke, only a check depending on it is refused
//! `StaleDecision`, naming the grant, after its one `CheckPermission` call;
//! unrelated authority is answered; the explain seam gives the same refusal
//! and no path. Once the projector has applied the revoke, the refusal is
//! `permission_revoked`, naming the withdrawn grant.

mod spicedb_support;

use std::error::Error;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;

use lys_identity::IdentityId;
use lys_identity::grants::{ExerciseRequest, GrantError, GrantId, Question, Recorded, Route};
use lys_identity_server::spicedb::explain::{QuestionSet, why};
use lys_identity_server::spicedb::freshness::Fresh;
use lys_identity_server::spicedb::schema::ensure;
use lys_identity_server::spicedb::wire::authzed::api::v1::check_permission_response::Permissionship;
use lys_identity_server::spicedb::{Clock, Evaluator, Projector};
use spicedb_support::server::SpiceDb;
use spicedb_support::{Counting, TestResult, World, checks, position, project, read};

/// A clock standing still at one time.
struct Still(AtomicU64);

impl Clock for Still {
    fn now(&self) -> u64 {
        self.0.load(std::sync::atomic::Ordering::SeqCst)
    }
}

/// P's grant G of read on X to A, from P's root grant on X; Q's grant H of
/// read on Y to E, from Q's root grant on Y; all projected.
struct Fixture {
    world: World,
    projector: Projector,
    evaluator: Evaluator,
    person_p: IdentityId,
    agent_a: IdentityId,
    agent_e: IdentityId,
    g: Recorded,
}

fn fixture(client: &Arc<Counting>) -> Result<Fixture, Box<dyn Error>> {
    let mut world = World::new()?;
    let (on_x, on_y) = (project("x")?, project("y")?);
    let person_p = world.person("P")?;
    let person_q = world.person("Q")?;
    let agent_a = IdentityId::Agent(world.agent(person_p, "A")?);
    let agent_e = IdentityId::Agent(world.agent(person_q, "E")?);
    let (pi, qi) = (IdentityId::Person(person_p), IdentityId::Person(person_q));
    let root_x = world.root_grant(person_p, &on_x, None)?;
    let root_y = world.root_grant(person_q, &on_y, None)?;
    let g = world.delegate(root_x.event.grant(), (pi, agent_a), &on_x, None)??;
    world.delegate(root_y.event.grant(), (qi, agent_e), &on_y, None)??;
    ensure(client.as_ref())?;
    let mut projector = world.projector(client, 1000)?;
    projector.catch_up(world.book(), world.grants.revision())?;
    let clock: Arc<dyn Clock> = Arc::new(Still(AtomicU64::new(world.now)));
    let asking: Arc<Counting> = Arc::clone(client);
    Ok(Fixture {
        evaluator: Evaluator::new(asking, clock),
        world,
        projector,
        person_p: pi,
        agent_a,
        agent_e,
        g,
    })
}

fn request(caller: IdentityId, on: &str) -> Result<ExerciseRequest, Box<dyn Error>> {
    Ok(ExerciseRequest {
        caller,
        route: Route::Tool,
        resource: project(on)?,
        action: read()?,
    })
}

impl Fixture {
    /// Ask whether `caller` may read project `on` now, at the moment of the
    /// action, with the projector as it stands.
    fn ask(
        &mut self,
        caller: IdentityId,
        on: &str,
    ) -> Result<Result<GrantId, GrantError>, Box<dyn Error>> {
        let reached = self.projector.reached().clone();
        let fresh = Fresh::new(&self.evaluator, &reached);
        let now = self.world.now;
        Ok(self
            .world
            .check_with(&request(caller, on)?, &fresh, now)?
            .map(|permit| permit.grant))
    }
}

#[test]
fn a_projector_behind_a_revoke_refuses_only_what_depends_on_it() -> TestResult {
    let spicedb = SpiceDb::start(None)?;
    let client = spicedb.client()?;
    let mut fixture = fixture(&client)?;
    assert!(fixture.ask(fixture.agent_a, "x")?.is_ok());
    let behind = fixture.projector.reached().position;
    let revoke = fixture
        .world
        .revoke(fixture.g.event.grant(), fixture.person_p)?;
    let g = fixture.g.event.grant().to_string();
    let mut admitted = 0;

    let before = checks(&client).len();
    let refused = fixture.ask(fixture.agent_a, "x")?;
    admitted += usize::from(refused.is_ok());
    let Err(stale) = refused else {
        return Err("a request after a committed revoke was admitted".into());
    };
    assert!(
        matches!(
            &stale,
            GrantError::StaleDecision { required, projected, grant: Some(named) }
                if *required == position(&revoke) && *projected == behind && *named == g
        ),
        "{stale:?}"
    );
    let words = stale.to_string();
    for part in [g.clone(), position(&revoke).to_string(), behind.to_string()] {
        assert!(words.contains(&part), "{words} names no {part}");
    }
    let asked = checks(&client)[before..].to_vec();
    assert_eq!(asked.len(), 1);
    let answered = asked[0].1.as_ref().map(|answer| answer.permissionship);
    assert_eq!(answered, Some(i32::from(Permissionship::HasPermission)));

    let unrelated = fixture.ask(fixture.agent_e, "y")?;
    assert!(unrelated.is_ok(), "{unrelated:?}");
    let again = fixture.ask(fixture.agent_a, "x")?;
    admitted += usize::from(again.is_ok());
    let names_g = matches!(
        &again,
        Err(GrantError::StaleDecision { grant: Some(named), .. }) if *named == g
    );
    assert!(names_g, "{again:?}");

    let reached = fixture.projector.reached().clone();
    let (on_x, action) = (project("x")?, read()?);
    let question = Question {
        subject: fixture.agent_a,
        resource: &on_x,
        action: &action,
    };
    let explained = why(
        &fixture.evaluator,
        fixture.world.book(),
        fixture.world.grants.revision(),
        &reached,
        &mut QuestionSet::new(),
        &question,
    );
    let names_g = matches!(
        &explained,
        Err(GrantError::StaleDecision { grant: Some(named), .. }) if *named == g
    );
    assert!(names_g, "{explained:?}");

    fixture
        .projector
        .catch_up(fixture.world.book(), fixture.world.grants.revision())?;
    assert_eq!(
        fixture.projector.reached().position,
        fixture.world.grants.revision()
    );
    let revoked = fixture.ask(fixture.agent_a, "x")?;
    admitted += usize::from(revoked.is_ok());
    assert_eq!(
        revoked,
        Err(GrantError::PermissionRevoked { grant: g.clone() })
    );
    let words = revoked
        .err()
        .map(|refusal| refusal.to_string())
        .unwrap_or_default();
    assert!(words.starts_with("permission_revoked"), "{words}");
    assert!(words.contains(&g), "{words}");
    assert_eq!(admitted, 0);
    Ok(())
}
