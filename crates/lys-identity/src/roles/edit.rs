//! Making a role, making its next version, and changing its title.
//!
//! Each is admitted only for an owner of the role's project, as the seam
//! answers; a directory administrator is admitted only as such an owner. A
//! role is made at version 1 with the default move policy
//! [`MovePolicy::MoveAtNextRenewal`]. Only a change to the grant templates
//! makes a version, and it changes no holding's version, grants, end date or
//! policy, nor any grant made from an earlier version. A title change is
//! recorded and makes no version. Each template's actions are the ones the
//! grants' model resolves its relation to, so a copy carries exactly them.

use lys_log_store::LeafStore;

use super::check::Check;
use super::error::RoleError;
use super::events::{Opening, RoleChange, RoleEvent};
use super::holding::{Acting, Roles};
use super::types::{MovePolicy, RoleId, Template, Version, title};
use crate::grants::types::PROJECT_KIND;
use crate::grants::{RelationshipStore, Resource};
use crate::id::IdentityId;
use crate::operation::OperationId;

/// A request to make a role in a project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MakeRole {
    /// The caller's operation id.
    pub operation: OperationId,
    /// The authenticated actor.
    pub actor: IdentityId,
    /// The project it is defined in.
    pub project: Resource,
    /// Its title.
    pub title: String,
    /// Its first version's grant templates.
    pub templates: Vec<Template>,
}

/// A request to change a role's grant templates, making its next version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditTemplates {
    /// The caller's operation id.
    pub operation: OperationId,
    /// The authenticated actor.
    pub actor: IdentityId,
    /// The role.
    pub role: RoleId,
    /// The next version's grant templates.
    pub templates: Vec<Template>,
}

/// A request to change a role's title.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangeTitle {
    /// The caller's operation id.
    pub operation: OperationId,
    /// The authenticated actor.
    pub actor: IdentityId,
    /// The role.
    pub role: RoleId,
    /// The new title.
    pub title: String,
}

/// Refuse a template whose actions are not the ones the model resolves its relation to.
fn resolved<S: LeafStore, R: RelationshipStore>(
    acting: &Acting<'_, S, R>,
    templates: &[Template],
) -> Result<(), RoleError> {
    for template in templates {
        if acting.grants.model().actions(template.relation())? != template.actions() {
            return Err(RoleError::TemplateInvalid {
                reason: "a template's actions are the ones the model resolves its relation to",
            });
        }
    }
    Ok(())
}

impl Roles {
    /// Make a role at version 1, admitted for an owner of its project.
    pub fn make_role<S: LeafStore, R: RelationshipStore>(
        &mut self,
        acting: &mut Acting<'_, S, R>,
        request: &MakeRole,
    ) -> Result<RoleEvent, RoleError> {
        self.fresh(request.operation)?;
        if request.project.kind() != PROJECT_KIND {
            return Err(RoleError::NotAProject {
                resource: request.project.to_string(),
            });
        }
        let capacity = acting.admit(request.actor, Check::MakeRole, &request.project, None)?;
        let title = title(&request.title)?;
        Version::new(1, request.templates.clone())?;
        resolved(acting, &request.templates)?;
        self.commit(RoleEvent::new(
            request.operation,
            request.actor,
            capacity,
            acting.at,
            RoleChange::VersionMade {
                role: RoleId::generate()?,
                project: request.project.clone(),
                version: 1,
                templates: request.templates.clone(),
                opening: Some(Opening {
                    title,
                    default_policy: MovePolicy::MoveAtNextRenewal,
                }),
            },
        )?)
    }

    /// Change a role's grant templates, making its next version and moving no holder.
    pub fn edit_templates<S: LeafStore, R: RelationshipStore>(
        &mut self,
        acting: &mut Acting<'_, S, R>,
        request: &EditTemplates,
    ) -> Result<RoleEvent, RoleError> {
        self.fresh(request.operation)?;
        let role = self.book().role_or_refuse(request.role)?;
        let project = role.project().clone();
        let next = role.current().number() + 1;
        let capacity = acting.admit(request.actor, Check::EditTemplates, &project, None)?;
        Version::new(next, request.templates.clone())?;
        resolved(acting, &request.templates)?;
        self.commit(RoleEvent::new(
            request.operation,
            request.actor,
            capacity,
            acting.at,
            RoleChange::VersionMade {
                role: request.role,
                project,
                version: next,
                templates: request.templates.clone(),
                opening: None,
            },
        )?)
    }

    /// Change a role's title, making no version.
    pub fn change_title<S: LeafStore, R: RelationshipStore>(
        &mut self,
        acting: &mut Acting<'_, S, R>,
        request: &ChangeTitle,
    ) -> Result<RoleEvent, RoleError> {
        self.fresh(request.operation)?;
        let role = self.book().role_or_refuse(request.role)?;
        let project = role.project().clone();
        let before = role.title().to_owned();
        let capacity = acting.admit(request.actor, Check::ChangeTitle, &project, None)?;
        let after = title(&request.title)?;
        self.commit(RoleEvent::new(
            request.operation,
            request.actor,
            capacity,
            acting.at,
            RoleChange::TitleChanged {
                role: request.role,
                before,
                after,
            },
        )?)
    }
}
