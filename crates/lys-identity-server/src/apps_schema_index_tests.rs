use lys_identity::grants::{
    Action, AppSchema, Grant, GrantBook, GrantChange, GrantEvent, GrantId, GrantParts, PassOn,
    Relation, Resource, Source, Window,
};
use lys_identity::{IdentityId, OperationId, PersonId};
use serde_json::json;
use std::error::Error;

#[test]
fn schema_checks_visit_no_whole_book_records_and_keep_expiry_and_revocation()
-> Result<(), Box<dyn Error>> {
    let mut book = GrantBook::new();
    let person = PersonId::from_bytes([1; 16]);
    for index in 1..=128_u8 {
        let kind = if index <= 3 {
            "notes.doc"
        } else if index == 4 {
            "project"
        } else {
            "other.doc"
        };
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
            window: Window::new(0, if index == 2 { Some(10) } else { None })?,
            model_version: 1,
            operation,
        })?;
        book.apply(
            &GrantEvent::new(
                operation,
                IdentityId::Person(person),
                1,
                GrantChange::Issue(Box::new(grant)),
            )?,
            u64::from(index),
        )?;
    }
    book.apply(
        &GrantEvent::new(
            OperationId::from_bytes([200; 16]),
            IdentityId::Person(person),
            2,
            GrantChange::Revoke {
                grant: GrantId::from_bytes([1; 16]),
                reason: "finished".to_owned(),
            },
        )?,
        129,
    )?;
    let old = AppSchema::parse(
        "notes",
        &json!({"kinds":{"notes.doc":{"actions":["read"],"relations":{"viewer":["read"]}}}}),
    )?;
    let new = AppSchema::parse(
        "notes",
        &json!({"kinds":{"notes.doc":{"actions":["write"],"relations":{"editor":["write"]}}}}),
    )?;
    let before = GrantBook::record_visits();
    let strands = super::strands(&book, &old, &new, 20);
    assert_eq!(strands.len(), 1);
    assert_eq!(strands[0].count, 1);
    assert_eq!(strands[0].kind, "notes.doc");
    let lys_old =
        AppSchema::lys([(Relation::new("viewer")?, [Action::new("read")?].into())].into());
    let lys_new =
        AppSchema::lys([(Relation::new("editor")?, [Action::new("write")?].into())].into());
    let lys = super::strands(&book, &lys_old, &lys_new, 20);
    assert_eq!(lys.len(), 1);
    assert_eq!(lys[0].kind, "project");
    assert_eq!(lys[0].count, 1);
    assert_eq!(GrantBook::record_visits() - before, 0);
    Ok(())
}
