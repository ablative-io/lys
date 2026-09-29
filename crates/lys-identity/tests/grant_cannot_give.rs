#![cfg(test)]
//! DIRECTORY-024 R1: what a person cannot give from the delegation form, each
//! item with its one reason (conformance row 2.4, the `CANNOT_GIVE_*` cases).
//!
//! The grants are written straight into a grant book under fixed ids, so the
//! order the list promises, by the bytes of each grant id, is checkable: `Gn`
//! is the id whose sixteen bytes are all `n`.

use std::collections::BTreeSet;
use std::error::Error;
use std::os::unix::fs::PermissionsExt;
use std::sync::Arc;

use lys_core::Ed25519Identity;
use lys_identity::grants::admission::judge_delegation;
use lys_identity::grants::cannot_give::cannot_give;
use lys_identity::grants::{
    Action, CannotGiveItem, CannotGiveList, CannotGiveReason, CannotGiveRequest, CannotGiveSubject,
    DelegateRequest, Grant, GrantBook, GrantChange, GrantEvent, GrantId, GrantParts, Model, PassOn,
    RecipientKind, Relation, Resource, Route, SERVICE_ACCOUNT, Source, Window,
};
use lys_identity::projection::Projection;
use lys_identity::{
    Actor, AgentId, AuthMethod, Directory, IdentityId, LoginBinding, OperationId, PersonId,
    Profile, Provenance, Transition,
};
use lys_log_store::FileLeafStore;

type TestResult = Result<(), Box<dyn Error>>;

const T0: u64 = 1_800_000_000;
/// The controlled clock every list is asked at.
const AT: u64 = T0 + 10;

use CannotGiveReason::{
    AboveWhatYouHold, AgentsOnly, LentToYou, PeopleOnly, SignInIdentity, UseOnly,
};

fn g(n: u8) -> GrantId {
    GrantId::from_bytes([n; 16])
}

fn actions(names: &[&str]) -> Result<BTreeSet<Action>, Box<dyn Error>> {
    Ok(names
        .iter()
        .map(|name| Action::new(name))
        .collect::<Result<_, _>>()?)
}

fn to(names: &[&str], kinds: &[RecipientKind]) -> Result<PassOn, Box<dyn Error>> {
    Ok(PassOn::to(
        actions(names)?,
        kinds.iter().copied().collect(),
    )?)
}

const BOTH: [RecipientKind; 2] = [RecipientKind::Person, RecipientKind::Agent];

/// The fixture model: four relations whose names say nothing of their actions.
fn model(swapped: bool) -> Result<Model, Box<dyn Error>> {
    let (alder, damson): (&[&str], &[&str]) = if swapped {
        (&["comment", "edit", "grant", "view"], &["view"])
    } else {
        (&["view"], &["comment", "edit", "grant", "view"])
    };
    Ok(Model::new(
        1,
        [
            (Relation::new("alder")?, actions(alder)?),
            (Relation::new("birch")?, actions(&["comment", "view"])?),
            (Relation::new("cedar")?, actions(&["edit", "view"])?),
            (Relation::new("damson")?, actions(damson)?),
        ],
    )?)
}

fn administrator() -> Result<Actor, Box<dyn Error>> {
    Ok(Actor::new(
        LoginBinding::new("https://issuer.test", "administrator")?,
        Provenance::new(AuthMethod::Oidc, T0),
    ))
}

/// The directory of the fixture: the root authority, people P1, P2 and P3,
/// and P1's agent A1, every one active; P1 has one linked sign-in identity.
/// The directory's store reopener holds the temporary directory for its life.
struct People {
    directory: Directory<FileLeafStore>,
    admin: PersonId,
    p1: PersonId,
    p2: PersonId,
    p3: PersonId,
    a1: AgentId,
}

