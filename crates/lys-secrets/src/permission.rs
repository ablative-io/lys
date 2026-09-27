//! How the broker asks whether an identity may use a secret. The broker keeps
//! no permit of its own: every issue and every use asks again.

/// A permit, naming the person the access traces to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Permitted {
    /// The person at the root of the grant chain.
    pub person: String,
}

/// A refusal, with the reason the permission source gave.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Denied {
    /// Why the access is refused.
    pub reason: String,
    /// Whether the access exists but traces to no person.
    pub no_person_root: bool,
}

/// A source of permission: the in-process grants, or `SpiceDB`.
pub trait PermissionCheck {
    /// Whether `identity` holds the `use` relation on `secret`, and which
    /// person it traces to.
    ///
    /// # Errors
    ///
    /// [`Denied`] for every answer that is not a permit.
    fn may_use(&self, identity: &str, secret: &str) -> Result<Permitted, Denied>;
}
