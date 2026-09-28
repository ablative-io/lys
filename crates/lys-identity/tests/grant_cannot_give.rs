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
    book: GrantBook,
    model: Model,
    index: u64,
}

impl Book {
    fn new(model: Model) -> Self {
        Self {
            book: GrantBook::new(),
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
        self.book.apply(&event, self.index)?;
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
        &book.book,
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

#[test]
fn cannot_give_fixture() -> TestResult {
    let mut people = People::new()?;
    let book = fixture(&people, false, &[], false)?;
    let a1 = IdentityId::Agent(people.a1);
    let list = ask(&mut people, &book, g(1), a1)?;
    assert_eq!(list.items.len(), 4, "{list:?}");
    assert_eq!(seen(&list)?, base_for_agent());
    assert!(!names_grant(&list, g(1)), "G1 may be given to an agent");
    assert_eq!(marked(&list), 0, "G1 is not listed, so nothing is marked");
    assert_eq!((list.source, list.recipient), (g(1), a1));
    Ok(())
}

#[test]
fn cannot_give_person_recipient() -> TestResult {
    let mut people = People::new()?;
    let book = fixture(&people, false, &[], false)?;
    let person = IdentityId::Person(people.p2);
    let list = ask(&mut people, &book, g(1), person)?;
    assert_eq!(list.items.len(), 3, "{list:?}");
    assert_eq!(seen(&list)?, base_for_person());
    assert!(list.items.iter().all(|item| item.reason != SignInIdentity));
    Ok(())
}

#[test]
fn cannot_give_precedence() -> TestResult {
    let mut people = People::new()?;
    let book = fixture(&people, false, &[], false)?;
    let mut legs = 0;
    for recipient in [IdentityId::Agent(people.a1), IdentityId::Person(people.p2)] {
        let list = ask(&mut people, &book, g(1), recipient)?;
        let g3: Vec<CannotGiveReason> = list
            .items
            .iter()
            .filter(|item| item.subject == CannotGiveSubject::Grant(g(3)))
            .map(|item| item.reason)
            .collect();
        assert_eq!(
            g3,
            vec![LentToYou],
            "{recipient}: G3 carries lent_to_you alone"
        );
        legs += 1;
    }
    assert_eq!(legs, 2);
    Ok(())
}

#[test]
fn cannot_give_order_table() {
    let mut cases = 0;
    for mask in 1u8..64 {
        let subset: Vec<CannotGiveReason> = CannotGiveReason::ALL
            .into_iter()
            .enumerate()
            .filter(|(bit, _)| mask & (1 << bit) != 0)
            .map(|(_, reason)| reason)
            .collect();
        let reversed: Vec<CannotGiveReason> = subset.iter().rev().copied().collect();
        assert_eq!(
            CannotGiveReason::first(reversed),
            subset.first().copied(),
            "{subset:?}"
        );
        cases += 1;
    }
    assert_eq!(cases, 63);
    assert_eq!(
        CannotGiveReason::ALL.map(CannotGiveReason::name),
        [
            "sign_in_identity",
            "above_what_you_hold",
            "lent_to_you",
            "use_only",
            "people_only",
            "agents_only"
        ]
    );
    assert_eq!(CannotGiveReason::first([]), None);
}

#[test]
fn cannot_give_people_only() -> TestResult {
    let mut people = People::new()?;
    let book = fixture(&people, false, &[Extra::PeopleOnly], false)?;
    let agent = IdentityId::Agent(people.a1);
    let person = IdentityId::Person(people.p2);
    let list = ask(&mut people, &book, g(1), agent)?;
    assert_eq!(list.items.len(), 5, "{list:?}");
    assert_eq!(
        seen(&list)?,
        vec![
            Seen::Grant(2, UseOnly),
            Seen::Grant(3, LentToYou),
            Seen::Grant(11, PeopleOnly),
            Seen::Relation("damson", AboveWhatYouHold),
            Seen::SignIn,
        ]
    );
    let list = ask(&mut people, &book, g(1), person)?;
    assert_eq!(seen(&list)?, base_for_person());
    assert!(!names_grant(&list, g(11)));
    Ok(())
}

#[test]
fn cannot_give_agents_only() -> TestResult {
    let mut people = People::new()?;
    let book = fixture(&people, false, &[Extra::AgentsOnly], false)?;
    let agent = IdentityId::Agent(people.a1);
    let person = IdentityId::Person(people.p2);
    let list = ask(&mut people, &book, g(1), person)?;
    assert_eq!(list.items.len(), 4, "{list:?}");
    assert_eq!(
        seen(&list)?,
        vec![
            Seen::Grant(2, UseOnly),
            Seen::Grant(3, LentToYou),
            Seen::Grant(10, AgentsOnly),
            Seen::Relation("damson", AboveWhatYouHold),
        ]
    );
    let list = ask(&mut people, &book, g(1), agent)?;
    assert_eq!(seen(&list)?, base_for_agent());
    assert!(!names_grant(&list, g(10)));
    Ok(())
}

#[test]
fn cannot_give_in_force_only() -> TestResult {
    let mut people = People::new()?;
    let book = fixture(&people, false, &[Extra::NotInForce], false)?;
    let agent = IdentityId::Agent(people.a1);
    let list = ask(&mut people, &book, g(1), agent)?;
    assert_eq!(list.items.len(), 3, "{list:?}");
    assert_eq!(
        seen(&list)?,
        vec![
            Seen::Grant(2, UseOnly),
            Seen::Grant(3, LentToYou),
            Seen::SignIn
        ]
    );
    for id in [5, 9, 13, 14] {
        assert!(!names_grant(&list, g(id)), "G{id} is not in force");
    }
    assert!(
        !names_relation(&list, "damson"),
        "G9 covers damson, in any standing"
    );
    // Before G9's end passes it is in force, and may be given, so it is still unlisted.
    let request = CannotGiveRequest {
        caller: IdentityId::Person(people.p1),
        route: Route::Browser,
        source: g(1),
        recipient: IdentityId::Agent(people.a1),
    };
    let directory = people.projection()?;
    let early = cannot_give(&book.book, directory, &book.model, &request, T0 + 1)?;
    assert_eq!(seen(&early)?, seen(&list)?);
    Ok(())
}

#[test]
fn cannot_give_source_mark() -> TestResult {
    let mut people = People::new()?;
    let book = fixture(&people, false, &[], false)?;
    let agent = IdentityId::Agent(people.a1);
    let list = ask(&mut people, &book, g(2), agent)?;
    assert_eq!(seen(&list)?, base_for_agent());
    assert_eq!(marked(&list), 1);
    assert!(list.items[0].source, "G2's item is the source");
    assert_eq!(list.items[0].subject, CannotGiveSubject::Grant(g(2)));
    Ok(())
}

#[test]
fn cannot_give_service_account() -> TestResult {
    let mut people = People::new()?;
    let book = fixture(&people, false, &[Extra::ServiceAccount], false)?;
    let agent = IdentityId::Agent(people.a1);
    let list = ask(&mut people, &book, g(1), agent)?;
    assert_eq!(list.items.len(), 5, "{list:?}");
    assert_eq!(
        seen(&list)?,
        vec![
            Seen::Grant(2, UseOnly),
            Seen::Grant(3, LentToYou),
            Seen::Account(7, UseOnly),
            Seen::Relation("damson", AboveWhatYouHold),
            Seen::SignIn,
        ]
    );
    let naming_g7 = list
        .items
        .iter()
        .filter(|item| named(item) == Some(g(7)))
        .count();
    assert_eq!(naming_g7, 1, "SA1 appears once");
    assert!(
        !list.items.contains(&CannotGiveItem {
            subject: CannotGiveSubject::Grant(g(7)),
            reason: UseOnly,
            source: false,
        }),
        "no grant item names G7"
    );
    Ok(())
}

#[test]
fn cannot_give_service_account_givable() -> TestResult {
    let mut people = People::new()?;
    let book = fixture(&people, false, &[Extra::ServiceAccountGivable], false)?;
    let agent = IdentityId::Agent(people.a1);
    let list = ask(&mut people, &book, g(1), agent)?;
    assert_eq!(seen(&list)?, base_for_agent());
    assert!(!names_grant(&list, g(8)));
    Ok(())
}

#[test]
fn cannot_give_no_rank() -> TestResult {
    let mut people = People::new()?;
    let book = fixture(&people, true, &[], false)?;
    let agent = IdentityId::Agent(people.a1);
    let list = ask(&mut people, &book, g(1), agent)?;
    let relations: Vec<Seen> = seen(&list)?
        .into_iter()
        .filter(|item| matches!(item, Seen::Relation(..)))
        .collect();
    assert_eq!(relations, vec![Seen::Relation("alder", AboveWhatYouHold)]);
    Ok(())
}

#[test]
fn cannot_give_admission_agrees() -> TestResult {
    let mut people = People::new()?;
    let book = fixture(&people, false, &[], false)?;
    let (holder, other, agent) = (people.p1, people.p2, people.a1);
    let cases = [(g(1), "alder"), (g(2), "birch"), (g(3), "cedar")];
    let (mut legs, mut permitted, mut refused) = (0, 0, 0);
    for (grant, relation) in cases {
        for (recipient, responsible) in [
            (IdentityId::Agent(agent), holder),
            (IdentityId::Person(other), other),
        ] {
            let list = ask(&mut people, &book, g(1), recipient)?;
            let request = DelegateRequest {
                operation: OperationId::generate()?,
                caller: IdentityId::Person(holder),
                route: Route::Api,
                source: grant,
                recipient,
                responsible,
                resource: project("p")?,
                relation: Relation::new(relation)?,
                pass_on: PassOn::UseOnly,
                window: Window::new(T0, None)?,
            };
            let directory = people.projection()?;
            let admitted =
                judge_delegation(&book.book, directory, &book.model, &request, AT, g(99)).is_ok();
            assert_eq!(
                !names_grant(&list, grant),
                admitted,
                "G{} to {recipient}",
                grant_number(grant)?
            );
            if admitted {
                permitted += 1;
            } else {
                refused += 1;
            }
            legs += 1;
        }
    }
    assert_eq!((legs, permitted, refused), (6, 2, 4));
    Ok(())
}

#[test]
fn cannot_give_deterministic() -> TestResult {
    let mut people = People::new()?;
    let forward = fixture(&people, false, &[], false)?;
    let reversed = fixture(&people, false, &[], true)?;
    let a1 = IdentityId::Agent(people.a1);
    let first = format!("{:?}", ask(&mut people, &forward, g(1), a1)?);
    let second = format!("{:?}", ask(&mut people, &reversed, g(1), a1)?);
    assert_eq!(first.as_bytes(), second.as_bytes());
    Ok(())
}

#[test]
fn cannot_give_refuses_a_source_or_recipient_it_cannot_answer_for() -> TestResult {
    let mut people = People::new()?;
    let book = fixture(&people, false, &[], false)?;
    let a1 = IdentityId::Agent(people.a1);
    let mut refusals = 0;
    for source in [g(0), g(42)] {
        assert!(ask(&mut people, &book, source, a1).is_err(), "{source}");
        refusals += 1;
    }
    let unknown = IdentityId::Agent(AgentId::generate()?);
    assert!(ask(&mut people, &book, g(1), unknown).is_err());
    refusals += 1;
    assert_eq!(refusals, 3);
    Ok(())
}
