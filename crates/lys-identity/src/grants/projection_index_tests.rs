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
