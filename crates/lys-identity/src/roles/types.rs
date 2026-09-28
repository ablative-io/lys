//! The records the role acts act on: a role, its versions, their grant
//! templates, and the holdings of agents.
//!
//! A role has a stable id, exactly one project it is defined in, a title, a
//! default move policy and its versions, numbered 1 and one more than the
//! last for each after it. A version's grant templates are grants with no
//! holder and no source, written in the grant fields relation, resource,
//! actions, pass-on, window and responsible; a committed version never
//! changes. A holding names its agent, its role, the version it holds, the
//! project it sits in (always its role's), the grants it carries (the grant
//! copied from the version's template at each index), its end date or none
//! (the one written on each grant it carries) and its own move policy. A
//! holding with no end date moves only by a deliberate act.
//!
//! Every value set here is closed: a missing field is never read as a
//! default, and no default grants authority.

use std::collections::BTreeSet;
use std::fmt;
use std::str::FromStr;

use super::error::RoleError;
use crate::grants::{Action, GrantId, PassOn, Relation, Resource, Window};
use crate::id::{AgentId, ID_LEN, PersonId, from_hex, random_bytes, to_hex};

/// The most characters a role's title carries.
pub const TITLE_MAX_CHARS: usize = 100;

/// A role's enduring identifier. Its text form is `role-` and 32 hex digits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RoleId([u8; ID_LEN]);

impl RoleId {
    const PREFIX: &'static str = "role-";

    /// A new role id from the secure random source.
    pub fn generate() -> Result<Self, RoleError> {
        Ok(Self(random_bytes()?))
    }

    /// The role id these bytes are.
    pub fn from_bytes(bytes: [u8; ID_LEN]) -> Self {
        Self(bytes)
    }

    /// The role id's bytes.
    pub fn as_bytes(&self) -> &[u8; ID_LEN] {
        &self.0
    }
}

impl fmt::Display for RoleId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", Self::PREFIX, to_hex(&self.0))
    }
}

impl FromStr for RoleId {
    type Err = RoleError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        text.strip_prefix(Self::PREFIX)
            .and_then(from_hex)
            .map(Self)
            .ok_or_else(|| RoleError::IdMalformed {
                kind: "role",
                text: text.to_owned(),
            })
    }
}

/// A holding's enduring identifier. Its text form is `holding-` and 32 hex digits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct HoldingId([u8; ID_LEN]);

impl HoldingId {
    const PREFIX: &'static str = "holding-";

    /// A new holding id from the secure random source.
    pub fn generate() -> Result<Self, RoleError> {
        Ok(Self(random_bytes()?))
    }

    /// The holding id these bytes are.
    pub fn from_bytes(bytes: [u8; ID_LEN]) -> Self {
        Self(bytes)
    }

    /// The holding id's bytes.
    pub fn as_bytes(&self) -> &[u8; ID_LEN] {
        &self.0
    }
}

impl fmt::Display for HoldingId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", Self::PREFIX, to_hex(&self.0))
    }
}

impl FromStr for HoldingId {
    type Err = RoleError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        text.strip_prefix(Self::PREFIX)
            .and_then(from_hex)
            .map(Self)
            .ok_or_else(|| RoleError::IdMalformed {
                kind: "holding",
                text: text.to_owned(),
            })
    }
}

/// When a holding moves to a newer version of its role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MovePolicy {
    /// Its next renewal moves it to the role's current version.
    MoveAtNextRenewal,
    /// It moves only by a deliberate act.
    DeliberateOnly,
}

impl MovePolicy {
    /// The policy's name.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::MoveAtNextRenewal => "move_at_next_renewal",
            Self::DeliberateOnly => "deliberate_only",
        }
    }
}

impl fmt::Display for MovePolicy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for MovePolicy {
    type Err = RoleError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        match text {
            "move_at_next_renewal" => Ok(Self::MoveAtNextRenewal),
            "deliberate_only" => Ok(Self::DeliberateOnly),
            _ => Err(RoleError::UnknownPolicy {
                text: text.to_owned(),
            }),
        }
    }
}