impl People {
    fn new() -> Result<Self, Box<dyn Error>> {
        let dir = Arc::new(tempfile::TempDir::new()?);
        FileLeafStore::create(&dir.path().join("log"), "example.test/lys/directory")?;
        let key = dir.path().join("service.key");
        std::fs::write(&key, [7; 32])?;
        std::fs::set_permissions(&key, std::fs::Permissions::from_mode(0o600))?;
        let held = Arc::clone(&dir);
        let mut directory = Directory::open(
            Box::new(move || FileLeafStore::open(&held.path().join("log"))),
            Ed25519Identity::load(&key)?,
        )?;
        let mut person = |name: &str| -> Result<PersonId, Box<dyn Error>> {
            let (id, _) = directory.register_person(
                administrator()?,
                OperationId::generate()?,
                Profile::new(name)?,
                T0,
            )?;
            Ok(id)
        };
        let (admin, holder, other, lender) = (
            person("Admin")?,
            person("P1")?,
            person("P2")?,
            person("P3")?,
        );
        directory.bind_login(
            administrator()?,
            OperationId::generate()?,
            holder,
            LoginBinding::new("https://issuer.test", "p1")?,
            T0,
        )?;
        let (agent, _) = directory.register_agent(
            administrator()?,
            OperationId::generate()?,
            holder,
            Profile::new("A1")?,
            T0,
        )?;
        for identity in [admin, holder, other, lender]
            .map(IdentityId::Person)
            .into_iter()
            .chain([IdentityId::Agent(agent)])
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
            admin,
            p1: holder,
            p2: other,
            p3: lender,
            a1: agent,
        })
    }

    fn projection(&mut self) -> Result<&Projection, Box<dyn Error>> {
        Ok(self.directory.projection()?)
    }
}

/// One grant to write into the book.
struct Spec {
    id: GrantId,
    issuer: PersonId,
    holder: PersonId,
    resource: Resource,
    relation: &'static str,
    pass_on: PassOn,
    source: Source,
    ends_at: Option<u64>,
}

fn project(id: &str) -> Result<Resource, Box<dyn Error>> {
    Ok(Resource::new("project", id)?)
}

/// The grants being written into one book, in the order they are applied.
struct Book {
    grants: GrantBook,
    model: Model,
    index: u64,
}

impl Book {
    fn new(model: Model) -> Self {
        Self {
            grants: GrantBook::new(),
            model,
            index: 0,
        }
    }

    fn apply(
        &mut self,
        issuer: PersonId,
        change: GrantChange,
        operation: OperationId,
    ) -> TestResult {
        let event = GrantEvent::new(operation, IdentityId::Person(issuer), T0, change)?;
        self.grants.apply(&event, self.index)?;
        self.index += 1;
        Ok(())
    }

    fn issue(&mut self, spec: Spec) -> TestResult {
        let operation = OperationId::generate()?;
        let relation = Relation::new(spec.relation)?;
        let grant = Grant::new(GrantParts {
            id: spec.id,
            issuer: IdentityId::Person(spec.issuer),
            holder: IdentityId::Person(spec.holder),
            responsible: spec.holder,
            resource: spec.resource,
            actions: self.model.actions(&relation)?.clone(),
            relation,
            pass_on: spec.pass_on,
            source: spec.source,
            window: Window::new(T0, spec.ends_at)?,
            model_version: 1,
            operation,
        })?;
        self.apply(spec.issuer, GrantChange::Issue(Box::new(grant)), operation)
    }

    fn revoke(&mut self, by: PersonId, grant: GrantId) -> TestResult {
        let change = GrantChange::Revoke {
            grant,
            reason: "fixture revocation".to_owned(),
        };
        self.apply(by, change, OperationId::generate()?)
    }
}

/// The extra grants a case adds to the fixture.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Extra {
    PeopleOnly,
    AgentsOnly,
    NotInForce,
    ServiceAccount,
    ServiceAccountGivable,
}

