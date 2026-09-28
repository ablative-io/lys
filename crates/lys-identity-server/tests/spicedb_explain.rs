#![cfg(test)]
//! R6: why an identity can or cannot do a thing, from one call to the one
//! evaluator's traced check, and who can, through `LookupSubjects`, every
//! answer of one question set read at one revision T.

mod spicedb_support;

use std::error::Error;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;

use lys_identity::grants::{Question, Recorded};
use lys_identity::{IdentityId, PersonId};
use lys_identity_server::spicedb::explain::{Explanation, QuestionSet, Verdict, why};
use lys_identity_server::spicedb::lookup::who;
use lys_identity_server::spicedb::schema::{digest, ensure};
use lys_identity_server::spicedb::{Clock, Evaluator, Projector};
use spicedb_support::server::SpiceDb;
use spicedb_support::{
    Counting, TestResult, World, checks, consistency, last_written, lookups, project, read,
};

/// A clock standing still at one time.
struct Still(AtomicU64);

impl Clock for Still {
    fn now(&self) -> u64 {
        self.0.load(std::sync::atomic::Ordering::SeqCst)
    }
}

/// P's root grant on X; P's grant PA of read on X to A; B holds nothing on X.
struct Fixture {
    world: World,
    projector: Projector,
    evaluator: Evaluator,
    person_p: PersonId,
    agent_a: IdentityId,
    agent_b: IdentityId,
    pa: Recorded,
}

fn fixture(client: &Arc<Counting>) -> Result<Fixture, Box<dyn Error>> {
    let mut world = World::new()?;
    let on_x = project("x")?;
    let person_p = world.person("P")?;
    let agent_a = IdentityId::Agent(world.agent(person_p, "A")?);
    let agent_b = IdentityId::Agent(world.agent(person_p, "B")?);
    let root = world.root_grant(person_p, &on_x, None)?;
    let pa = world.delegate(
        root.event.grant(),
        (IdentityId::Person(person_p), agent_a),
        &on_x,
        None,
    )??;
    ensure(client.as_ref())?;
    let mut projector = world.projector(client, 1000)?;
    projector.catch_up(world.book(), world.grants.revision())?;
    let clock: Arc<dyn Clock> = Arc::new(Still(AtomicU64::new(world.now)));
    let asking: Arc<Counting> = Arc::clone(client);
    Ok(Fixture {
        evaluator: Evaluator::new(asking, clock),
        world,
        projector,
        person_p,
        agent_a,
        agent_b,
        pa,
    })
}

impl Fixture {
    fn why(
        &self,
        subject: IdentityId,
        set: &mut QuestionSet,
    ) -> Result<Explanation, Box<dyn Error>> {
        let (on_x, action) = (project("x")?, read()?);
        let question = Question {
            subject,
            resource: &on_x,
            action: &action,
        };
        Ok(why(
            &self.evaluator,
            self.world.book(),
            self.world.grants.revision(),
            self.projector.reached(),
            set,
            &question,
        )?)
    }

    /// The plain check's verdict for `subject`, as the projection stands.
    fn plain(&self, subject: IdentityId) -> Result<bool, Box<dyn Error>> {
        let (on_x, action) = (project("x")?, read()?);
        let question = Question {
            subject,
            resource: &on_x,
            action: &action,
        };
        let token = self.projector.reached().token.clone();
        Ok(self.evaluator.check(&question, token.as_deref())?.permitted)
    }

    /// The traced check's verdict for `subject` at the exact snapshot `at`.
    fn traced_at(&self, subject: IdentityId, at: &str) -> Result<bool, Box<dyn Error>> {
        let (on_x, action) = (project("x")?, read()?);
        let question = Question {
            subject,
            resource: &on_x,
            action: &action,
        };
        let token = self.projector.reached().token.clone();
        Ok(self
            .evaluator
            .check_traced(&question, token.as_deref(), Some(at))?
            .permitted)
    }
}

/// The explanation's verdict, after exactly one traced check.
fn explained_once(
    fixture: &Fixture,
    client: &Counting,
    subject: IdentityId,
) -> Result<Explanation, Box<dyn Error>> {
    let before = checks(client).len();
    let explanation = fixture.why(subject, &mut QuestionSet::new())?;
    let made = checks(client)[before..].to_vec();
    assert_eq!(made.len(), 1);
    assert!(
        made[0].0.with_tracing,
        "the explanation's check was not traced"
    );
    let permitted = matches!(explanation.verdict, Verdict::Permitted { .. });
    assert_eq!(permitted, fixture.plain(subject)?);
    Ok(explanation)
}

