#![cfg(test)]
//! Shared support for the road-step-2 tests: a disposable `SpiceDB` per
//! test, a wrapper around the client that counts and keeps every call it
//! forwards, a directory with grants to project, and the test's own direct
//! reads, checks and writes against its store.
//!
//! Every relationship write a test makes is built here and reaches only the
//! test's own disposable store. Expected relationships are rendered as
//! text by the test from the recorded grant contract and schema, never
//! produced by the mapping under test. The last test here uses every helper,
//! so each test binary that includes this module uses all of it.

pub mod server;

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::os::unix::fs::PermissionsExt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex, PoisonError};

use lys_core::Ed25519Identity;
use lys_identity::grants::relationships::{EXERCISE, resource_object};
use lys_identity::grants::{
    Action, DelegateRequest, Evaluator as Decides, ExerciseRequest, GrantBook, GrantChange,
    GrantError, GrantEvent, GrantId, GrantLedger, GrantParts, Grants, MemoryRelationships, Model,
    ObjectRef, PassOn, Permit, RecipientKind, Recorded, Relation, Resource, RevokeRequest,
    RootRequest, Route, Source, Window, sign_grant_event,
};
use lys_identity::log::Reopen;
use lys_identity::{
    Actor, AgentId, AuthMethod, Directory, IdentityId, LoginBinding, OperationId, PersonId,
    Profile, Provenance, SNAPSHOT_EVERY, Transition,
};
use lys_identity_server::spicedb::check::{CheckAnswer, CheckQuestion};
use lys_identity_server::spicedb::freshness::Fresh;
use lys_identity_server::spicedb::lookup::{LookupAnswer, LookupQuestion};
use lys_identity_server::spicedb::projector::{
    RelationshipsWrite, RelationshipsWritten, number, object_reference,
};
use lys_identity_server::spicedb::schema::{SchemaWrite, SchemaWritten};
use lys_identity_server::spicedb::wire::authzed::api::v1::check_permission_response::Permissionship;
use lys_identity_server::spicedb::wire::authzed::api::v1::consistency::Requirement;
use lys_identity_server::spicedb::wire::authzed::api::v1::relationship_update::Operation;
use lys_identity_server::spicedb::wire::authzed::api::v1::{
    CheckPermissionRequest, Consistency, ObjectReference, ReadRelationshipsRequest,
    ReadRelationshipsResponse, Relationship, RelationshipFilter, RelationshipUpdate,
    SubjectReference, WriteRelationshipsRequest,
};
use lys_identity_server::spicedb::{
    Clock, Evaluator, FilePosition, Projector, SchemaRead, SpiceDbApi, SpiceDbError,
};
use lys_log_store::FileLeafStore;

/// A test's result.
pub type TestResult = Result<(), Box<dyn Error>>;

/// The first second every fixture's windows start at.
pub const T0: u64 = 1_800_000_000;

/// One call the wrapper forwarded.
#[derive(Debug, Clone)]
pub enum Call {
    /// A schema read.
    ReadSchema,
    /// A schema write.
    WriteSchema,
    /// A relationship write, as sent.
    Write(RelationshipsWrite),
    /// A relationship read.
    Read,
    /// A permission check, as sent, with the answer when there was one.
    Check(Box<CheckQuestion>, Option<Box<CheckAnswer>>),
    /// A subject lookup, as sent.
    Lookup(Box<LookupQuestion>),
}

/// A clock that stands at one second.
pub struct Fixed(pub u64);

impl Clock for Fixed {
    fn now(&self) -> u64 {
        self.0
    }
}

/// A pause after one write: say so on `reached`, then wait for `resume`.
pub struct Pause {
    /// The write, counting from 1, after which the wrapper pauses.
    pub after: usize,
    /// Told once that write is answered.
    pub reached: Sender<()>,
    /// Waited on before the write's answer is handed back.
    pub resume: Receiver<()>,
}

