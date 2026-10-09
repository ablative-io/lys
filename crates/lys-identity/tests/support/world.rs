//! A directory of four people and two agents on a temporary log, every one
//! active, and the grants on their own log, judged against a three-relation
//! model, with a permission engine the test chooses.

use std::collections::BTreeSet;
use std::error::Error;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::sync::Arc;

use lys_core::Ed25519Identity;
use lys_identity::grants::{
    Action, DelegateRequest, ExerciseRequest, GrantError, GrantId, Grants, MemoryRelationships,
    Model, PassOn, Permit, RecipientKind, Recorded, Relation, RelationshipStore, Resource,
    RootRequest, Route, Window,
};
use lys_identity::log::Reopen;
use lys_identity::{
    Actor, AgentId, AuthMethod, Directory, IdentityId, LoginBinding, OperationId, PersonId,
    Profile, Provenance, Transition,
};
use lys_log_store::{FileLeafStore, LeafStore};

/// The first second every test starts at.
pub const T0: u64 = 1_800_000_000;

/// The actions `names` spell.
pub fn actions(names: &[&str]) -> Result<BTreeSet<Action>, GrantError> {
    names.iter().map(|name| Action::new(name)).collect()
}

/// Pass-on of `names` to `kinds`.
pub fn pass(names: &[&str], kinds: &[RecipientKind]) -> Result<PassOn, GrantError> {
    PassOn::to(actions(names)?, kinds.iter().copied().collect())
}

/// The project every grant in these tests is on, unless a test names another.
pub fn alpha() -> Result<Resource, GrantError> {
    Resource::new("project", "alpha")
}

/// The model: `kite` carries read, write and delete; `heron` read and write;
/// `tern` read alone. The names say nothing about the sets.
pub fn model() -> Result<Model, GrantError> {
    Model::new(
        1,
        [
            (
                Relation::new("kite")?,
                actions(&["delete", "read", "write"])?,
            ),
            (Relation::new("heron")?, actions(&["read", "write"])?),
            (Relation::new("tern")?, actions(&["read"])?),
        ],
    )
}

fn administrator() -> Result<Actor, Box<dyn Error>> {
    Ok(Actor::new(
        LoginBinding::new("https://issuer.test", "administrator")?,
        Provenance::new(AuthMethod::Oidc, T0),
    ))
}

/// Opens a grant log's store in the directory it is given.
pub type Opener<S> = Box<dyn Fn(&Path) -> Reopen<S>>;

/// The directory, the grants, and who is in them.
pub struct World<S: LeafStore = FileLeafStore, R: RelationshipStore = MemoryRelationships> {
    /// The directory of people and agents.
    pub directory: Directory<FileLeafStore>,
    /// The grants.
    pub grants: Grants<S, R>,
    /// The root authority.
    pub admin: PersonId,
    /// A person who holds root grants.
    pub dana: PersonId,
    /// A person Dana lends to.
    pub tom: PersonId,
    /// A third person.
    pub lee: PersonId,
    /// Tom's agent.
    pub tom_agent: AgentId,
    /// Dana's agent.
    pub dana_agent: AgentId,
    /// The time every request is made at.
    pub now: u64,
    /// Holds the logs and the key: `log`, `grants` and `service.key`.
    pub dir: Arc<tempfile::TempDir>,
    opener: Opener<S>,
}

/// The directory over its log in `dir`.
fn open_directory(
    dir: &Arc<tempfile::TempDir>,
) -> Result<Directory<FileLeafStore>, Box<dyn Error>> {
    let key = Ed25519Identity::load(&dir.path().join("service.key"))?;
    let held = Arc::clone(dir);
    let reopen = Box::new(move || FileLeafStore::open(&held.path().join("log")));
    Ok(Directory::open(reopen, key)?)
}

impl World {
    /// A fresh world at [`T0`], its grants on a file log and an in-process engine.
    pub fn new() -> Result<Self, Box<dyn Error>> {
        World::with(
            Box::new(|path: &Path| -> Reopen<FileLeafStore> {
                let path = path.to_owned();
                Box::new(move || FileLeafStore::open(&path))
            }),
            MemoryRelationships::default(),
        )
    }
}