/// The fixture: G0 held by P3; G1, G2 and G3 held by P1, and whatever
/// `extras` add, written in `reverse` order of P1's grants when asked.
fn fixture(
    people: &People,
    swapped: bool,
    extras: &[Extra],
    reverse: bool,
) -> Result<Book, Box<dyn Error>> {
    let (admin, holder, lender) = (people.admin, people.p1, people.p3);
    let mut book = Book::new(model(swapped)?);
    book.issue(Spec {
        id: g(0),
        issuer: admin,
        holder: lender,
        resource: project("p")?,
        relation: "cedar",
        pass_on: to(&["edit", "view"], &[RecipientKind::Person])?,
        source: Source::Root,
        ends_at: None,
    })?;
    let mut mine = vec![
        Spec {
            id: g(1),
            issuer: admin,
            holder,
            resource: project("p")?,
            relation: if swapped { "damson" } else { "alder" },
            pass_on: to(&["view"], &BOTH)?,
            source: Source::Root,
            ends_at: None,
        },
        Spec {
            id: g(2),
            issuer: admin,
            holder,
            resource: project("p")?,
            relation: "birch",
            pass_on: PassOn::UseOnly,
            source: Source::Root,
            ends_at: None,
        },
        Spec {
            id: g(3),
            issuer: lender,
            holder,
            resource: project("p")?,
            relation: "cedar",
            pass_on: PassOn::UseOnly,
            source: Source::Grant(g(0)),
            ends_at: None,
        },
    ];
    if reverse {
        mine.reverse();
    }
    for spec in mine {
        book.issue(spec)?;
    }
    for extra in extras {
        match extra {
            Extra::PeopleOnly => book.issue(Spec {
                id: g(11),
                issuer: admin,
                holder,
                resource: project("u")?,
                relation: "alder",
                pass_on: to(&["view"], &[RecipientKind::Person])?,
                source: Source::Root,
                ends_at: None,
            })?,
            Extra::AgentsOnly => book.issue(Spec {
                id: g(10),
                issuer: admin,
                holder,
                resource: project("s")?,
                relation: "alder",
                pass_on: to(&["view"], &[RecipientKind::Agent])?,
                source: Source::Root,
                ends_at: None,
            })?,
            Extra::NotInForce => {
                book.issue(Spec {
                    id: g(5),
                    issuer: admin,
                    holder,
                    resource: project("r")?,
                    relation: "alder",
                    pass_on: PassOn::UseOnly,
                    source: Source::Root,
                    ends_at: None,
                })?;
                book.revoke(admin, g(5))?;
                book.issue(Spec {
                    id: g(9),
                    issuer: admin,
                    holder,
                    resource: project("p")?,
                    relation: "damson",
                    pass_on: to(&["comment", "edit", "grant", "view"], &BOTH)?,
                    source: Source::Root,
                    ends_at: Some(T0 + 5),
                })?;
                book.issue(Spec {
                    id: g(13),
                    issuer: admin,
                    holder: lender,
                    resource: project("v")?,
                    relation: "alder",
                    pass_on: to(&["view"], &[RecipientKind::Person])?,
                    source: Source::Root,
                    ends_at: None,
                })?;
                book.issue(Spec {
                    id: g(14),
                    issuer: lender,
                    holder,
                    resource: project("v")?,
                    relation: "alder",
                    pass_on: PassOn::UseOnly,
                    source: Source::Grant(g(13)),
                    ends_at: None,
                })?;
                book.revoke(admin, g(13))?;
            }
            Extra::ServiceAccount => book.issue(Spec {
                id: g(7),
                issuer: admin,
                holder,
                resource: Resource::new(SERVICE_ACCOUNT, "sa1")?,
                relation: "alder",
                pass_on: PassOn::UseOnly,
                source: Source::Root,
                ends_at: None,
            })?,
            Extra::ServiceAccountGivable => book.issue(Spec {
                id: g(8),
                issuer: admin,
                holder,
                resource: Resource::new(SERVICE_ACCOUNT, "sa2")?,
                relation: "alder",
                pass_on: to(&["view"], &BOTH)?,
                source: Source::Root,
                ends_at: None,
            })?,
        }
    }
    Ok(book)
}