/// The client, wrapped: every call is kept, and a test may strip each
/// check's context, fail one write without sending it, or pause after one.
pub struct Counting {
    /// The client calls are forwarded to.
    pub inner: Arc<dyn SpiceDbApi>,
    /// Every call forwarded, in order.
    pub calls: Mutex<Vec<Call>>,
    /// Whether each check's context is removed before it is sent.
    pub strip_context: AtomicBool,
    /// The write, counting from 1, that fails without being sent.
    pub fail_write: Mutex<Option<usize>>,
    /// The pause after a write, when one is set.
    pub pause: Mutex<Option<Pause>>,
    /// The revision token each answered write was written at, in order.
    pub written: Mutex<Vec<String>>,
}

impl Counting {
    /// `inner`, wrapped.
    pub fn around(inner: Arc<dyn SpiceDbApi>) -> Arc<Self> {
        Arc::new(Self {
            inner,
            calls: Mutex::new(Vec::new()),
            strip_context: AtomicBool::new(false),
            fail_write: Mutex::new(None),
            pause: Mutex::new(None),
            written: Mutex::new(Vec::new()),
        })
    }

    fn keep(&self, call: Call) -> usize {
        let mut calls = self.calls.lock().unwrap_or_else(PoisonError::into_inner);
        calls.push(call);
        calls
            .iter()
            .filter(|kept| matches!(kept, Call::Write(_)))
            .count()
    }
}

impl SpiceDbApi for Counting {
    fn address(&self) -> &str {
        self.inner.address()
    }

    fn read_schema(&self) -> Result<SchemaRead, SpiceDbError> {
        self.keep(Call::ReadSchema);
        self.inner.read_schema()
    }

    fn write_schema(&self, request: SchemaWrite) -> Result<SchemaWritten, SpiceDbError> {
        self.keep(Call::WriteSchema);
        self.inner.write_schema(request)
    }

    fn write_relationships(
        &self,
        request: RelationshipsWrite,
    ) -> Result<RelationshipsWritten, SpiceDbError> {
        let number = self.keep(Call::Write(request.clone()));
        let failing = *self
            .fail_write
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if failing == Some(number) {
            return Err(SpiceDbError::Unreachable {
                operation: "WriteRelationships",
                address: self.inner.address().to_owned(),
                reason: format!("write {number} was failed by the test before it was sent"),
            });
        }
        let answer = self.inner.write_relationships(request);
        if let Some(token) = answer
            .as_ref()
            .ok()
            .and_then(|written| written.written_at.as_ref())
        {
            let mut written = self.written.lock().unwrap_or_else(PoisonError::into_inner);
            written.push(token.token.clone());
        }
        let pause = self.pause.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(pause) = pause.as_ref().filter(|pause| pause.after == number) {
            pause.reached.send(()).ok();
            pause.resume.recv().ok();
        }
        answer
    }

    fn read_relationships(
        &self,
        request: ReadRelationshipsRequest,
    ) -> Result<Vec<ReadRelationshipsResponse>, SpiceDbError> {
        self.keep(Call::Read);
        self.inner.read_relationships(request)
    }

    fn check(&self, mut request: CheckQuestion) -> Result<CheckAnswer, SpiceDbError> {
        if self.strip_context.load(Ordering::SeqCst) {
            request.context = None;
        }
        let answer = self.inner.check(request.clone());
        let kept = answer.as_ref().ok().cloned().map(Box::new);
        self.keep(Call::Check(Box::new(request), kept));
        answer
    }

    fn lookup(&self, request: LookupQuestion) -> Result<Vec<LookupAnswer>, SpiceDbError> {
        self.keep(Call::Lookup(Box::new(request.clone())));
        self.inner.lookup(request)
    }
}

/// Every relationship write the wrapper forwarded, as sent.
pub fn writes(client: &Counting) -> Vec<RelationshipsWrite> {
    let calls = client.calls.lock().unwrap_or_else(PoisonError::into_inner);
    calls
        .iter()
        .filter_map(|call| match call {
            Call::Write(write) => Some(write.clone()),
            _ => None,
        })
        .collect()
}

/// The revision token of the last write `SpiceDB` answered.
pub fn last_written(client: &Counting) -> Option<String> {
    let written = client
        .written
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    written.last().cloned()
}

