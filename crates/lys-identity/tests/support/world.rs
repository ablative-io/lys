//! A directory of four people and two agents on a temporary log, every one
//! active, and the grants judged against a three-relation model.

use std::collections::BTreeSet;
use std::error::Error;
use std::os::unix::fs::PermissionsExt;
use std::sync::Arc;

use lys_core::Ed25519Identity;
use lys_identity::grants::{
    Action, DelegateRequest, ExerciseRequest, GrantError, GrantId, Grants, Model, PassOn, Permit,
    RecipientKind, Recorded, Relation, Resource, RootRequest, Route, Window,
};
use lys_identity::{
    Actor, AgentId, AuthMethod, Directory, IdentityId, LoginBinding, OperationId, PersonId,
    Profile, Provenance, Transition,
};
use lys_log_store::FileLeafStore;

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

/// The directory, the grants, and who is in them.
pub struct World {
    /// The directory of people and agents.
    pub directory: Directory<FileLeafStore>,
    /// The grants.
    pub grants: Grants,
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
}

/// The directory over a fresh log in `dir`, which lives as long as the directory does.
fn open_directory(dir: tempfile::TempDir) -> Result<Directory<FileLeafStore>, Box<dyn Error>> {
    let key = Ed25519Identity::load(&dir.path().join("service.key"))?;
    let dir = Arc::new(dir);
    let reopen = Box::new(move || FileLeafStore::open(&dir.path().join("log")));
    Ok(Directory::open(reopen, key)?)
}

impl World {
    /// A fresh world at [`T0`].
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let dir = tempfile::TempDir::new()?;
        FileLeafStore::create(&dir.path().join("log"), "example.test/lys/directory")?;
        let key = dir.path().join("service.key");
        std::fs::write(&key, [7; 32])?;
        std::fs::set_permissions(&key, std::fs::Permissions::from_mode(0o600))?;
        let mut directory = open_directory(dir)?;
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
        Ok(Self {
            directory,
            grants: Grants::new(model()?, admin),
            admin,
            dana,
            tom,
            lee,
            tom_agent,
            dana_agent,
            now: T0 + 10,
        })
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
            IdentityId::Agent(_) => self
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
        self.grants.check(directory, &request, self.now)
    }

    /// How many grant events are recorded.
    pub fn events(&self) -> usize {
        self.grants.events().len()
    }
}
