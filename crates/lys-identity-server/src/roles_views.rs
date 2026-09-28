//! The typed answers of the role routes, as they go on the wire.

use serde::Serialize;

use crate::grant_contract::ResourceView;
use crate::roles_records::{Move, Template, Version};

/// The policy every role is kept under: a holder stays at the version it
/// holds until a person moves it.
pub const POLICY: &str = "stays_until_moved";

/// A grant a holder of the role usually needs.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct TemplateView {
    /// What the grant would be on.
    pub resource: ResourceView,
    /// The relation.
    pub relation: String,
    /// How many days such a grant usually runs, null for no end of its own.
    pub days: Option<u32>,
}

impl From<&Template> for TemplateView {
    fn from(template: &Template) -> Self {
        Self {
            resource: ResourceView {
                kind: template.resource_kind.clone(),
                id: template.resource_id.clone(),
            },
            relation: template.relation.clone(),
            days: template.days,
        }
    }
}

/// One version of a role.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[schema(as = RoleVersionView)]
pub struct VersionView {
    /// Its number, from 1.
    pub number: u32,
    /// What the holder answers for.
    pub responsibilities: String,
    /// What the holder works towards.
    pub goals: String,
    /// How the holder works.
    pub practice: String,
    /// The profile an agent holding the role starts from; may be empty.
    pub profile: String,
    /// The grants a holder usually needs. They are copied when a grant is
    /// made, so holding the role gives none of them.
    pub grant_templates: Vec<TemplateView>,
    /// The person who made the version.
    pub made_by: String,
    /// When it was made, in seconds since the Unix epoch.
    pub made_at: u64,
    /// Why it was made.
    pub note: String,
}

impl From<&Version> for VersionView {
    fn from(version: &Version) -> Self {
        Self {
            number: version.number,
            responsibilities: version.words.responsibilities.clone(),
            goals: version.words.goals.clone(),
            practice: version.words.practice.clone(),
            profile: version.words.profile.clone(),
            grant_templates: version
                .words
                .grant_templates
                .iter()
                .map(TemplateView::from)
                .collect(),
            made_by: version.made_by.clone(),
            made_at: version.made_at,
            note: version.words.note.clone(),
        }
    }
}

/// A deliberate move of a holder from one version to another.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct MoveView {
    /// The version moved from.
    pub from: u32,
    /// The version moved to.
    pub to: u32,
    /// The person who moved the holder.
    pub by: String,
    /// When, in seconds since the Unix epoch.
    pub at: u64,
}

impl From<&Move> for MoveView {
    fn from(moved: &Move) -> Self {
        Self {
            from: moved.from,
            to: moved.to,
            by: moved.by.clone(),
            at: moved.at,
        }
    }
}

/// One identity holding a role.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[schema(as = RoleHolderView)]
pub struct HolderView {
    /// The operation id the holding was assigned with. A move or an end
    /// names it, so neither acts on a later holding of the same holder.
    pub assignment: String,
    /// The identity that holds the role.
    pub holder: String,
    /// Its display name, null when the directory no longer holds it.
    pub display_name: Option<String>,
    /// The version held.
    pub version: u32,
    /// Whether the role has a newer version than the one held.
    pub behind: bool,
    /// The person who assigned the role.
    pub assigned_by: String,
    /// When it was assigned, in seconds since the Unix epoch.
    pub assigned_at: u64,
    /// When the holding lapses, in seconds since the Unix epoch, null for no
    /// end. No move and no new version changes it.
    pub ends_at: Option<u64>,
    /// When the holder will move to a newer version by itself, which under
    /// the policy `stays_until_moved` is never: always null.
    pub moves_at: Option<u64>,
    /// `holding`, `lapsed` or `ended`.
    pub state: &'static str,
    /// Every move, in order, each with who made it.
    pub moves: Vec<MoveView>,
    /// The person who ended the holding, null while none did.
    pub ended_by: Option<String>,
    /// When it was ended, in seconds since the Unix epoch, null while it was not.
    pub ended_at: Option<u64>,
}

/// One role.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct RoleView {
    /// The role's id, which is the operation id it was made with.
    pub id: String,
    /// Its name.
    pub name: String,
    /// The number of its latest version.
    pub latest: u32,
    /// How holders come to a newer version: `stays_until_moved`.
    pub policy: &'static str,
    /// Its versions, in order from 1.
    pub versions: Vec<VersionView>,
    /// Its holders, in the order assigned.
    pub holders: Vec<HolderView>,
}

/// The answer of `GET /roles`.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct RoleList {
    /// Every role, in the order made.
    pub roles: Vec<RoleView>,
}

/// The answer of a move: the holder as it stands now, and the two versions
/// side by side, so what changes for the holder can be read.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct MovedView {
    /// The role.
    pub role: String,
    /// The holder after the move.
    pub holder: HolderView,
    /// The version the holder was at.
    pub from: VersionView,
    /// The version the holder is at now.
    pub to: VersionView,
}
