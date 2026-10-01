//! Responsibilities a person keeps, even when an agent holds every grant.

/// The exact method and declared route of each retained responsibility.
pub(crate) const KEPT: &[(&str, &str)] = &[
    ("POST", "/setup"),
    ("POST", "/setup/open"),
    ("POST", "/setup/administrator"),
    ("POST", "/setup/password"),
    ("POST", "/grants/roots"),
    ("POST", "/sign-in-providers"),
];