/// Every check the wrapper forwarded, as sent, with its answer.
pub fn checks(client: &Counting) -> Vec<(CheckQuestion, Option<CheckAnswer>)> {
    let calls = client.calls.lock().unwrap_or_else(PoisonError::into_inner);
    calls
        .iter()
        .filter_map(|call| match call {
            Call::Check(question, answer) => Some((
                question.as_ref().clone(),
                answer.as_ref().map(|answer| answer.as_ref().clone()),
            )),
            _ => None,
        })
        .collect()
}

/// Every lookup the wrapper forwarded, as sent.
pub fn lookups(client: &Counting) -> Vec<LookupQuestion> {
    let calls = client.calls.lock().unwrap_or_else(PoisonError::into_inner);
    calls
        .iter()
        .filter_map(|call| match call {
            Call::Lookup(question) => Some(question.as_ref().clone()),
            _ => None,
        })
        .collect()
}

/// How many calls of each other kind the wrapper forwarded: schema reads,
/// schema writes and relationship reads.
pub fn others(client: &Counting) -> (usize, usize, usize) {
    let calls = client.calls.lock().unwrap_or_else(PoisonError::into_inner);
    let count = |kind: fn(&Call) -> bool| calls.iter().filter(|call| kind(call)).count();
    (
        count(|call| matches!(call, Call::ReadSchema)),
        count(|call| matches!(call, Call::WriteSchema)),
        count(|call| matches!(call, Call::Read)),
    )
}

/// The revision a consistency asks for, as words: `at_least_as_fresh <t>`,
/// `at_exact_snapshot <t>`, `fully_consistent` or `minimize_latency`.
pub fn consistency(asked: Option<&Consistency>) -> String {
    match asked.and_then(|asked| asked.requirement.as_ref()) {
        Some(Requirement::AtLeastAsFresh(token)) => format!("at_least_as_fresh {}", token.token),
        Some(Requirement::AtExactSnapshot(token)) => format!("at_exact_snapshot {}", token.token),
        Some(Requirement::FullyConsistent(_)) => "fully_consistent".to_owned(),
        Some(Requirement::MinimizeLatency(_)) => "minimize_latency".to_owned(),
        None => "none".to_owned(),
    }
}

fn object_text(object: Option<&ObjectReference>) -> String {
    object.map_or_else(
        || "?".to_owned(),
        |object| format!("{}:{}", object.object_type, object.object_id),
    )
}

/// A relationship as text: `type:id#relation@type:id`, with its caveat and
/// context as `[caveat key=value ...]`, numbers written whole.
pub fn render(relationship: &Relationship) -> String {
    let subject = relationship
        .subject
        .as_ref()
        .and_then(|subject| subject.object.as_ref());
    let mut text = format!(
        "{}#{}@{}",
        object_text(relationship.resource.as_ref()),
        relationship.relation,
        object_text(subject)
    );
    if let Some(caveat) = &relationship.optional_caveat {
        let context: Vec<String> = caveat
            .context
            .iter()
            .flat_map(|context| context.fields.iter())
            .map(|(key, value)| match &value.kind {
                Some(prost_types::value::Kind::NumberValue(number)) => {
                    format!("{key}={number:.0}")
                }
                other => format!("{key}={other:?}"),
            })
            .collect();
        text = format!("{text} [{} {}]", caveat.caveat_name, context.join(" "));
    }
    text
}

/// Every relationship the store holds, rendered and sorted.
pub fn stored(client: &dyn SpiceDbApi) -> Result<Vec<String>, Box<dyn Error>> {
    let mut held = Vec::new();
    for kind in ["grant", "resource", "root_authority"] {
        let answers = client.read_relationships(ReadRelationshipsRequest {
            consistency: Some(Consistency {
                requirement: Some(Requirement::FullyConsistent(true)),
            }),
            relationship_filter: Some(RelationshipFilter {
                resource_type: kind.to_owned(),
                optional_resource_id: String::new(),
                optional_resource_id_prefix: String::new(),
                optional_relation: String::new(),
                optional_subject_filter: None,
            }),
            optional_limit: 0,
            optional_cursor: None,
        })?;
        held.extend(
            answers
                .iter()
                .filter_map(|answer| answer.relationship.as_ref())
                .map(render),
        );
    }
    held.sort();
    Ok(held)
}

