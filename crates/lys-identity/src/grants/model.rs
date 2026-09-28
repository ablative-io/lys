//! The permission model: each relation, resolved to the actions it carries.
//!
//! A relation's name is only a name. Whether one relation lies within another
//! authority is answered by comparing the action sets the model resolves them
//! to, never by the order of their names or a rank read from a label. Every
//! answer carries the model version it was made under.

use std::collections::{BTreeMap, BTreeSet};

use super::error::GrantError;
use super::types::{Action, Relation};

/// A versioned model of relations and the actions each carries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Model {
    version: u64,
    relations: BTreeMap<Relation, BTreeSet<Action>>,
}

/// A relation the model placed within an authority, with what it resolved to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Within {
    /// The relation requested.
    pub relation: Relation,
    /// The actions the model resolved it to.
    pub actions: BTreeSet<Action>,
    /// The model version the answer was made under.
    pub model_version: u64,
}

impl Model {
    /// The model `version` defining `relations`, refused if the version is 0,
    /// there is no relation, or a relation carries no action.
    pub fn new(
        version: u64,
        relations: impl IntoIterator<Item = (Relation, BTreeSet<Action>)>,
    ) -> Result<Self, GrantError> {
        if version == 0 {
            return Err(GrantError::ModelInvalid {
                reason: "a model version is 1 or more",
            });
        }
        let relations: BTreeMap<_, _> = relations.into_iter().collect();
        if relations.is_empty() {
            return Err(GrantError::ModelInvalid {
                reason: "a model defines at least one relation",
            });
        }
        if relations.values().any(BTreeSet::is_empty) {
            return Err(GrantError::ModelInvalid {
                reason: "every relation carries at least one action",
            });
        }
        Ok(Self { version, relations })
    }

    /// The model's version.
    pub fn version(&self) -> u64 {
        self.version
    }

    /// Every relation the model defines, with the actions it carries, in
    /// the order of their names.
    pub fn relations(&self) -> impl Iterator<Item = (&Relation, &BTreeSet<Action>)> {
        self.relations.iter()
    }

    /// The actions `relation` carries in this model.
    pub fn actions(&self, relation: &Relation) -> Result<&BTreeSet<Action>, GrantError> {
        self.relations
            .get(relation)
            .ok_or_else(|| GrantError::RelationUnknown {
                relation: relation.to_string(),
                model_version: self.version,
            })
    }

    /// Whether `requested` resolves to actions that all lie within `held`.
    pub fn within(
        &self,
        requested: &Relation,
        held: &BTreeSet<Action>,
    ) -> Result<Within, GrantError> {
        let actions = self.actions(requested)?;
        let outside: Vec<&str> = actions.difference(held).map(Action::as_str).collect();
        if !outside.is_empty() {
            return Err(GrantError::ActionsOutside {
                relation: requested.to_string(),
                outside: outside.join(", "),
                model_version: self.version,
            });
        }
        Ok(Within {
            relation: requested.clone(),
            actions: actions.clone(),
            model_version: self.version,
        })
    }
}
