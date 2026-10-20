#![cfg(test)]

use std::collections::BTreeSet;
use std::error::Error;
use std::os::unix::fs::PermissionsExt;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use lys_core::Ed25519Identity;
use lys_log_store::FileLeafStore;

use super::{
    Action, ExerciseRequest, GrantError, Grants, MemoryRelationships, Model, ObjectRef, PassOn,
    Relation, Relationship, RelationshipStore, Resource, RootRequest, Route, Window,
};
use crate::{
    Actor, AuthMethod, Directory, IdentityId, LoginBinding, OperationId, Profile, Provenance,
    Transition,
};

#[derive(Clone)]
struct Counted {
    inner: MemoryRelationships,
    unrelated: Arc<BTreeSet<Relationship>>,
    rows: Arc<AtomicUsize>,
}

impl Counted {
    fn about(
        &self,
        subject: IdentityId,
        resource: &Resource,
    ) -> Result<BTreeSet<Relationship>, GrantError> {
        self.inner.read_about(subject, resource)
    }
}

impl RelationshipStore for Counted {
    fn admit_resource(&self, resource: &Resource) -> Result<(), GrantError> {
        self.inner.admit_resource(resource)
    }
    fn revision(&self) -> Result<u64, GrantError> {
        self.inner.revision()
    }
    fn write(
        &mut self,
        revision: u64,
        touch: &[Relationship],
        delete: &[Relationship],
    ) -> Result<(), GrantError> {
        self.inner.write(revision, touch, delete)
    }
    fn read(&self) -> Result<BTreeSet<Relationship>, GrantError> {
        let mut held = self.inner.read()?;
        held.extend(self.unrelated.iter().cloned());
        self.rows.fetch_add(held.len(), Ordering::Relaxed);
        Ok(held)
    }
    fn read_about(
        &self,
        subject: IdentityId,
        resource: &Resource,
    ) -> Result<BTreeSet<Relationship>, GrantError> {
        let held = self.about(subject, resource)?;
        self.rows.fetch_add(held.len(), Ordering::Relaxed);
        Ok(held)
    }
}

struct Fixture {
    directory: Directory<FileLeafStore>,
    grants: Grants<FileLeafStore, Counted>,
    asked: ExerciseRequest,
    rows: Arc<AtomicUsize>,
    temporary: tempfile::TempDir,
}

fn fixture(others: usize) -> Result<Fixture, Box<dyn Error>> {
    let temporary = tempfile::tempdir()?;
    let directory_path = temporary.path().join("directory");
    let grant_path = temporary.path().join("grants");
    FileLeafStore::create(&directory_path, "fixture/directory")?;
    FileLeafStore::create(&grant_path, "fixture/grants")?;
    let key_path = temporary.path().join("service.key");
    std::fs::write(&key_path, [7; 32])?;
    std::fs::set_permissions(&key_path, std::fs::Permissions::from_mode(0o600))?;
    let mut directory = Directory::open(
        Box::new(move || FileLeafStore::open(&directory_path)),
        Ed25519Identity::load(&key_path)?,
    )?;
    let actor = Actor::new(
        LoginBinding::new("https://issuer.test", "administrator")?,
        Provenance::new(AuthMethod::Oidc, 1),
    );
    let founded = directory.found_organisation(
        actor.clone(),
        OperationId::generate()?,
        Profile::new("Example organisation")?,
        Profile::new("holder")?,
        1,
    )?;
    let (person, organisation) = (founded.founder, founded.organisation);
    directory.transition(
        actor,
        OperationId::generate()?,
        IdentityId::Person(person),
        Transition::Activate,
        "",
        1,
    )?;
    let subject = IdentityId::Person(person);
    let unrelated = (0..others)
        .map(|n| Relationship {
            resource: ObjectRef {
                kind: "grant".to_owned(),
                id: format!("unrelated-{n:06}"),
            },
            relation: "holder".to_owned(),
            subject: ObjectRef {
                kind: "person".to_owned(),
                id: format!("other-{n:06}"),
            },
            subject_relation: None,
            ends_at: None,
        })
        .collect();
    let rows = Arc::new(AtomicUsize::new(0));
    let engine = Counted {
        inner: MemoryRelationships::default(),
        unrelated: Arc::new(unrelated),
        rows: Arc::clone(&rows),
    };
    let model = Model::new(
        1,
        [
            (
                Relation::new("reader")?,
                BTreeSet::from([Action::new("read")?]),
            ),
            (
                Relation::new("administrator")?,
                [
                    "organisation.administer",
                    "organisation.admit",
                    "organisation.grant",
                    "organisation.read",
                ]
                .into_iter()
                .map(Action::new)
                .collect::<Result<BTreeSet<_>, _>>()?,
            ),
        ],
    )?;
    let mut grants = Grants::open(
        Box::new(move || FileLeafStore::open(&grant_path)),
        Ed25519Identity::load(&key_path)?,
        engine,
        model,
        person,
    )?;
    // The founder's administrator grant, which the five roots rest on.
    grants.found_organisation(directory.projection()?, organisation, 1)?;
    for n in 0..5 {
        grants.issue_root(
            directory.projection()?,
            &RootRequest {
                operation: OperationId::generate()?,
                caller: subject,
                route: Route::Api,
                holder: person,
                resource: Resource::new("project", &format!("asked-{n}"))?,
                relation: Relation::new("reader")?,
                pass_on: PassOn::UseOnly,
                window: Window::new(1, None)?,
            },
            2,
        )?;
    }
    rows.store(0, Ordering::Relaxed);
    Ok(Fixture {
        directory,
        grants,
        asked: ExerciseRequest {
            caller: subject,
            route: Route::Api,
            resource: Resource::new("project", "asked-0")?,
            action: Action::new("read")?,
        },
        rows,
        temporary,
    })
}

#[test]
fn a_decision_reads_only_the_subjects_five_grants() -> Result<(), Box<dyn Error>> {
    let mut f = fixture(1_000)?;
    let permit = f
        .grants
        .explain(f.directory.projection()?, &f.asked, 3, None)?;
    assert_eq!(permit.path.len(), 1);
    assert_eq!(
        f.rows.load(Ordering::Relaxed),
        18,
        "only the subject's six grants, the founding one among them, reach the frame"
    );
    assert!(f.temporary.path().exists());
    Ok(())
}

fn median(f: &mut Fixture) -> Result<Duration, Box<dyn Error>> {
    let mut runs = Vec::new();
    for _ in 0..10 {
        let started = Instant::now();
        f.grants
            .explain(f.directory.projection()?, &f.asked, 3, None)?;
        runs.push(started.elapsed());
    }
    runs.sort();
    Ok(runs[5])
}

#[test]
fn unrelated_relationships_do_not_grow_decision_time() -> Result<(), Box<dyn Error>> {
    let mut small = fixture(1_000)?;
    let mut large = fixture(10_000)?;
    let one = median(&mut small)?;
    let ten = median(&mut large)?;
    eprintln!("ten runs each: 1x median={one:?}, 10x median={ten:?}");
    assert!(
        ten <= one * 3 + Duration::from_micros(100),
        "unrelated rows grow the decision: 1x={one:?}, 10x={ten:?}"
    );
    Ok(())
}