/// How many of `held` are relationships of one of `grants`: its resource
/// relationships, whose subject is the grant, and its holder and standing
/// relationships, which are on the grant.
pub fn belonging(held: &[String], grants: &[GrantId]) -> usize {
    held.iter()
        .filter(|line| {
            grants.iter().any(|grant| {
                line.starts_with(&format!("grant:{grant}#"))
                    || (line.starts_with("resource:") && line.ends_with(&format!("@grant:{grant}")))
            })
        })
        .count()
}

/// The test's own check against the store: may `identity` perform
/// `action` on `resource` at `now`, read fully consistently.
pub fn check_directly(
    client: &dyn SpiceDbApi,
    identity: IdentityId,
    resource: &Resource,
    action: &Action,
    now: u64,
) -> Result<Permissionship, Box<dyn Error>> {
    let mut fields = BTreeMap::new();
    fields.insert("now".to_owned(), number(now));
    let answer = client.check(CheckPermissionRequest {
        consistency: Some(Consistency {
            requirement: Some(Requirement::FullyConsistent(true)),
        }),
        resource: Some(object_reference(&resource_object(resource, action))),
        permission: EXERCISE.to_owned(),
        subject: Some(SubjectReference {
            object: Some(object_reference(&ObjectRef::identity(identity))),
            optional_relation: String::new(),
        }),
        context: Some(prost_types::Struct { fields }),
        with_tracing: false,
    })?;
    Ok(Permissionship::try_from(answer.permissionship)?)
}

/// The test's own write of one relationship, `resource#relation@subject`,
/// to its store. It needs no schema beyond the objects named.
pub fn write_one(
    client: &dyn SpiceDbApi,
    (resource, relation, subject): (&ObjectRef, &str, &ObjectRef),
) -> Result<(), Box<dyn Error>> {
    client.write_relationships(WriteRelationshipsRequest {
        updates: vec![RelationshipUpdate {
            operation: Operation::Touch.into(),
            relationship: Some(Relationship {
                resource: Some(object_reference(resource)),
                relation: relation.to_owned(),
                subject: Some(SubjectReference {
                    object: Some(object_reference(subject)),
                    optional_relation: String::new(),
                }),
                optional_caveat: None,
                optional_expires_at: None,
            }),
        }],
        optional_preconditions: Vec::new(),
        optional_transaction_metadata: None,
    })?;
    Ok(())
}

/// The action every fixture grants.
pub fn read() -> Result<Action, GrantError> {
    Action::new("read")
}

/// The model: `reader` carries read alone, so each grant has one resource
/// relationship.
pub fn model() -> Result<Model, GrantError> {
    Model::new(1, [(Relation::new("reader")?, BTreeSet::from([read()?]))])
}

fn administrator() -> Result<Actor, Box<dyn Error>> {
    Ok(Actor::new(
        LoginBinding::new("https://issuer.test", "administrator")?,
        Provenance::new(AuthMethod::Oidc, T0),
    ))
}

fn reopen(path: std::path::PathBuf) -> Reopen<FileLeafStore> {
    Box::new(move || FileLeafStore::open(&path))
}

/// A directory of active identities, its root authority, and the grants on
/// their own log.
pub struct World {
    /// Holds the logs, the key and the projector's position.
    pub dir: tempfile::TempDir,
    /// The directory.
    pub directory: Directory<FileLeafStore>,
    /// The grants.
    pub grants: Grants<FileLeafStore, MemoryRelationships>,
    /// The directory's root authority.
    pub root: PersonId,
    /// When every request is made.
    pub now: u64,
}

