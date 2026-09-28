#![cfg(test)]
//! The grant representation in `SpiceDB` (ADR-078, C5) as a pure mapping:
//! what each committed grant event does to the relationships, checked
//! against updates written by hand from the recorded contract and schema.
//! Nothing here reaches an engine.

use std::collections::BTreeSet;
use std::error::Error;

use lys_identity::grants::relationships::{Planned, Step, Update, plan};
use lys_identity::grants::{
    Action, Grant, GrantBook, GrantChange, GrantEvent, GrantId, GrantParts, PassOn,
    ProjectionRefusal, RecipientKind, Relation, Resource, Route, Source, Window,
};
use lys_identity::{AgentId, IdentityId, OperationId, PersonId};

type TestResult = Result<(), Box<dyn Error>>;

/// When every window starts.
const T0: u64 = 1_800_000_000;

/// The people and agents of every test.
struct Cast {
    root: PersonId,
    person: PersonId,
    agent: AgentId,
    other: AgentId,
}

fn cast() -> Result<Cast, Box<dyn Error>> {
    Ok(Cast {
        root: PersonId::generate()?,
        person: PersonId::generate()?,
        agent: AgentId::generate()?,
        other: AgentId::generate()?,
    })
}

fn read() -> Result<Action, Box<dyn Error>> {
    Ok(Action::new("read")?)
}

fn project_x() -> Result<Resource, Box<dyn Error>> {
    Ok(Resource::new("project", "x")?)
}

/// A grant of read on X to `holder`, issued by `issuer`, standing on `source`.
fn grant(
    (issuer, holder, responsible): (IdentityId, IdentityId, PersonId),
    source: Source,
    ends: Option<u64>,
) -> Result<Grant, Box<dyn Error>> {
    Ok(Grant::new(GrantParts {
        id: GrantId::generate()?,
        issuer,
        holder,
        responsible,
        resource: project_x()?,
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
    })?)
}

/// Fold `event` into `book` at `index`, keeping a refusal as the grants do.
fn fold(book: &mut GrantBook, event: &GrantEvent, index: u64) {
    if let Err(refusal) = book.apply(event, index) {
        book.refuse(event, index, refusal);
    }
}

fn issue(book: &mut GrantBook, index: u64, grant: Grant) -> Result<GrantId, Box<dyn Error>> {
    let id = grant.id();
    let event = GrantEvent::new(
        grant.parts().operation,
        grant.parts().issuer,
        T0 + index,
        GrantChange::Issue(Box::new(grant)),
    )?;
    fold(book, &event, index);
    Ok(id)
}

fn revoke(book: &mut GrantBook, index: u64, by: IdentityId, grant: GrantId) -> TestResult {
    let event = GrantEvent::new(
        OperationId::generate()?,
        by,
        T0 + index,
        GrantChange::Revoke {
            grant,
            reason: "withdrawn".to_owned(),
        },
    )?;
    fold(book, &event, index);
    Ok(())
}

/// An update as text: `touch` or `delete`, then `type:id#relation@type:id`
/// and the window caveat when there is one.
fn render(update: &Update) -> String {
    let (verb, tuple) = match update {
        Update::Touch(tuple) => ("touch", tuple),
        Update::Delete(tuple) => ("delete", tuple),
    };
    let caveat = tuple.window.map_or_else(String::new, |window| {
        let ends = window
            .ends_at
            .map_or_else(String::new, |ends| format!(" ends_at={ends}"));
        format!(
            " [{} starts_at={}{ends}]",
            window.caveat(),
            window.starts_at
        )
    });
    let (resource, subject) = (&tuple.resource, &tuple.subject);
    format!(
        "{verb} {}:{}#{}@{}:{}{caveat}",
        resource.kind, resource.id, tuple.relation, subject.kind, subject.id
    )
}

fn sorted(updates: &[Update]) -> Vec<String> {
    let mut text: Vec<String> = updates.iter().map(render).collect();
    text.sort();
    text
}

fn sorted_lines(lines: &[String]) -> Vec<String> {
    let mut lines = lines.to_vec();
    lines.sort();
    lines
}

fn only(planned: Vec<Planned>) -> Result<Planned, Box<dyn Error>> {
    let mut planned = planned.into_iter();
    let one = planned.next().ok_or("nothing was planned")?;
    if planned.next().is_some() {
        return Err("more than one event was planned".into());
    }
    Ok(one)
}

#[test]
fn a_root_grant_is_its_three_relationships_beside_the_root_authoritys_anchor() -> TestResult {
    let cast = cast()?;
    let mut book = GrantBook::new();
    let (root, person) = (
        IdentityId::Person(cast.root),
        IdentityId::Person(cast.person),
    );
    let rp = issue(
        &mut book,
        0,
        grant((root, person, cast.person), Source::Root, Some(T0 + 500))?,
    )?;
    let planned = only(plan(&book, 1, 1, cast.root))?;
    assert_eq!(planned.position, 1);
    let Ok(Step::Issue(updates)) = planned.step else {
        return Err(format!("a root issue planned {:?}", planned.step).into());
    };
    let expected = [
        "touch root_authority:directory#anyone@person:*".to_owned(),
        "touch root_authority:directory#anyone@agent:*".to_owned(),
        format!("touch resource:project/x/read#granted@grant:{rp}"),
        format!(
            "touch grant:{rp}#holder@person:{} [within_window starts_at={T0} ends_at={}]",
            cast.person,
            T0 + 500
        ),
        format!("touch grant:{rp}#root@root_authority:directory"),
    ];
    assert_eq!(sorted(&updates), sorted_lines(&expected));
    Ok(())
}

