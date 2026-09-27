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

/// The relation a permission is asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Relation {
    /// Using a credential or key through the proxy.
    Use,
    /// Reading a memory record.
    Read,
    /// Lending a secret one does not own to another identity.
    Lend,
}

impl Relation {
    /// The relation as it is written.
    pub fn label(self) -> &'static str {
        match self {
            Self::Use => "use",
            Self::Read => "read",
            Self::Lend => "lend",
        }
    }
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

    /// Whether `identity` holds the `read` relation on the sealed record
    /// `record`.
    ///
    /// # Errors
    ///
    /// [`Denied`] for every answer that is not a permit.
    fn may_read(&self, identity: &str, record: &str) -> Result<Permitted, Denied>;

    /// Whether `identity`, not owning `secret`, holds a human-rooted right
    /// to lend it: the `lend` relation.
    ///
    /// # Errors
    ///
    /// [`Denied`] for every answer that is not a permit.
    fn may_lend(&self, identity: &str, secret: &str) -> Result<Permitted, Denied>;
}
