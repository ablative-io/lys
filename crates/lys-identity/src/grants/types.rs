//! What one grant says.
//!
//! A grant names itself, who issued it, who holds it, the person responsible
//! for the holder, the resource, the relation and the actions it lets the
//! holder exercise, what it lets the holder pass on and to which kinds of
//! recipient, the grant it derives from, its time window, the model version it
//! was judged under and the operation of the event that authorised it.
//!
//! Exercising and passing on are separate members. Pass-on is an affirmative
//! member of every grant: [`PassOn::UseOnly`] is written out, never implied by
//! a missing field, and nothing in this module turns absence into permission.

use std::collections::BTreeSet;
use std::fmt;
use std::str::FromStr;

use super::error::GrantError;
pub use super::grant::{Grant, Mode};
use crate::id::{ID_LEN, IdentityId, PersonId, from_hex, random_bytes, to_hex};
use crate::operation::OperationId;

// Each bound is the permission engine's (`SpiceDB`'s) own for what the field becomes there.
/// The longest action: `SpiceDB` names a permission `[a-z][a-z0-9_]{1,62}[a-z0-9]`, 64 bytes.
pub const ACTION_MAX_BYTES: usize = 64;
/// The longest relation: `SpiceDB` names a relation `[a-z][a-z0-9_]{1,62}[a-z0-9]`, 64 bytes.
pub const RELATION_MAX_BYTES: usize = 64;
/// The longest resource kind: a `SpiceDB` object type is at most 128 bytes.
pub const RESOURCE_KIND_MAX_BYTES: usize = 128;
/// The longest resource id: a `SpiceDB` object id is `[a-zA-Z0-9/_|\-=+]{1,1024}`.
pub const RESOURCE_ID_MAX_BYTES: usize = 1024;

/// A grant's enduring identifier. Its text form is `grant-` and 32 hex digits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct GrantId([u8; ID_LEN]);

impl GrantId {
    const PREFIX: &'static str = "grant-";

    /// A new grant id from the secure random source.
    ///
    /// # Errors
    /// Reports a failed secure random source.
    pub fn generate() -> Result<Self, GrantError> {
        Ok(Self(random_bytes()?))
    }

    /// The grant id these bytes are, as read back from an encoded grant.
    #[must_use]
    pub fn from_bytes(bytes: [u8; ID_LEN]) -> Self {
        Self(bytes)
    }

    /// The grant id's bytes, as an encoded grant carries them.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8; ID_LEN] {
        &self.0
    }
}

impl fmt::Display for GrantId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", Self::PREFIX, to_hex(&self.0))
    }
}

impl FromStr for GrantId {
    type Err = GrantError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        text.strip_prefix(Self::PREFIX)
            .and_then(from_hex)
            .map(Self)
            .ok_or_else(|| GrantError::GrantIdMalformed {
                text: text.to_owned(),
            })
    }
}

/// Whether `byte` may stand in a name the model defines (a kind, an action
/// or a relation): `a-z`, `0-9`, `_`, `-` and `.`.
fn name_byte(byte: u8) -> bool {
    byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"_-.".contains(&byte)
}

/// Whether `byte` may stand in a resource id: `A-Z`, `a-z`, `0-9`, `_`, `-`
/// and `.`. A product's id is named in Lys exactly as the product names it,
/// so no case is folded and two ids that differ only in case are two
/// resources.
pub(crate) fn resource_id_byte(byte: u8) -> bool {
    byte.is_ascii_uppercase() || name_byte(byte)
}

/// Refuse `text` unless it is a token: 1 to `max` bytes, each `allowed`.
fn token(
    kind: &'static str,
    text: &str,
    max: usize,
    allowed: fn(u8) -> bool,
) -> Result<String, GrantError> {
    if text.is_empty() || text.len() > max || !text.bytes().all(allowed) {
        return Err(GrantError::TokenInvalid {
            kind,
            text: text.to_owned(),
            max,
        });
    }
    Ok(text.to_owned())
}

/// One act the model names, such as reading a project.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Action(String);

impl Action {
    /// The action `name`, refused unless it is a token.
    ///
    /// # Errors
    /// Refuses invalid tokens or bounds for the declared grant member.
    pub fn new(name: &str) -> Result<Self, GrantError> {
        token("action", name, ACTION_MAX_BYTES, name_byte).map(Self)
    }

    /// The action's name.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A relation the model defines. Its name says nothing about its actions:
/// only the model resolves a relation to the actions it carries.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Relation(String);

impl Relation {
    /// The relation `name`, refused unless it is a token.
    ///
    /// # Errors
    /// Refuses invalid tokens or bounds for the declared grant member.
    pub fn new(name: &str) -> Result<Self, GrantError> {
        token("relation", name, RELATION_MAX_BYTES, name_byte).map(Self)
    }

    /// The relation's name.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Relation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// The object a grant is on: a kind and an id, as `kind:id`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Resource {
    kind: String,
    id: String,
}

impl Resource {
    /// The resource `id` of `kind`, each refused unless it is a token: the
    /// kind lowercase, the id as the product names it, case and all.
    ///
    /// # Errors
    /// Refuses invalid tokens or bounds for the declared grant member.
    pub fn new(kind: &str, id: &str) -> Result<Self, GrantError> {
        Ok(Self {
            kind: token("resource kind", kind, RESOURCE_KIND_MAX_BYTES, name_byte)?,
            id: token("resource id", id, RESOURCE_ID_MAX_BYTES, resource_id_byte)?,
        })
    }