/// The capacity an actor acted in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Capacity {
    /// The person the holder answers to.
    ResponsiblePerson,
    /// A holder of the owner relation on the role's project.
    ProjectOwner,
}

impl Capacity {
    /// The capacity's name.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ResponsiblePerson => "responsible_person",
            Self::ProjectOwner => "project_owner",
        }
    }
}

impl fmt::Display for Capacity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Capacity {
    type Err = RoleError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        match text {
            "responsible_person" => Ok(Self::ResponsiblePerson),
            "project_owner" => Ok(Self::ProjectOwner),
            _ => Err(RoleError::UnknownCapacity {
                text: text.to_owned(),
            }),
        }
    }
}

/// When a move takes effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Timing {
    /// At the holder's next start.
    NextStart,
    /// Now, every open session of the holder stopped first.
    Now,
}

impl Timing {
    /// The timing's name.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NextStart => "next_start",
            Self::Now => "now",
        }
    }
}

impl fmt::Display for Timing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Timing {
    type Err = RoleError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        match text {
            "next_start" => Ok(Self::NextStart),
            "now" => Ok(Self::Now),
            _ => Err(RoleError::UnknownTiming {
                text: text.to_owned(),
            }),
        }
    }
}

/// Every member of a grant template, before it is checked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateParts {
    /// The relation its grant is requested as.
    pub relation: Relation,
    /// The object its grant is on.
    pub resource: Resource,
    /// The actions its grant carries.
    pub actions: BTreeSet<Action>,
    /// What its grant lets the holder pass on.
    pub pass_on: PassOn,
    /// Its grant's window, with no end of its own.
    pub window: Window,
    /// The responsible value written in the role; every copy replaces it with
    /// the holder's own responsible person.
    pub responsible: PersonId,
}

/// A checked grant template: a grant with no holder and no source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Template {
    parts: TemplateParts,
}

impl Template {
    /// The template `parts` describe, refused by name if they break the rules.
    pub fn new(parts: TemplateParts) -> Result<Self, RoleError> {
        if parts.actions.is_empty() {
            return Err(RoleError::TemplateInvalid {
                reason: "a template carries at least one action",
            });
        }
        if parts
            .pass_on
            .actions()
            .is_some_and(|passed| !passed.is_subset(&parts.actions))
        {
            return Err(RoleError::TemplateInvalid {
                reason: "a template passes on only actions it carries",
            });
        }
        if parts.window.ends_at().is_some() {
            return Err(RoleError::TemplateHasEnd);
        }
        Ok(Self { parts })
    }

    /// Every member of the template.
    pub fn parts(&self) -> &TemplateParts {
        &self.parts
    }

    /// The relation.
    pub fn relation(&self) -> &Relation {
        &self.parts.relation
    }

    /// The resource.
    pub fn resource(&self) -> &Resource {
        &self.parts.resource
    }

    /// The actions.
    pub fn actions(&self) -> &BTreeSet<Action> {
        &self.parts.actions
    }

    /// The pass-on.
    pub fn pass_on(&self) -> &PassOn {
        &self.parts.pass_on
    }

    /// The window.
    pub fn window(&self) -> Window {
        self.parts.window
    }

    /// The responsible value written in the role.
    pub fn responsible(&self) -> PersonId {
        self.parts.responsible
    }
}

/// A committed version of a role: its number and its grant templates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Version {
    number: u64,
    templates: Vec<Template>,
}

impl Version {
    /// Version `number` carrying `templates`, refused unless the number is 1
    /// or more and the templates are one or more, each once.
    pub fn new(number: u64, templates: Vec<Template>) -> Result<Self, RoleError> {
        if number == 0 {
            return Err(RoleError::TemplateInvalid {
                reason: "a version number is 1 or more",
            });
        }
        if templates.is_empty() {
            return Err(RoleError::TemplateInvalid {
                reason: "a version carries at least one template",
            });
        }
        let repeated = templates
            .iter()
            .enumerate()
            .any(|(index, template)| templates[..index].contains(template));
        if repeated {
            return Err(RoleError::TemplateInvalid {
                reason: "a version carries each template once",
            });
        }
        Ok(Self { number, templates })
    }

