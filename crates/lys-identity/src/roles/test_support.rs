//! The world the role tests run in.
//!
//! Built only for tests, and for other crates' tests through the
//! `test-support` feature. Every identity is a disposable test identity on
//! temporary logs. D1 is the directory's configured administrator and the
//! grants' root authority, holding no grant on P1. H1 is the person agents
//! A1 to A4 answer to, O2 the person agent B1 answers to, O1 the owner of P1
//! and X1, displayed as "Owner of P1", holds only a use-only read grant on
//! P1. H1 and O1 each hold reader, tester and pusher on P1 with authority to
//! pass each on to agents. Role builder is defined in P1 by O1, its version
//! 1 carrying the templates read-P1 and test-P1.

use std::collections::BTreeSet;
use std::error::Error;
use std::path::Path;

use lys_core::Ed25519Identity;
use lys_log_store::FileLeafStore;

use super::check::{Asked, Check, RecordCheck, RoleCheck};
use super::edit::{EditTemplates, MakeRole};
use super::error::RoleError;
use super::events::RoleEvent;
use super::holding::{Acting, Assign, Roles};
use super::move_holder::ConfirmMove;
use super::ownership::{GiveOwner, give_owner};
use super::types::{Capacity, Holding, RoleId, Template, TemplateParts, Timing};
use crate::grants::test_support::FailingRelationships;
use crate::grants::types::{OWNER_ACTION, PROJECT_KIND};
use crate::grants::{
    Action, Grant, GrantError, GrantId, Grants, Model, PassOn, RecipientKind, Recorded, Relation,
    Resource, RootRequest, Route, Window,
};
use crate::{
    Actor, AgentId, AuthMethod, Directory, IdentityId, LoginBinding, OperationId, PersonId,
    Profile, Provenance, Transition,
};

/// The first second every role test starts at.
pub const T0: u64 = 1_800_000_000;

/// One day, in seconds.
pub const DAY: u64 = 86_400;

/// The end date E the role tests give a holding: 30 days after [`T0`].
pub const E: u64 = T0 + 30 * DAY;

/// A fallible fixture step.
pub type Fallible<T> = Result<T, Box<dyn Error>>;

fn administrator() -> Result<Actor, RoleError> {
    Ok(Actor::new(
        LoginBinding::new("https://issuer.test", "administrator")?,
        Provenance::new(AuthMethod::Oidc, T0),
    ))
}

fn actions(names: &[&str]) -> Result<BTreeSet<Action>, GrantError> {
    names.iter().map(|name| Action::new(name)).collect()
}

/// The model: `reader` carries read, `tester` test, `pusher` push and the
/// project's `owner` relation own.
pub fn model() -> Result<Model, GrantError> {
    Model::new(
        1,
        [
            (Relation::new("reader")?, actions(&["read"])?),
            (Relation::new("tester")?, actions(&["test"])?),
            (Relation::new("pusher")?, actions(&["push"])?),
            (Relation::owner(), actions(&[OWNER_ACTION])?),
        ],
    )
}

/// The role tests' directory, grants, roles and who is in them.
pub struct RoleWorld {
    /// The directory of people and agents.
    pub directory: Directory<FileLeafStore>,
    /// The grants.
    pub grants: Grants<FileLeafStore, FailingRelationships>,
    /// The roles.
    pub roles: Roles,
    /// The permission engine, which a test may take down.
    pub engine: FailingRelationships,
    /// The configured administrator and root authority.
    pub d1: PersonId,
    /// The person agents A1 to A4 answer to.
    pub h1: PersonId,
    /// The owner of P1.
    pub o1: PersonId,
    /// The person agent B1 answers to.
    pub o2: PersonId,
    /// A person holding neither capacity, displayed as "Owner of P1".
    pub x1: PersonId,
    /// H1's first agent.
    pub a1: AgentId,
    /// H1's second agent.
    pub a2: AgentId,
    /// H1's third agent.
    pub a3: AgentId,
    /// H1's fourth agent.
    pub a4: AgentId,
    /// O2's agent.
    pub b1: AgentId,
    /// The project every role here is defined in.
    pub p1: Resource,
    /// Role builder, defined in P1 by O1.
    pub builder: RoleId,
    /// The controlled clock.
    pub now: u64,
}