impl World {
    /// A fresh world whose root authority is a person of its own.
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let dir = tempfile::TempDir::new()?;
        FileLeafStore::create(&dir.path().join("log"), "example.test/lys/directory")?;
        FileLeafStore::create(&dir.path().join("grants"), "example.test/lys/grants")?;
        let key = dir.path().join("service.key");
        std::fs::write(&key, [7; 32])?;
        std::fs::set_permissions(&key, std::fs::Permissions::from_mode(0o600))?;
        let mut directory =
            Directory::open(reopen(dir.path().join("log")), Ed25519Identity::load(&key)?)?;
        let (root, _) = directory.register_person(
            administrator()?,
            OperationId::generate()?,
            Profile::new("Root authority")?,
            T0,
        )?;
        directory.transition(
            administrator()?,
            OperationId::generate()?,
            IdentityId::Person(root),
            Transition::Activate,
            "",
            T0,
        )?;
        let grants = Grants::open(
            reopen(dir.path().join("grants")),
            Ed25519Identity::load(&key)?,
            MemoryRelationships::default(),
            model()?,
            root,
        )?;
        Ok(Self {
            dir,
            directory,
            grants,
            root,
            now: T0 + 10,
        })
    }

    fn activate(&mut self, identity: IdentityId) -> Result<(), Box<dyn Error>> {
        self.directory.transition(
            administrator()?,
            OperationId::generate()?,
            identity,
            Transition::Activate,
            "",
            T0,
        )?;
        Ok(())
    }

    /// A new active person.
    pub fn person(&mut self, name: &str) -> Result<PersonId, Box<dyn Error>> {
        let (id, _) = self.directory.register_person(
            administrator()?,
            OperationId::generate()?,
            Profile::new(name)?,
            T0,
        )?;
        self.activate(IdentityId::Person(id))?;
        Ok(id)
    }

    /// A new active agent `of` answers for.
    pub fn agent(&mut self, of: PersonId, name: &str) -> Result<AgentId, Box<dyn Error>> {
        let (id, _) = self.directory.register_agent(
            administrator()?,
            OperationId::generate()?,
            of,
            Profile::new(name)?,
            T0,
        )?;
        self.activate(IdentityId::Agent(id))?;
        Ok(id)
    }

    /// The root authority issues `holder` a root grant of read on
    /// `resource`, which it may pass on to agents and people, ending at `ends`.
    pub fn root_grant(
        &mut self,
        holder: PersonId,
        resource: &Resource,
        ends: Option<u64>,
    ) -> Result<Recorded, Box<dyn Error>> {
        let request = RootRequest {
            operation: OperationId::generate()?,
            caller: IdentityId::Person(self.root),
            route: Route::Api,
            holder,
            resource: resource.clone(),
            relation: Relation::new("reader")?,
            pass_on: PassOn::to(
                BTreeSet::from([read()?]),
                BTreeSet::from([RecipientKind::Agent, RecipientKind::Person]),
            )?,
            window: Window::new(T0, ends)?,
        };
        let directory = self.directory.projection()?;
        Ok(self.grants.issue_root(directory, &request, self.now)?)
    }

    /// `from` passes read of `source` on `resource` on to `to`, which may pass
    /// it on to agents, ending at `ends`.
    pub fn delegate(
        &mut self,
        source: GrantId,
        (from, to): (IdentityId, IdentityId),
        resource: &Resource,
        ends: Option<u64>,
    ) -> Result<Result<Recorded, GrantError>, Box<dyn Error>> {
        let responsible = match to {
            IdentityId::Person(person) => person,
            IdentityId::Agent(_) => self
                .directory
                .record(to)?
                .and_then(|record| record.responsible())
                .ok_or("the agent answers to no person")?,
        };
        let request = DelegateRequest {
            operation: OperationId::generate()?,
            caller: from,
            route: Route::Api,
            source,
            recipient: to,
            responsible,
            resource: resource.clone(),
            relation: Relation::new("reader")?,
            pass_on: PassOn::to(
                BTreeSet::from([read()?]),
                BTreeSet::from([RecipientKind::Agent]),
            )?,
            window: Window::new(T0, ends)?,
        };
        let directory = self.directory.projection()?;
        Ok(self.grants.delegate(directory, &request, self.now))
    }

    /// `by` revokes `grant`.
    pub fn revoke(&mut self, grant: GrantId, by: IdentityId) -> Result<Recorded, Box<dyn Error>> {
        let request = RevokeRequest {
            operation: OperationId::generate()?,
            caller: by,
            route: Route::Api,
            grant,
            reason: "withdrawn by the test".to_owned(),
        };
        Ok(self.grants.revoke(&request, self.now)?)
    }

    /// The administrator suspends `identity`.
    pub fn suspend(&mut self, identity: IdentityId) -> Result<(), Box<dyn Error>> {
        self.directory.transition(
            administrator()?,
            OperationId::generate()?,
            identity,
            Transition::Suspend,
            "suspended by the test",
            self.now,
        )?;
        Ok(())
    }

    /// The permission decision as the service makes it at the moment of the
    /// action: over this world's directory, answered by `evaluator`, a
    /// permit recording the use.
    pub fn check_with(
        &mut self,
        request: &ExerciseRequest,
        evaluator: &dyn Decides,
        at: u64,
    ) -> Result<Result<Permit, GrantError>, Box<dyn Error>> {
        let directory = self.directory.projection()?;
        Ok(self.grants.check_with(directory, request, evaluator, at))
    }

    /// Append a grant event issuing a grant of `parts` straight to the grant
    /// log, signed by the service key but judged by nobody, and reopen the
    /// grants over it: a corrupt or foreign committed event.
    pub fn append_raw(&mut self, parts: GrantParts) -> Result<(), Box<dyn Error>> {
        let key = Ed25519Identity::load(&self.dir.path().join("service.key"))?;
        let event = GrantEvent::new(
            parts.operation,
            parts.issuer,
            self.now,
            GrantChange::Issue(Box::new(lys_identity::grants::Grant::new(parts)?)),
        )?;
        {
            let (mut ledger, _) =
                GrantLedger::open(reopen(self.dir.path().join("grants")), &key, SNAPSHOT_EVERY)?;
            ledger.append(&sign_grant_event(event, &key)?)?;
        }
        self.grants = Grants::open(
            reopen(self.dir.path().join("grants")),
            key,
            MemoryRelationships::default(),
            model()?,
            self.root,
        )?;
        Ok(())
    }

    /// The parts of a grant of read on `resource` to `holder`, answered for
    /// by `responsible`, issued by `issuer`, standing on `source`, ending at
    /// `ends`, for [`World::append_raw`].
    pub fn parts(
        (issuer, holder, responsible): (IdentityId, IdentityId, PersonId),
        resource: &Resource,
        source: Source,
        ends: Option<u64>,
    ) -> Result<GrantParts, Box<dyn Error>> {
        Ok(GrantParts {
            id: GrantId::generate()?,
            issuer,
            holder,
            responsible,
            resource: resource.clone(),
            relation: Relation::new("reader")?,
            actions: BTreeSet::from([read()?]),
            pass_on: PassOn::to(
                BTreeSet::from([read()?]),
                BTreeSet::from([RecipientKind::Agent]),
            )?,
            source,
            window: Window::new(T0, ends)?,
            model_version: 1,
            operation: OperationId::generate()?,
        })
    }

    /// The projector writing through `client` at most `cap` updates at once,
    /// keeping its position in this world's directory, so a projector opened
    /// again resumes where the last stood.
    pub fn projector(&self, client: &Arc<Counting>, cap: u32) -> Result<Projector, Box<dyn Error>> {
        let client: Arc<Counting> = Arc::clone(client);
        Ok(Projector::open(
            client,
            Box::new(FilePosition::new(self.dir.path().join("projection.json"))),
            cap,
            self.root,
        )?)
    }

    /// The grant book.
    pub fn book(&self) -> &GrantBook {
        self.grants.book()
    }
}