/// One item as a case writes it.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Seen {
    Grant(u8, CannotGiveReason),
    Account(u8, CannotGiveReason),
    Relation(&'static str, CannotGiveReason),
    SignIn,
}

fn ask(
    people: &mut People,
    book: &Book,
    source: GrantId,
    recipient: IdentityId,
) -> Result<CannotGiveList, Box<dyn Error>> {
    let caller = IdentityId::Person(people.p1);
    let request = CannotGiveRequest {
        caller,
        route: Route::Api,
        source,
        recipient,
    };
    let directory = people.projection()?;
    Ok(cannot_give(
        &book.grants,
        directory,
        &book.model,
        &request,
        AT,
    )?)
}

fn grant_number(id: GrantId) -> Result<u8, Box<dyn Error>> {
    let bytes = id.as_bytes();
    if bytes.iter().all(|byte| *byte == bytes[0]) {
        Ok(bytes[0])
    } else {
        Err(format!("{id} is not a fixture grant").into())
    }
}

/// The list as the cases write it, checking on the way that a relation and
/// the sign-in identity each carry their own reason.
fn seen(list: &CannotGiveList) -> Result<Vec<Seen>, Box<dyn Error>> {
    list.items
        .iter()
        .map(|item| -> Result<Seen, Box<dyn Error>> {
            Ok(match &item.subject {
                CannotGiveSubject::Grant(id) => Seen::Grant(grant_number(*id)?, item.reason),
                CannotGiveSubject::ServiceAccount(id) => {
                    Seen::Account(grant_number(*id)?, item.reason)
                }
                CannotGiveSubject::Relation(relation) => {
                    let name = ["alder", "birch", "cedar", "damson"]
                        .into_iter()
                        .find(|name| *name == relation.as_str())
                        .ok_or("an unknown relation")?;
                    assert_eq!(item.reason, AboveWhatYouHold, "a relation's reason");
                    Seen::Relation(name, item.reason)
                }
                CannotGiveSubject::SignInIdentity => {
                    assert_eq!(item.reason, SignInIdentity, "the sign-in identity's reason");
                    Seen::SignIn
                }
            })
        })
        .collect()
}

fn marked(list: &CannotGiveList) -> usize {
    list.items.iter().filter(|item| item.source).count()
}

fn base_for_agent() -> Vec<Seen> {
    vec![
        Seen::Grant(2, UseOnly),
        Seen::Grant(3, LentToYou),
        Seen::Relation("damson", AboveWhatYouHold),
        Seen::SignIn,
    ]
}

fn base_for_person() -> Vec<Seen> {
    vec![
        Seen::Grant(2, UseOnly),
        Seen::Grant(3, LentToYou),
        Seen::Relation("damson", AboveWhatYouHold),
    ]
}

/// The grant an item names, as a grant or as a service account.
fn named(item: &CannotGiveItem) -> Option<GrantId> {
    match item.subject {
        CannotGiveSubject::Grant(id) | CannotGiveSubject::ServiceAccount(id) => Some(id),
        CannotGiveSubject::Relation(_) | CannotGiveSubject::SignInIdentity => None,
    }
}

fn names_grant(list: &CannotGiveList, id: GrantId) -> bool {
    list.items.iter().any(|item| named(item) == Some(id))
}

fn names_relation(list: &CannotGiveList, relation: &str) -> bool {
    list.items.iter().any(|item| {
        matches!(&item.subject, CannotGiveSubject::Relation(listed) if listed.as_str() == relation)
    })
}

#[path = "shared/cannot_give_cases.rs"]
mod cannot_give_cases;
