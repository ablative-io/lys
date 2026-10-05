use super::{GrantBook, GrantChange, GrantEvent};
use crate::grants::{
    Action, Grant, GrantId, GrantParts, PassOn, Relation, Resource, Source, Window,
};
use crate::{IdentityId, OperationId, PersonId};
use std::error::Error;

#[test]
fn kind_index_rebuilds_from_the_unchanged_snapshot_and_retains_order() -> Result<(), Box<dyn Error>>
{
    let mut book = GrantBook::new();
    let person = PersonId::from_bytes([1; 16]);
    for (index, kind) in [
        (3, "notes.doc"),
        (2, "other.doc"),
        (1, "notes.doc"),
        (4, "project"),
    ] {
        let operation = OperationId::from_bytes([index; 16]);
        let grant = Grant::new(GrantParts {
            id: GrantId::from_bytes([index; 16]),
            issuer: IdentityId::Person(person),
            holder: IdentityId::Person(person),
            responsible: person,
            resource: Resource::new(kind, "one")?,
            relation: Relation::new("viewer")?,
            actions: [Action::new("read")?].into(),
            pass_on: PassOn::UseOnly,
            source: Source::Root,
            window: Window::new(0, None)?,
            model_version: 1,
            operation,
        })?;
        let event = GrantEvent::new(
            operation,
            IdentityId::Person(person),
            1,
            GrantChange::Issue(Box::new(grant)),
        )?;
        book.apply(&event, u64::from(index))?;
    }
    let encoded = super::state::encode(&book)?;
    let decoded = super::state::decode(encoded.clone())?;
    assert_eq!(super::state::encode(&decoded)?, encoded);
    assert_eq!(decoded, book);
    let before = GrantBook::record_visits();
    let ids: Vec<_> = decoded
        .in_app("notes")
        .map(|record| record.grant().id())
        .collect();
    assert_eq!(
        ids,
        [GrantId::from_bytes([1; 16]), GrantId::from_bytes([3; 16])]
    );
    assert_eq!(decoded.in_app("lys").count(), 1);
    assert_eq!(decoded.in_app("absent").count(), 0);
    assert_eq!(GrantBook::record_visits() - before, 0);
    Ok(())
}

/// The grant state written at main d71e8670, whose encoders are those of
/// 5215ed6c, from the inputs below: a person's root, passed on to an agent
/// and to a service account, the agent's then revoked. It is never
/// regenerated (DIRECTORY-080 R1); the state's writer is private to the crate.
const GRANT_STATE: &str = include_str!("../../tests/fixtures/grant-state-d71e8670.hex");
/// The lowercase hexadecimal digits.
const DIGITS: &[u8; 16] = b"0123456789abcdef";

/// A grant on one resource from the person, from `source`, held by `holder`.
fn issued(
    id: u8,
    holder: IdentityId,
    source: Source,
    pass_on: PassOn,
) -> Result<GrantEvent, Box<dyn Error>> {
    let person = PersonId::from_bytes([0xd1; 16]);
    let operation = OperationId::from_bytes([id; 16]);
    let grant = Grant::new(GrantParts {
        id: GrantId::from_bytes([id; 16]),
        issuer: IdentityId::Person(person),
        holder,
        responsible: person,
        resource: Resource::new("project", "alpha")?,
        relation: Relation::new("viewer")?,
        actions: [Action::new("read")?].into(),
        pass_on,
        source,
        window: Window::new(10, Some(20))?,
        model_version: 3,
        operation,
    })?;
    Ok(GrantEvent::new(
        operation,
        IdentityId::Person(person),
        u64::from(id),
        GrantChange::Issue(Box::new(grant)),
    )?)
}

#[test]
fn a_grant_state_holding_a_person_an_agent_and_a_service_account_keeps_its_bytes()
-> Result<(), Box<dyn Error>> {
    use crate::grants::RecipientKind;
    let person = PersonId::from_bytes([0xd1; 16]);
    let root = GrantId::from_bytes([1; 16]);
    let events = [
        issued(
            1,
            IdentityId::Person(person),
            Source::Root,
            PassOn::to(
                [Action::new("read")?].into(),
                [
                    RecipientKind::Person,
                    RecipientKind::Agent,
                    RecipientKind::ServiceAccount,
                ]
                .into(),
            )?,
        )?,
        issued(
            2,
            IdentityId::Agent(crate::AgentId::from_bytes([0xa7; 16])),
            Source::Grant(root),
            PassOn::UseOnly,
        )?,
        issued(
            3,
            IdentityId::ServiceAccount(crate::ServiceAccountId::from_bytes([0x5a; 16])),
            Source::Grant(root),
            PassOn::UseOnly,
        )?,
        GrantEvent::new(
            OperationId::from_bytes([4; 16]),
            IdentityId::Person(person),
            4,
            GrantChange::Revoke {
                grant: GrantId::from_bytes([2; 16]),
                reason: "fixture".to_owned(),
            },
        )?,
    ];
    let mut book = GrantBook::new();
    for (index, event) in (0..).zip(&events) {
        book.apply(event, index)?;
    }
    let mut written = String::new();
    for byte in crate::grants::state::encode(&book, 4)? {
        written.push(char::from(DIGITS[usize::from(byte >> 4)]));
        written.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    let fixture = GRANT_STATE.trim();
    assert_eq!(
        fixture, written,
        "today's writer no longer writes the fixture's grant state"
    );
    let bytes = (0..fixture.len())
        .step_by(2)
        .map(|at| u8::from_str_radix(&fixture[at..at + 2], 16))
        .collect::<Result<Vec<u8>, _>>()?;
    assert_eq!(
        crate::grants::state::decode(&bytes, 4)?,
        book,
        "today's reader"
    );
    Ok(())
}