    /// The version's number.
    pub fn number(&self) -> u64 {
        self.number
    }

    /// The version's grant templates, in order.
    pub fn templates(&self) -> &[Template] {
        &self.templates
    }
}

/// The title `text`, refused unless it is 1 to 100 characters and not blank.
pub fn title(text: &str) -> Result<String, RoleError> {
    if text.trim().is_empty() || text.chars().count() > TITLE_MAX_CHARS {
        return Err(RoleError::TitleInvalid);
    }
    Ok(text.to_owned())
}

/// A role: defined in one project, with its title, default policy and versions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Role {
    id: RoleId,
    project: Resource,
    title: String,
    default_policy: MovePolicy,
    earlier: Vec<Version>,
    current: Version,
}

impl Role {
    /// The role `id` in `project`, opened at its first version.
    pub(crate) fn open(
        id: RoleId,
        project: Resource,
        title: String,
        default_policy: MovePolicy,
        first: Version,
    ) -> Self {
        Self {
            id,
            project,
            title,
            default_policy,
            earlier: Vec::new(),
            current: first,
        }
    }

    /// The role's id.
    pub fn id(&self) -> RoleId {
        self.id
    }

    /// The one project the role is defined in.
    pub fn project(&self) -> &Resource {
        &self.project
    }

    /// The role's title.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// The move policy a holding with an end date is granted with.
    pub fn default_policy(&self) -> MovePolicy {
        self.default_policy
    }

    /// The current version: the last one made.
    pub fn current(&self) -> &Version {
        &self.current
    }

    /// Version `number`, if the role has it.
    pub fn version(&self, number: u64) -> Option<&Version> {
        self.versions().find(|version| version.number == number)
    }

    /// Every version, first to current.
    pub fn versions(&self) -> impl Iterator<Item = &Version> {
        self.earlier.iter().chain(std::iter::once(&self.current))
    }

    /// Version `number` or its refusal by name.
    pub fn version_or_refuse(&self, number: u64) -> Result<&Version, RoleError> {
        self.version(number)
            .ok_or_else(|| RoleError::VersionUnknown {
                role: self.id.to_string(),
                version: number,
            })
    }

    pub(crate) fn push(&mut self, version: Version) {
        let previous = std::mem::replace(&mut self.current, version);
        self.earlier.push(previous);
    }

    pub(crate) fn set_title(&mut self, title: String) {
        self.title = title;
    }

    pub(crate) fn set_default(&mut self, policy: MovePolicy) {
        self.default_policy = policy;
    }
}

/// One agent's holding of a role.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Holding {
    /// The holding's id.
    pub id: HoldingId,
    /// The agent holding the role.
    pub holder: AgentId,
    /// The role held.
    pub role: RoleId,
    /// The version held.
    pub version: u64,
    /// The project it sits in: always its role's.
    pub project: Resource,
    /// The grants it carries: the grant copied from the held version's
    /// template at each index.
    pub grants: Vec<GrantId>,
    /// Its end date, written on each grant it carries, or none.
    pub ends_at: Option<u64>,
    /// Its own move policy.
    pub policy: MovePolicy,
}

impl Holding {
    /// Refuse a holding that breaks the record's rules: a version of 1 or
    /// more, one grant or more, and a move at the next renewal only with an
    /// end date.
    pub fn check(&self) -> Result<(), RoleError> {
        if self.version == 0 {
            return Err(RoleError::VersionUnknown {
                role: self.role.to_string(),
                version: 0,
            });
        }
        if self.grants.is_empty() {
            return Err(RoleError::GrantsMismatch {
                reason: "a holding carries one grant for each template of its version",
            });
        }
        if self.policy == MovePolicy::MoveAtNextRenewal && self.ends_at.is_none() {
            return Err(RoleError::NoEndDate {
                holding: self.id.to_string(),
            });
        }
        Ok(())
    }

    /// Whether the holding has lapsed at `at`: its end date is at or before it.
    pub fn lapsed(&self, at: u64) -> bool {
        self.ends_at.is_some_and(|ends| at >= ends)
    }
}
