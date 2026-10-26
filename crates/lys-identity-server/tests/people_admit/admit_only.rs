//! The admit tests’ own helpers, which the login-move tests do not share.

use std::error::Error;

use identity_contract::fake_rauthy::FakeRauthy;
use identity_contract::harness::Service;

use crate::table::table_with_mail;

pub(super) const CY: &str = "cy@example.test";
pub(super) const STEPS: [&str; 6] = ["register", "account", "bind", "activate", "grant", "letter"];

/// A service with a fake issuer and Ada as its first-run administrator.
pub(super) async fn table() -> Result<(Service, FakeRauthy, String), Box<dyn Error>> {
    table_with_mail(true).await
}

/// The accounts the issuer holds for `email`.
pub(super) fn accounts_for(rauthy: &FakeRauthy, email: &str) -> Result<usize, Box<dyn Error>> {
    Ok(rauthy
        .users()?
        .iter()
        .filter(|user| user["email"] == email)
        .count())
}

/// The people the directory lists, as the administrator reads them.
pub(super) async fn people(service: &Service, ada: &str) -> Result<usize, Box<dyn Error>> {
    let (status, listed) = service.get("/directory/people", Some(ada)).await?;
    assert_eq!(status, 200, "{listed}");
    let people = listed["people"]
        .as_array()
        .or_else(|| listed.as_array())
        .ok_or("the directory lists people")?;
    Ok(people.len())
}