#[test]
fn a_delegated_grant_stands_on_its_source_and_a_revoke_deletes_its_standing_first() -> TestResult {
    let cast = cast()?;
    let mut book = GrantBook::new();
    let (root, person) = (
        IdentityId::Person(cast.root),
        IdentityId::Person(cast.person),
    );
    let (agent, other) = (IdentityId::Agent(cast.agent), IdentityId::Agent(cast.other));
    let rp = issue(
        &mut book,
        0,
        grant((root, person, cast.person), Source::Root, None)?,
    )?;
    let g1 = issue(
        &mut book,
        1,
        grant((person, agent, cast.person), Source::Grant(rp), None)?,
    )?;
    let g2 = issue(
        &mut book,
        2,
        grant((agent, other, cast.person), Source::Grant(g1), None)?,
    )?;
    let twin = issue(
        &mut book,
        3,
        grant((person, agent, cast.person), Source::Grant(rp), None)?,
    )?;
    revoke(&mut book, 4, person, g1)?;

    let planned = only(plan(&book, 2, 2, cast.root))?;
    let Ok(Step::Issue(updates)) = planned.step else {
        return Err(format!("a delegation planned {:?}", planned.step).into());
    };
    let expected = [
        format!("touch resource:project/x/read#granted@grant:{g1}"),
        format!(
            "touch grant:{g1}#holder@agent:{} [started starts_at={T0}]",
            cast.agent
        ),
        format!("touch grant:{g1}#source@grant:{rp}"),
    ];
    assert_eq!(sorted(&updates), sorted_lines(&expected));

    let planned = only(plan(&book, 5, 5, cast.root))?;
    let Ok(Step::Revoke {
        grant: withdrawn,
        standing,
        rest,
    }) = planned.step
    else {
        return Err(format!("a revoke planned {:?}", planned.step).into());
    };
    assert_eq!(withdrawn, g1);
    assert_eq!(
        render(&standing),
        format!("delete grant:{g1}#source@grant:{rp}")
    );
    let expected = [
        format!("delete resource:project/x/read#granted@grant:{g1}"),
        format!(
            "delete grant:{g1}#holder@agent:{} [started starts_at={T0}]",
            cast.agent
        ),
        format!("delete resource:project/x/read#granted@grant:{g2}"),
        format!(
            "delete grant:{g2}#holder@agent:{} [started starts_at={T0}]",
            cast.other
        ),
        format!("delete grant:{g2}#source@grant:{g1}"),
    ];
    assert_eq!(sorted(&rest), sorted_lines(&expected));
    assert!(
        !rest
            .iter()
            .any(|update| render(update).contains(&twin.to_string()))
    );
    Ok(())
}

#[test]
fn a_committed_delegation_ending_after_its_source_is_refused_by_name() -> TestResult {
    let cast = cast()?;
    let mut book = GrantBook::new();
    let (root, person) = (
        IdentityId::Person(cast.root),
        IdentityId::Person(cast.person),
    );
    let agent = IdentityId::Agent(cast.agent);
    let rp = issue(
        &mut book,
        0,
        grant((root, person, cast.person), Source::Root, Some(T0 + 500))?,
    )?;
    issue(
        &mut book,
        1,
        grant(
            (person, agent, cast.person),
            Source::Grant(rp),
            Some(T0 + 900),
        )?,
    )?;
    let planned = only(plan(&book, 2, 2, cast.root))?;
    assert_eq!(
        planned.step,
        Err(ProjectionRefusal::ExpiryPastSource {
            position: 2,
            source_grant: rp.to_string(),
            source_ends: T0 + 500,
            requested: (T0 + 900).to_string(),
        })
    );
    Ok(())
}

#[test]
fn a_root_sourced_grant_not_issued_by_the_root_authority_stands_on_nothing() -> TestResult {
    let cast = cast()?;
    let mut book = GrantBook::new();
    let person = IdentityId::Person(cast.person);
    let agent = IdentityId::Agent(cast.agent);
    let stray = issue(
        &mut book,
        0,
        grant((agent, person, cast.person), Source::Root, None)?,
    )?;
    let planned = only(plan(&book, 1, 1, cast.root))?;
    assert_eq!(
        planned.step,
        Err(ProjectionRefusal::StandingMissing {
            position: 1,
            grant: stray.to_string(),
        })
    );
    Ok(())
}

#[test]
fn a_use_changes_no_relationship() -> TestResult {
    let cast = cast()?;
    let mut book = GrantBook::new();
    let person = IdentityId::Person(cast.person);
    let rp = issue(
        &mut book,
        0,
        grant(
            (IdentityId::Person(cast.root), person, cast.person),
            Source::Root,
            None,
        )?,
    )?;
    let used = GrantEvent::new(
        OperationId::generate()?,
        person,
        T0 + 1,
        GrantChange::Use {
            grant: rp,
            route: Route::Api,
        },
    )?;
    fold(&mut book, &used, 1);
    let planned = only(plan(&book, 2, 2, cast.root))?;
    assert_eq!(planned.step, Ok(Step::Unchanged));
    Ok(())
}