#[test]
fn why_answers_yes_with_the_path_and_no_with_the_named_reason() -> TestResult {
    let spicedb = SpiceDb::start(None)?;
    let client = spicedb.client()?;
    let mut fixture = fixture(&client)?;

    let yes = explained_once(&fixture, &client, fixture.agent_a)?;
    assert_eq!(
        yes.verdict,
        Verdict::Permitted {
            grant: Some(fixture.pa.event.grant()),
            path: vec![fixture.agent_a, IdentityId::Person(fixture.person_p)],
            responsible: Some(fixture.person_p),
        }
    );
    assert_eq!(yes.resource, project("x")?);
    assert_eq!(yes.action, read()?);
    assert_eq!(yes.policy.schema_sha256, digest());
    assert!(!yes.policy.revision.is_empty());

    let no_grant = explained_once(&fixture, &client, fixture.agent_b)?;
    assert_eq!(
        no_grant.verdict,
        Verdict::Refused {
            reason: "no_grant".to_owned(),
            grant: None,
        }
    );

    let (pa, by) = (
        fixture.pa.event.grant(),
        IdentityId::Person(fixture.person_p),
    );
    fixture.world.revoke(pa, by)?;
    fixture
        .projector
        .catch_up(fixture.world.book(), fixture.world.grants.revision())?;
    let revoked = explained_once(&fixture, &client, fixture.agent_a)?;
    assert_eq!(
        revoked.verdict,
        Verdict::Refused {
            reason: "permission_revoked".to_owned(),
            grant: Some(pa.to_string()),
        }
    );
    assert_eq!(revoked.policy.schema_sha256, digest());
    Ok(())
}

#[test]
fn one_question_set_reads_every_answer_at_one_revision() -> TestResult {
    let spicedb = SpiceDb::start(None)?;
    let client = spicedb.client()?;
    let fixture = fixture(&client)?;
    let projected = last_written(&client).ok_or("the projector wrote nothing")?;
    let before = checks(&client).len();
    let mut set = QuestionSet::new();

    let why_a = fixture.why(fixture.agent_a, &mut set)?;
    let why_b = fixture.why(fixture.agent_b, &mut set)?;
    let resource = project("x")?;
    let who_can = who(
        &fixture.evaluator,
        fixture.projector.reached(),
        &mut set,
        &resource,
        &read()?,
    )?;

    let made = checks(&client)[before..].to_vec();
    assert_eq!(made.len(), 2);
    assert_eq!(
        consistency(made[0].0.consistency.as_ref()),
        format!("at_least_as_fresh {projected}")
    );
    let t = made[0]
        .1
        .as_ref()
        .and_then(|answer| answer.checked_at.as_ref())
        .map(|token| token.token.clone())
        .ok_or("the first check answered no revision")?;
    assert_eq!(
        consistency(made[1].0.consistency.as_ref()),
        format!("at_exact_snapshot {t}")
    );
    let looked = lookups(&client);
    assert!(!looked.is_empty());
    for lookup in &looked {
        assert_eq!(
            consistency(lookup.consistency.as_ref()),
            format!("at_exact_snapshot {t}")
        );
    }
    assert_eq!(why_a.policy.revision, t);
    assert_eq!(why_b.policy.revision, t);
    assert_eq!(set.revision(), Some(t.as_str()));

    let before = checks(&client).len();
    let person = IdentityId::Person(fixture.person_p);
    for identity in [person, fixture.agent_a, fixture.agent_b] {
        let permitted = fixture.traced_at(identity, &t)?;
        assert_eq!(who_can.contains(&identity), permitted, "{identity}");
    }
    let snapshots = checks(&client)[before..].to_vec();
    assert_eq!(snapshots.len(), 3);
    for (question, _) in &snapshots {
        assert_eq!(
            consistency(question.consistency.as_ref()),
            format!("at_exact_snapshot {t}")
        );
    }
    assert!(who_can.contains(&fixture.agent_a));
    assert!(!who_can.contains(&fixture.agent_b));
    Ok(())
}