impl RoleWorld {
    /// A fresh world on logs in `dir`, at [`T0`] plus ten seconds.
    pub fn new(dir: &Path) -> Fallible<Self> {
        FileLeafStore::create(&dir.join("log"), "example.test/lys/directory")?;
        FileLeafStore::create(&dir.join("grants"), "example.test/lys/grants")?;
        let key = dir.join("service.key");
        let directory_key = Ed25519Identity::load_or_generate(&key)?;
        let log = dir.join("log");
        let mut directory = Directory::open(
            Box::new(move || FileLeafStore::open(&log)),
            directory_key,
        )?;
        let mut person = |name: &str| -> Fallible<PersonId> {
            let (id, _) = directory.register_person(
                administrator()?,
                OperationId::generate()?,
                Profile::new(name)?,
                T0,
            )?;
            Ok(id)
        };
        let (d1, h1, o1, o2, x1) = (
            person("D1")?,
            person("H1")?,
            person("O1")?,
            person("O2")?,
            person("Owner of P1")?,
        );
        let mut agent = |responsible: PersonId, name: &str| -> Fallible<AgentId> {
            let (id, _) = directory.register_agent(
                administrator()?,
                OperationId::generate()?,
                responsible,
                Profile::new(name)?,
                T0,
            )?;
            Ok(id)
        };
        let (a1, a2, a3, a4, b1) = (
            agent(h1, "A1")?,
            agent(h1, "A2")?,
            agent(h1, "A3")?,
            agent(h1, "A4")?,
            agent(o2, "B1")?,
        );
        let people = [d1, h1, o1, o2, x1].map(IdentityId::Person);
        let agents = [a1, a2, a3, a4, b1].map(IdentityId::Agent);
        for identity in people.into_iter().chain(agents) {
            directory.transition(
                administrator()?,
                OperationId::generate()?,
                identity,
                Transition::Activate,
                "",
                T0,
            )?;
        }
        let engine = FailingRelationships::default();
        let grants_log = dir.join("grants");
        let grants = Grants::open(
            Box::new(move || FileLeafStore::open(&grants_log)),
            Ed25519Identity::load(&key)?,
            engine.clone(),
            model()?,
            d1,
        )?;
        let mut world = Self {
            directory,
            grants,
            roles: Roles::new(Ed25519Identity::load(&key)?),
            engine,
            d1,
            h1,
            o1,
            o2,
            x1,
            a1,
            a2,
            a3,
            a4,
            b1,
            p1: Resource::new(PROJECT_KIND, "p1")?,
            builder: RoleId::from_bytes([0; 16]),
            now: T0 + 10,
        };
        for holder in [h1, o1] {
            for (relation, action) in [("reader", "read"), ("tester", "test"), ("pusher", "push")]
            {
                let pass = PassOn::to(actions(&[action])?, [RecipientKind::Agent].into())?;
                world.root(holder, relation, pass)?;
            }
        }
        world.root(x1, "reader", PassOn::UseOnly)?;
        world.give_owner(IdentityId::Person(d1), IdentityId::Person(o1))?;
        let made = world.make_role("Builder", &[("reader", "read"), ("tester", "test")])?;
        let super::events::RoleChange::VersionMade { role, .. } = made.change() else {
            return Err("making builder did not make a version".into());
        };
        world.builder = *role;
        Ok(world)
    }

    /// The roles and what an act is judged and made with, at the clock's now.
    pub fn acting(
        &mut self,
    ) -> Result<(&mut Roles, Acting<'_, FileLeafStore, FailingRelationships>), RoleError> {
        let directory = self.directory.projection()?;
        Ok((
            &mut self.roles,
            Acting {
                grants: &mut self.grants,
                directory,
                check: &RecordCheck,
                at: self.now,
            },
        ))
    }

    /// Issue `holder` a root grant of `relation` on P1 with no end, as D1.
    pub fn root(
        &mut self,
        holder: PersonId,
        relation: &str,
        pass_on: PassOn,
    ) -> Result<GrantId, RoleError> {
        let request = RootRequest {
            operation: OperationId::generate()?,
            caller: IdentityId::Person(self.d1),
            route: Route::Api,
            holder,
            resource: self.p1.clone(),
            relation: Relation::new(relation)?,
            pass_on,
            window: Window::new(T0, None)?,
        };
        let directory = self.directory.projection()?;
        Ok(self
            .grants
            .issue_root(directory, &request, self.now)?
            .event
            .grant())
    }

    /// The template of `relation` carrying `action` on P1: use-only, a
    /// window from [`T0`] with no end of its own, responsible H1.
    pub fn template(&self, relation: &str, action: &str) -> Result<Template, RoleError> {
        Template::new(TemplateParts {
            relation: Relation::new(relation)?,
            resource: self.p1.clone(),
            actions: actions(&[action])?,
            pass_on: PassOn::UseOnly,
            window: Window::new(T0, None)?,
            responsible: self.h1,
        })
    }