    /// The resource's kind.
    #[must_use]
    pub fn kind(&self) -> &str {
        &self.kind
    }

    /// The resource's id within its kind.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }
}

impl fmt::Display for Resource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.kind, self.id)
    }
}

/// The kind of identity a grant may be passed on to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum RecipientKind {
    /// A person.
    Person,
    /// An agent.
    Agent,
    /// A service account owned by a person.
    ServiceAccount,
    /// An approved app's connector.
    Connector,
    /// A machine that joined with a connection code (ACCESS-005 R1).
    Machine,
    /// An organisation (DIRECTORY-109 R1). It is never a recipient: a grant
    /// is held by a person or by what answers to one, and an organisation
    /// issues its founding grant rather than holding one. The kind exists
    /// so every identity has one, and pass-on to it is refused by name.
    Organisation,
}

impl RecipientKind {
    /// The kind `identity` is.
    #[must_use]
    pub fn of(identity: IdentityId) -> Self {
        match identity {
            IdentityId::Person(_) => Self::Person,
            IdentityId::Agent(_) => Self::Agent,
            IdentityId::ServiceAccount(_) => Self::ServiceAccount,
            IdentityId::Connector(_) => Self::Connector,
            IdentityId::Machine(_) => Self::Machine,
            IdentityId::Organisation(_) => Self::Organisation,
        }
    }
}

impl fmt::Display for RecipientKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Person => "person",
            Self::Agent => "agent",
            Self::ServiceAccount => "service_account",
            Self::Connector => "connector",
            Self::Machine => "machine",
            Self::Organisation => "organisation",
        })
    }
}

/// What a grant lets its holder pass on, stated affirmatively.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PassOn {
    /// The holder may exercise the grant and pass none of it on.
    UseOnly,
    /// The holder may pass on these actions, to these kinds of recipient.
    To {
        /// The actions that may be passed on.
        actions: BTreeSet<Action>,
        /// The kinds of recipient they may be passed on to.
        recipients: BTreeSet<RecipientKind>,
    },
}

impl PassOn {
    /// Pass-on of `actions` to `recipients`, refused if either is empty.
    ///
    /// # Errors
    /// Refuses empty pass-on actions or recipient kinds.
    pub fn to(
        actions: BTreeSet<Action>,
        recipients: BTreeSet<RecipientKind>,
    ) -> Result<Self, GrantError> {
        if actions.is_empty() {
            return Err(GrantError::AuthorityAbsent {
                member: "pass-on actions",
            });
        }
        if recipients.is_empty() {
            return Err(GrantError::AuthorityAbsent {
                member: "pass-on recipients",
            });
        }
        Ok(Self::To {
            actions,
            recipients,
        })
    }

    /// The actions that may be passed on, or none for a use-only grant.
    #[must_use]
    pub fn actions(&self) -> Option<&BTreeSet<Action>> {
        match self {
            Self::UseOnly => None,
            Self::To { actions, .. } => Some(actions),
        }
    }

    /// Whether the grant may be passed on to a recipient of `kind`.
    #[must_use]
    pub fn permits(&self, kind: RecipientKind) -> bool {
        match self {
            Self::UseOnly => false,
            Self::To { recipients, .. } => recipients.contains(&kind),
        }
    }
}

/// The grant a grant derives from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Source {
    /// A root grant, held by its responsible person under the root authority.
    Root,
    /// A grant derived from another grant.
    Grant(GrantId),
    /// A grant passed on from the administrator's standing.
    Administrator,
}

/// When a grant may be exercised: from `starts_at`, until before `ends_at`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Window {
    starts_at: u64,
    ends_at: Option<u64>,
}

impl Window {
    /// The window from `starts_at` to `ends_at`, or with no end of its own,
    /// in seconds since the Unix epoch. Refused unless it ends after it starts.
    ///
    /// # Errors
    /// Refuses invalid tokens or bounds for the declared grant member.
    pub fn new(starts_at: u64, ends_at: Option<u64>) -> Result<Self, GrantError> {
        if ends_at.is_some_and(|ends| ends <= starts_at) {
            return Err(GrantError::WindowInvalid {
                reason: "a window ends after it starts",
            });
        }
        Ok(Self { starts_at, ends_at })
    }

    /// When the grant starts.
    #[must_use]
    pub fn starts_at(&self) -> u64 {
        self.starts_at
    }

    /// When the grant ends, if it has an end of its own.
    #[must_use]
    pub fn ends_at(&self) -> Option<u64> {
        self.ends_at
    }

    /// Whether `at` is inside the window: at or after the start, before the end.
    #[must_use]
    pub fn contains(&self, at: u64) -> bool {
        at >= self.starts_at && self.ends_at.is_none_or(|ends| at < ends)
    }
}

/// Every member of a grant, before it is checked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrantParts {
    /// The grant's id.
    pub id: GrantId,
    /// The identity that issued it.
    pub issuer: IdentityId,
    /// The identity that holds it.
    pub holder: IdentityId,
    /// The person responsible for the holder.
    pub responsible: PersonId,
    /// The object it is on.
    pub resource: Resource,
    /// The relation it was requested as.
    pub relation: Relation,
    /// The actions it lets the holder exercise, fixed when it was issued.
    pub actions: BTreeSet<Action>,
    /// What it lets the holder pass on.
    pub pass_on: PassOn,
    /// What it derives from.
    pub source: Source,
    /// When it may be exercised.
    pub window: Window,
    /// The model version it was judged under.
    pub model_version: u64,
    /// The operation of the event that authorised it.
    pub operation: OperationId,
}