impl<S: LeafStore, R: RelationshipStore> World<S, R> {
    /// A fresh world at [`T0`], its grant log opened by `opener` and its
    /// permission relationships held by `relationships`.
    pub fn with(opener: Opener<S>, relationships: R) -> Result<Self, Box<dyn Error>> {
        let dir = Arc::new(tempfile::TempDir::new()?);
        FileLeafStore::create(&dir.path().join("log"), "example.test/lys/directory")?;
        FileLeafStore::create(&dir.path().join("grants"), "example.test/lys/grants")?;
        let key = dir.path().join("service.key");
        std::fs::write(&key, [7; 32])?;
        std::fs::set_permissions(&key, std::fs::Permissions::from_mode(0o600))?;
        let mut directory = open_directory(&dir)?;
        let mut person = |name: &str| -> Result<PersonId, Box<dyn Error>> {
            let (id, _) = directory.register_person(
                administrator()?,
                OperationId::generate()?,
                Profile::new(name)?,
                T0,
            )?;
            Ok(id)
        };
        let (admin, dana, tom, lee) = (
            person("Admin")?,
            person("Dana")?,
            person("Tom")?,
            person("Lee")?,
        );
        let mut agent = |responsible: PersonId, name: &str| -> Result<AgentId, Box<dyn Error>> {
            let (id, _) = directory.register_agent(
                administrator()?,
                OperationId::generate()?,
                responsible,
                Profile::new(name)?,
                T0,
            )?;
            Ok(id)
        };
        let (tom_agent, dana_agent) = (agent(tom, "Tom's agent")?, agent(dana, "Dana's agent")?);
        let everyone = [admin, dana, tom, lee].map(IdentityId::Person);
        for identity in everyone
            .into_iter()
            .chain([tom_agent, dana_agent].map(IdentityId::Agent))
        {
            directory.transition(
                administrator()?,
                OperationId::generate()?,
                identity,
                Transition::Activate,
                "",
                T0,
            )?;
        }
        let grants = Grants::open(
            opener(&dir.path().join("grants")),
            Ed25519Identity::load(&key)?,
            relationships,
            model()?,
            admin,
        )?;
        Ok(Self {
            directory,
            grants,
            admin,
            dana,
            tom,
            lee,
            tom_agent,
            dana_agent,
            now: T0 + 10,
            dir,
            opener,
        })
    }

    /// Restart the grants over the same log, with `relationships` as the engine.
    pub fn reopen(&mut self, relationships: R) -> Result<(), Box<dyn Error>> {
        self.grants = Grants::open(
            (self.opener)(&self.dir.path().join("grants")),
            Ed25519Identity::load(&self.dir.path().join("service.key"))?,
            relationships,
            model()?,
            self.admin,
        )?;
        Ok(())
    }

    /// Issue `holder` a root grant on [`alpha`], as the root authority.
    pub fn root(
        &mut self,
        holder: PersonId,
        relation: &str,
        pass_on: PassOn,
        ends_at: Option<u64>,
    ) -> Result<GrantId, Box<dyn Error>> {
        let request = RootRequest {
            operation: OperationId::generate()?,
            caller: IdentityId::Person(self.admin),
            route: Route::Api,
            holder,
            resource: alpha()?,
            relation: Relation::new(relation)?,
            pass_on,
            window: Window::new(T0, ends_at)?,
        };
        let directory = self.directory.projection()?;
        Ok(self
            .grants
            .issue_root(directory, &request, self.now)?
            .event
            .grant())
    }

    /// A request from `caller` to pass `relation` of `source` on to
    /// `recipient` on [`alpha`], naming the recipient's recorded responsible person.
    pub fn request(
        &mut self,
        caller: IdentityId,
        source: GrantId,
        recipient: IdentityId,
        relation: &str,
        pass_on: PassOn,
        ends_at: Option<u64>,
    ) -> Result<DelegateRequest, Box<dyn Error>> {
        let responsible = match recipient {
            IdentityId::Person(person) => person,
            IdentityId::Agent(_)
            | IdentityId::ServiceAccount(_)
            | IdentityId::Connector(_)
            | IdentityId::Machine(_) => self
                .directory
                .record(recipient)?
                .and_then(|record| record.responsible())
                .ok_or("the agent has no responsible person")?,
        };
        Ok(DelegateRequest {
            operation: OperationId::generate()?,
            caller,
            route: Route::Api,
            source,
            recipient,
            responsible,
            resource: alpha()?,
            relation: Relation::new(relation)?,
            pass_on,
            window: Window::new(T0, ends_at)?,
        })
    }

    /// Put `request` to the grants.
    pub fn delegate(&mut self, request: &DelegateRequest) -> Result<Recorded, GrantError> {
        let directory = self.directory.projection()?;
        self.grants.delegate(directory, request, self.now)
    }

    /// Whether `caller` may exercise `action` on [`alpha`] through `route`.
    pub fn exercise(
        &mut self,
        caller: IdentityId,
        action: &str,
        route: Route,
    ) -> Result<Permit, GrantError> {
        let request = ExerciseRequest {
            caller,
            route,
            resource: alpha()?,
            action: Action::new(action)?,
        };
        let directory = self.directory.projection()?;
        self.grants.check(directory, &request, self.now, None)
    }

    /// How many grant events are recorded.
    pub fn events(&self) -> u64 {
        self.grants.revision()
    }
}

impl<S: LeafStore, R: RelationshipStore> std::fmt::Debug for World<S, R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("World")
            .field("admin", &self.admin)
            .field("dana", &self.dana)
            .field("tom", &self.tom)
            .field("lee", &self.lee)
            .field("tom_agent", &self.tom_agent)
            .field("dana_agent", &self.dana_agent)
            .field("now", &self.now)
            .field("dir", &self.dir.path())
            .finish_non_exhaustive()
    }
}
