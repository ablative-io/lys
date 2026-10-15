//! Additional recipient, expiry and service-account grant fixtures.

use super::{
    BOTH, Book, Extra, PassOn, People, RecipientKind, Resource, SERVICE_ACCOUNT, Source, Spec, T0,
    TestResult, g, project, to,
};

pub(super) fn add(book: &mut Book, people: &People, extra: Extra) -> TestResult {
    let (admin, holder, lender) = (people.admin, people.p1, people.p3);
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
    Ok(())
}
