//! A checked grant and the way its actions are exercised.

use super::error::GrantError;
use super::types::{Action, GrantId, GrantParts, PassOn, Resource, Source, Window};
use crate::{IdentityId, OrganisationId, PersonId};
use std::collections::BTreeSet;

/// How a grant's actions may be exercised (ACCESS-001 R1, D3): outright, or
/// held until a draft is approved, by one approver or by two.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Mode {
    /// Exercised at once. Every grant written before the mode existed is outright.
    Outright,
    /// Exercised through a draft one approver approves.
    ByDraft,
    /// Exercised through a draft two approvers approve.
    ByTwo,
}

impl Mode {
    /// The mode in words, as the API and the console name it.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Outright => "outright",
            Self::ByDraft => "by_draft",
            Self::ByTwo => "by_two",
        }
    }

    /// Whether exercise is held for approval.
    #[must_use]
    pub fn is_held(self) -> bool {
        self != Self::Outright
    }
}

/// A checked grant: every member present, pass-on inside exercise, lineage well formed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Grant {
    parts: GrantParts,
    once: bool,
    mode: Mode,
    role: bool,
    organisation: Option<OrganisationId>,
}

impl Grant {
    /// The grant `parts` describe, refused by name if they break the contract.
    ///
    /// # Errors
    /// Refuses empty authority, pass-on outside exercise, invalid model versions or malformed lineage.
    pub fn new(parts: GrantParts) -> Result<Self, GrantError> {
        if parts.actions.is_empty() {
            return Err(GrantError::AuthorityAbsent { member: "actions" });
        }
        if let PassOn::To {
            actions,
            recipients,
        } = &parts.pass_on
        {
            PassOn::to(actions.clone(), recipients.clone())?;
            if !actions.is_subset(&parts.actions) {
                return Err(GrantError::PassOnOutside);
            }
        }
        if parts.model_version == 0 {
            return Err(GrantError::ModelInvalid {
                reason: "a model version is 1 or more",
            });
        }
        match parts.source {
            Source::Root if parts.holder != IdentityId::Person(parts.responsible) => {
                Err(GrantError::LineageMalformed {
                    reason: "a root grant is held by its responsible person",
                })
            }
            Source::Grant(source) if source == parts.id => Err(GrantError::LineageMalformed {
                reason: "a grant does not derive from itself",
            }),
            Source::Administrator
                if !matches!(parts.issuer, IdentityId::Person(_) | IdentityId::Agent(_)) =>
            {
                Err(GrantError::LineageMalformed {
                    reason: "administrator standing is passed on by a person or an agent",
                })
            }
            Source::Root | Source::Grant(_) | Source::Administrator => Ok(Self {
                parts,
                once: false,
                mode: Mode::Outright,
                role: false,
                organisation: None,
            }),
        }
    }

    /// The grant marked as belonging to `organisation` (DIRECTORY-109 R3):
    /// the holder's, read from the directory when it was judged, and never
    /// defaulted. `None` keeps a grant written before organisations as it
    /// was; the boot of DIRECTORY-109 R2 stamps every such grant once.
    #[must_use]
    pub fn with_organisation(mut self, organisation: Option<OrganisationId>) -> Self {
        self.organisation = organisation;
        self
    }

    /// The organisation the grant belongs to, when it carries one.
    #[must_use]
    pub fn organisation(&self) -> Option<OrganisationId> {
        self.organisation
    }

    /// The one-time grant `parts` describe: spent by its first exercise,
    /// and use-only, since what may be used once is never passed on.
    ///
    /// # Errors
    /// Refuses further pass-on or any malformed grant member.
    pub fn once(parts: GrantParts) -> Result<Self, GrantError> {
        if parts.pass_on != PassOn::UseOnly {
            return Err(GrantError::UseOnly {
                grant: parts.id.to_string(),
            });
        }
        let mut grant = Self::new(parts)?;
        grant.once = true;
        Ok(grant)
    }

    /// Whether the grant is spent by its first exercise.
    #[must_use]
    pub fn is_once(&self) -> bool {
        self.once
    }

    /// The grant exercised in `mode`.
    #[must_use]
    pub fn with_mode(mut self, mode: Mode) -> Self {
        self.mode = mode;
        self
    }

    /// How the grant's actions may be exercised.
    #[must_use]
    pub fn mode(&self) -> Mode {
        self.mode
    }

    /// The grant naming, as its relation, a role of its resource's kind
    /// (ACCESS-004 R1): judged as the role's actions under the schema
    /// version current when it is judged. Its own actions are the role's
    /// when it was issued, kept as the record of what it first gave.
    #[must_use]
    pub fn as_role(mut self) -> Self {
        self.role = true;
        self
    }

    /// Whether the grant's relation names a role of its kind.
    #[must_use]
    pub fn names_role(&self) -> bool {
        self.role
    }

    /// Refused `ModeHeld` unless the grant is exercised at once: a held
    /// grant's act is taken through an approved draft.
    ///
    /// # Errors
    /// Refuses a grant whose mode requires an approved draft.
    pub fn exercisable(&self) -> Result<(), GrantError> {
        if self.mode.is_held() {
            return Err(GrantError::ModeHeld {
                grant: self.parts.id.to_string(),
                mode: self.mode.as_str(),
            });
        }
        Ok(())
    }

    /// Every member of the grant.
    #[must_use]
    pub fn parts(&self) -> &GrantParts {
        &self.parts
    }

    /// The grant's id.
    #[must_use]
    pub fn id(&self) -> GrantId {
        self.parts.id
    }

    /// The identity that holds it.
    #[must_use]
    pub fn holder(&self) -> IdentityId {
        self.parts.holder
    }

    /// The person responsible for the holder.
    #[must_use]
    pub fn responsible(&self) -> PersonId {
        self.parts.responsible
    }

    /// The object it is on.
    #[must_use]
    pub fn resource(&self) -> &Resource {
        &self.parts.resource
    }

    /// The actions the holder may exercise.
    #[must_use]
    pub fn actions(&self) -> &BTreeSet<Action> {
        &self.parts.actions
    }

    /// What the holder may pass on.
    #[must_use]
    pub fn pass_on(&self) -> &PassOn {
        &self.parts.pass_on
    }

    /// What it derives from.
    #[must_use]
    pub fn source(&self) -> Source {
        self.parts.source
    }

    /// When it may be exercised.
    #[must_use]
    pub fn window(&self) -> Window {
        self.parts.window
    }
}