/// A recorded event's log position: its index plus one.
pub fn position(recorded: &Recorded) -> u64 {
    recorded.index + 1
}

/// The fixtures' project `id`.
pub fn project(id: &str) -> Result<Resource, GrantError> {
    Resource::new("project", id)
}

/// The support's own check: the wrapper forwards and keeps every kind of
/// call, fails and pauses the writes a test names, and strips a check's
/// context when told; the world, the projector and the direct reads,
/// checks and writes work together against a disposable `SpiceDB`.
#[test]
fn support_forwards_and_keeps_every_call() -> TestResult {
    let spicedb = server::SpiceDb::start(None)?;
    let client = spicedb.client()?;
    assert_eq!(client.read_schema()?, SchemaRead::NoSchema);
    let mut world = World::new()?;
    let started = lys_identity_server::spicedb::schema::ensure(client.as_ref())?;
    assert_eq!(
        started,
        lys_identity_server::spicedb::schema::Started::Written
    );
    let project_x = project("x")?;
    let person = world.person("P")?;
    let agent = world.agent(person, "A")?;
    let root = world.root_grant(person, &project_x, None)?;
    let lent = world.delegate(
        root.event.grant(),
        (IdentityId::Person(person), IdentityId::Agent(agent)),
        &project_x,
        None,
    )??;
    let raw = World::parts(
        (IdentityId::Agent(agent), IdentityId::Agent(agent), person),
        &project_x,
        Source::Grant(lent.event.grant()),
        None,
    )?;
    world.append_raw(raw)?;
    let mut projector = world.projector(&client, 1000)?;
    projector.catch_up(world.book(), position(&lent))?;
    let (told, reached) = std::sync::mpsc::channel();
    let (release, held) = std::sync::mpsc::channel();
    *client.pause.lock().unwrap_or_else(PoisonError::into_inner) = Some(Pause {
        after: writes(&client).len() + 1,
        reached: told,
        resume: held,
    });
    let resumer = std::thread::spawn(move || {
        reached.recv().ok();
        release.send(()).ok();
    });
    projector.catch_up(world.book(), world.grants.revision())?;
    assert_eq!(projector.reached().token, last_written(&client));
    resumer
        .join()
        .map_err(|panic| format!("the resumer stopped: {panic:?}"))?;
    let kept = stored(client.as_ref())?;
    assert_eq!(
        belonging(&kept, &[root.event.grant(), lent.event.grant()]),
        6
    );
    assert_eq!(kept.len(), 11);
    let holder = IdentityId::Agent(agent);
    assert_eq!(
        check_directly(client.as_ref(), holder, &project_x, &read()?, world.now)?,
        Permissionship::HasPermission
    );
    client.strip_context.store(true, Ordering::SeqCst);
    assert_eq!(
        check_directly(client.as_ref(), holder, &project_x, &read()?, world.now)?,
        Permissionship::ConditionalPermission
    );
    client.strip_context.store(false, Ordering::SeqCst);
    *client
        .fail_write
        .lock()
        .unwrap_or_else(PoisonError::into_inner) = Some(writes(&client).len() + 1);
    let marker = ObjectRef {
        kind: "root_authority".to_owned(),
        id: "directory".to_owned(),
    };
    let everyone = ObjectRef {
        kind: "person".to_owned(),
        id: "*".to_owned(),
    };
    assert!(write_one(client.as_ref(), (&marker, "anyone", &everyone)).is_err());
    let asked = checks(&client);
    assert_eq!(asked.len(), 2);
    assert_eq!(
        consistency(asked[0].0.consistency.as_ref()),
        "fully_consistent"
    );
    assert!(asked.iter().all(|(_, answer)| answer.is_some()));
    assert!(lookups(&client).is_empty());
    let (schema_reads, schema_writes, reads) = others(&client);
    assert_eq!((schema_reads, schema_writes), (3, 1));
    assert_eq!(reads, 3);
    *client
        .fail_write
        .lock()
        .unwrap_or_else(PoisonError::into_inner) = None;
    world.revoke(lent.event.grant(), IdentityId::Person(person))?;
    projector.catch_up(world.book(), world.grants.revision())?;
    let revoked = stored(client.as_ref())?;
    assert_eq!(belonging(&revoked, &[lent.event.grant()]), 0);
    assert_eq!(belonging(&revoked, &[root.event.grant()]), 3);
    let asking: Arc<Counting> = Arc::clone(&client);
    let evaluator = Evaluator::new(asking, Arc::new(Fixed(world.now)));
    let reached = projector.reached().clone();
    let request = ExerciseRequest {
        caller: IdentityId::Person(person),
        route: Route::Api,
        resource: project_x,
        action: read()?,
    };
    let at = world.now;
    let permitted = world.check_with(&request, &Fresh::new(&evaluator, &reached), at)?;
    assert_eq!(permitted?.grant, root.event.grant());
    world.suspend(IdentityId::Person(person))?;
    let refused = world.check_with(&request, &Fresh::new(&evaluator, &reached), at)?;
    assert!(
        matches!(refused, Err(GrantError::IdentityNotActive { .. })),
        "{refused:?}"
    );
    Ok(())
}