    fn templates(&self, pairs: &[(&str, &str)]) -> Result<Vec<Template>, RoleError> {
        pairs
            .iter()
            .map(|(relation, action)| self.template(relation, action))
            .collect()
    }

    /// O1 makes a role in P1 titled `title` with the templates `pairs` name.
    pub fn make_role(
        &mut self,
        title: &str,
        pairs: &[(&str, &str)],
    ) -> Result<RoleEvent, RoleError> {
        let request = MakeRole {
            operation: OperationId::generate()?,
            actor: IdentityId::Person(self.o1),
            project: self.p1.clone(),
            title: title.to_owned(),
            templates: self.templates(pairs)?,
        };
        let (roles, mut acting) = self.acting()?;
        roles.make_role(&mut acting, &request)
    }

    /// `actor` changes builder's templates to those `pairs` name.
    pub fn edit(
        &mut self,
        actor: PersonId,
        pairs: &[(&str, &str)],
    ) -> Result<RoleEvent, RoleError> {
        let request = EditTemplates {
            operation: OperationId::generate()?,
            actor: IdentityId::Person(actor),
            role: self.builder,
            templates: self.templates(pairs)?,
        };
        let (roles, mut acting) = self.acting()?;
        roles.edit_templates(&mut acting, &request)
    }

    /// O1 makes builder's version 2: read-P1 and push-P1.
    pub fn make_version_2(&mut self) -> Result<RoleEvent, RoleError> {
        self.edit(self.o1, &[("reader", "read"), ("pusher", "push")])
    }

    /// `giver` gives `holder` the owner relation on P1.
    pub fn give_owner(
        &mut self,
        giver: IdentityId,
        holder: IdentityId,
    ) -> Result<Recorded, RoleError> {
        let request = GiveOwner {
            operation: OperationId::generate()?,
            giver,
            holder,
            project: self.p1.clone(),
        };
        let (_, mut acting) = self.acting()?;
        give_owner(&mut acting, &request)
    }

    /// `actor` assigns builder to `agent`, ending at `ends_at`.
    pub fn assign(
        &mut self,
        actor: PersonId,
        agent: AgentId,
        ends_at: Option<u64>,
    ) -> Result<RoleEvent, RoleError> {
        let request = Assign {
            operation: OperationId::generate()?,
            actor: IdentityId::Person(actor),
            role: self.builder,
            agent,
            ends_at,
        };
        let (roles, mut acting) = self.acting()?;
        roles.assign(&mut acting, &request)
    }

    /// `agent`'s holding of builder.
    pub fn holding(&self, agent: AgentId) -> Result<Holding, RoleError> {
        self.roles
            .book()
            .held_by(agent, self.builder)
            .next()
            .cloned()
            .ok_or_else(|| RoleError::HoldingUnknown {
                holding: format!("builder held by {agent}"),
            })
    }

    /// The grant `id`.
    pub fn grant(&self, id: GrantId) -> Result<&Grant, RoleError> {
        self.grants
            .book()
            .grant(id)
            .ok_or_else(|| RoleError::Grant(GrantError::GrantUnknown {
                grant: id.to_string(),
            }))
    }

    /// `actor` previews and confirms the move of `agent`'s holding to `to`.
    pub fn move_to(
        &mut self,
        actor: PersonId,
        agent: AgentId,
        to: u64,
        timing: Option<Timing>,
    ) -> Result<RoleEvent, RoleError> {
        let holding = self.holding(agent)?.id;
        let directory = self.directory.projection()?;
        let preview = self.roles.preview(directory, holding, to)?;
        let request = ConfirmMove {
            operation: OperationId::generate()?,
            actor: IdentityId::Person(actor),
            preview,
            timing,
        };
        let (roles, mut acting) = self.acting()?;
        roles.confirm_move(&mut acting, &request)
    }

    /// The capacity the seam answers `actor` for builder, and for `holder`'s holding when named.
    pub fn capacity(
        &mut self,
        actor: PersonId,
        holder: Option<AgentId>,
        check: Check,
    ) -> Result<Option<Capacity>, RoleError> {
        let project = self.p1.clone();
        let (_, acting) = self.acting()?;
        RecordCheck.capacity(
            &acting.facts(),
            &Asked {
                actor: IdentityId::Person(actor),
                check,
                project: &project,
                holder,
                at: acting.at,
            },
        )
    }

    /// How many committed role events are of change kind `kind`.
    pub fn events_of_kind(&self, kind: u64) -> usize {
        self.roles
            .events()
            .iter()
            .filter(|signed| signed.event().change().kind() == kind)
            .count()
    }
}
