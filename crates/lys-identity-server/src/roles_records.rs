//! A role, its versions and its holdings, as they are kept.
//!
//! A role is a job and a starting point: what the holder answers for, works
//! towards and how, the profile an agent holding it starts from, and the
//! grants a holder usually needs. Editing a role makes a new version. A
//! holding names the version held, and only a person's act changes it.

use serde::{Deserialize, Serialize};

/// A grant a holder of the role usually needs. It is copied when a grant is
/// made, so it gives nothing by itself.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Template {
    /// The kind of the resource.
    pub resource_kind: String,
    /// The id of the resource.
    pub resource_id: String,
    /// The relation.
    pub relation: String,
    /// How many days such a grant usually runs, null for no end of its own.
    pub days: Option<u32>,
}

/// What a version of a role says.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Words {
    /// What the holder answers for.
    pub responsibilities: String,
    /// What the holder works towards.
    pub goals: String,
    /// How the holder works.
    pub practice: String,
    /// The profile an agent holding the role starts from; may be empty.
    pub profile: String,
    /// The grants a holder usually needs.
    pub grant_templates: Vec<Template>,
    /// Why this version was made.
    pub note: String,
}

/// One version of a role.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Version {
    /// Its number, from 1.
    pub number: u32,
    /// The operation id it was made with.
    pub operation: String,
    /// What it says.
    pub words: Words,
    /// The person who made it.
    pub made_by: String,
    /// When it was made, in seconds since the Unix epoch.
    pub made_at: u64,
}

/// A deliberate move of a holder from one version to another.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Move {
    /// The version moved from.
    pub from: u32,
    /// The version moved to.
    pub to: u32,
    /// The person who moved the holder.
    pub by: String,
    /// When, in seconds since the Unix epoch.
    pub at: u64,
}

/// The end of a holding, by a person's act.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ending {
    /// The person who ended it.
    pub by: String,
    /// When, in seconds since the Unix epoch.
    pub at: u64,
}

/// One identity holding a role.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Holding {
    /// The operation id it was assigned with.
    pub operation: String,
    /// The identity that holds the role.
    pub holder: String,
    /// The version held.
    pub version: u32,
    /// The person who assigned it.
    pub assigned_by: String,
    /// When it was assigned, in seconds since the Unix epoch.
    pub assigned_at: u64,
    /// When it lapses, in seconds since the Unix epoch, null for no end.
    pub ends_at: Option<u64>,
    /// Every move, in order.
    pub moves: Vec<Move>,
    /// Its end by a person's act, null while none ended it.
    pub ended: Option<Ending>,
}

impl Holding {
    /// `ended`, `lapsed` or `holding`, as it stands at `at`.
    pub fn state(&self, at: u64) -> &'static str {
        if self.ended.is_some() {
            "ended"
        } else if self.ends_at.is_some_and(|end| end <= at) {
            "lapsed"
        } else {
            "holding"
        }
    }
}

/// A role as it is kept.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Role {
    /// The operation id it was made with, which names it.
    pub id: String,
    /// Its name.
    pub name: String,
    /// Its versions, in order from 1.
    pub versions: Vec<Version>,
    /// Its holdings, in the order assigned.
    pub holdings: Vec<Holding>,
}

impl Role {
    /// The number of the latest version.
    pub fn latest(&self) -> u32 {
        self.versions.last().map_or(0, |version| version.number)
    }

    /// The version numbered `number`.
    pub fn version(&self, number: u32) -> Option<&Version> {
        self.versions
            .iter()
            .find(|version| version.number == number)
    }

    /// The last holding of `holder`.
    pub fn holding(&self, holder: &str) -> Option<&Holding> {
        self.holdings
            .iter()
            .rev()
            .find(|holding| holding.holder == holder)
    }
}
