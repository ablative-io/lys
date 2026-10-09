//! ACCESS-006 R5: the live indexes keep only standing authority, prune a
//! revoked chain whole, keep an independent chain, and are rebuilt from the
//! snapshot exactly as applying every event built them.

use std::error::Error;

use crate::grants::{
    Action, Grant, GrantBook, GrantChange, GrantEvent, GrantId, GrantParts, PassOn, RecipientKind,
    Relation, Resource, Source, Window,
};
use crate::{AgentId, IdentityId, OperationId, PersonId};

const PERSON: [u8; 16] = [0xd1; 16];

fn person() -> IdentityId {
    IdentityId::Person(PersonId::from_bytes(PERSON))
}

fn agent() -> IdentityId {
    IdentityId::Agent(AgentId::from_bytes([0xa7; 16]))
}

fn channel() -> Result<Resource, Box<dyn Error>> {
    Ok(Resource::new("rooms.channel", "ward-a")?)
}

/// Grant `id` on the channel, held by `holder`, from `source`, issued by
/// the person or, from a source the agent holds, by the agent.
fn issued(id: u8, holder: IdentityId, source: Source) -> Result<GrantEvent, Box<dyn Error>> {
    let issuer = if source == Source::Grant(GrantId::from_bytes([2; 16])) {
        agent()
    } else {
        person()
    };
    let operation = OperationId::from_bytes([id; 16]);
    let pass_on = PassOn::to(
        [Action::new("read")?].into(),
        [RecipientKind::Person, RecipientKind::Agent].into(),
    )?;
    let grant = Grant::new(GrantParts {
        id: GrantId::from_bytes([id; 16]),
        issuer,
        holder,
        responsible: PersonId::from_bytes(PERSON),
        resource: channel()?,
        relation: Relation::new("reader")?,
        actions: [Action::new("read")?].into(),
        pass_on,
        source,
        window: Window::new(10, Some(20))?,
        model_version: 3,
        operation,
    })?;
    Ok(GrantEvent::new(
        operation,
        issuer,
        u64::from(id),
        GrantChange::Issue(Box::new(grant)),
    )?)
}

fn revoked(op: u8, grant: u8) -> Result<GrantEvent, Box<dyn Error>> {
    Ok(GrantEvent::new(
        OperationId::from_bytes([op; 16]),
        person(),
        u64::from(op),
        GrantChange::Revoke {
            grant: GrantId::from_bytes([grant; 16]),
            reason: "left the ward".to_owned(),
        },
    )?)
}

fn live_holders(book: &GrantBook, holder: IdentityId) -> Vec<GrantId> {
    book.live_held_by(holder)
        .map(|record| record.grant().id())
        .collect()
}

fn live_on(book: &GrantBook) -> Result<Vec<GrantId>, Box<dyn Error>> {
    Ok(book
        .live_on_resource(&channel()?)
        .map(|record| record.grant().id())
        .collect())
}

/// A root (1) passed on to the agent (2) and again to the person (3), and
/// an independent root (4) to the person: revoking 2 prunes 2 and 3, keeps
/// 1 and 4, and the decoded snapshot holds the same live indexes.
#[test]
fn revoking_a_chain_prunes_it_whole_and_keeps_an_independent_one() -> Result<(), Box<dyn Error>> {
    let (one, two, three, four) = (
        GrantId::from_bytes([1; 16]),
        GrantId::from_bytes([2; 16]),
        GrantId::from_bytes([3; 16]),
        GrantId::from_bytes([4; 16]),
    );
    let mut book = GrantBook::new();
    let events = [
        issued(1, person(), Source::Root)?,
        issued(2, agent(), Source::Grant(one))?,
        issued(3, person(), Source::Grant(two))?,
        issued(4, person(), Source::Root)?,
    ];
    for (index, event) in (0..).zip(&events) {
        book.apply(event, index)?;
    }
    assert_eq!(live_holders(&book, person()), [one, three, four]);
    assert_eq!(live_holders(&book, agent()), [two]);
    assert_eq!(book.live_entries(), (4, 4));

    book.apply(&revoked(5, 2)?, 4)?;
    assert_eq!(live_holders(&book, person()), [one, four]);
    assert!(live_holders(&book, agent()).is_empty());
    assert_eq!(live_on(&book)?, [one, four]);
    assert_eq!(book.live_entries(), (2, 2));
    assert_eq!(
        book.held_by(person()).count(),
        3,
        "the holder index still answers the history"
    );

    let decoded = super::super::state::decode(super::super::state::encode(&book)?)?;
    assert_eq!(decoded, book, "the live indexes are rebuilt, not stored");
    assert_eq!(live_holders(&decoded, person()), [one, four]);
    Ok(())
}

/// Revoking the last live chain on a resource returns the live indexes to
/// what they kept before it was issued: no entry for the holder or the
/// resource survives it.
#[test]
fn the_last_chain_revoked_leaves_no_live_entry() -> Result<(), Box<dyn Error>> {
    let mut book = GrantBook::new();
    assert_eq!(book.live_entries(), (0, 0));
    book.apply(&issued(1, person(), Source::Root)?, 0)?;
    book.apply(
        &issued(2, agent(), Source::Grant(GrantId::from_bytes([1; 16])))?,
        1,
    )?;
    assert_eq!(book.live_entries(), (2, 2));
    book.apply(&revoked(3, 1)?, 2)?;
    assert_eq!(book.live_entries(), (0, 0));
    assert!(live_on(&book)?.is_empty());
    assert_eq!(book.records().count(), 2, "the history is kept");
    let decoded = super::super::state::decode(super::super::state::encode(&book)?)?;
    assert_eq!(decoded.live_entries(), (0, 0));
    Ok(())
}
