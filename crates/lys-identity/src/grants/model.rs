//! The permission model: each relation, resolved to the actions it carries.
//!
//! A relation's name is only a name. Whether one relation lies within another
//! authority is answered by comparing the action sets the model resolves them
//! to, never by the order of their names or a rank read from a label. Every
//! answer carries the model version it was made under.
//!
//! The model's own relations are Lys's, and judge every kind written without
//! a prefix. A kind written `{app}.{kind}` is judged only by the relations
//! its approved app's schema gives that kind, under that schema's version; a
//! prefixed kind the model does not hold resolves no relation at all, so no
//! grant is ever judged on a kind no approved app declares.

use std::collections::{BTreeMap, BTreeSet};

use lys_log_store::LeafStore;

use super::authority::Grants;
use super::error::GrantError;
use super::permission::RelationshipStore;
use super::types::{Action, Relation};

/// A versioned model of relations and the actions each carries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Model {
    version: u64,
    relations: BTreeMap<Relation, BTreeSet<Action>>,
    kinds: BTreeMap<String, KindModel>,
}

/// One app kind's relations, under the version of the app's schema that
/// gives them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KindModel {
    /// The app schema's version.
    pub version: u64,
    /// The actions the kind declares.
    pub actions: BTreeSet<Action>,
    /// Each relation of the kind, with the actions it carries.
    pub relations: BTreeMap<Relation, BTreeSet<Action>>,
    /// The kinds whose relations flow down to this one.
    pub parents: BTreeSet<String>,
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
        Ok(Self {
            version,
            relations,
            kinds: BTreeMap::new(),
        })
    }

    /// The same model judging each app kind of `kinds` by its own relations.
    #[must_use]
    pub fn with_kinds(mut self, kinds: BTreeMap<String, KindModel>) -> Self {
        self.kinds = kinds;
        self
    }

    /// Every app kind the model judges, in the order of their names.
    pub fn kinds(&self) -> &BTreeMap<String, KindModel> {
        &self.kinds
    }

    /// The version and relations a resource of `kind` is judged by: the
    /// model's own for a kind without a prefix, its app's for one with.
    fn table(&self, kind: &str) -> Option<(u64, &BTreeMap<Relation, BTreeSet<Action>>)> {
        if !kind.contains('.') {
            return Some((self.version, &self.relations));
        }
        self.kinds
            .get(kind)
            .map(|model| (model.version, &model.relations))
    }

    /// The version a resource of `kind` is judged under: its app schema's,
    /// or the model's own for a kind without a prefix or one it does not hold.
    pub fn version_on(&self, kind: &str) -> u64 {
        self.table(kind)
            .map_or(self.version, |(version, _)| version)
    }

    /// Every relation a resource of `kind` may be held under, with the
    /// actions each carries, in the order of their names; none for a kind
    /// the model does not hold.
    pub fn relations_on(&self, kind: &str) -> impl Iterator<Item = (&Relation, &BTreeSet<Action>)> {
        self.table(kind)
            .into_iter()
            .flat_map(|(_, relations)| relations.iter())
    }

    /// The actions `relation` carries on a resource of `kind`.
    pub fn actions_on(
        &self,
        kind: &str,
        relation: &Relation,
    ) -> Result<&BTreeSet<Action>, GrantError> {
        let unknown = || GrantError::RelationUnknown {
            relation: relation.to_string(),
            model_version: self.version_on(kind),
        };
        let (_, relations) = self.table(kind).ok_or_else(unknown)?;
        relations.get(relation).ok_or_else(unknown)
    }

    /// Whether `requested`, on a resource of `kind`, resolves to actions
    /// that all lie within `held`.
    pub fn within_on(
        &self,
        kind: &str,
        requested: &Relation,
        held: &BTreeSet<Action>,
    ) -> Result<Within, GrantError> {
        let actions = self.actions_on(kind, requested)?;
        let model_version = self.version_on(kind);
        let outside: Vec<&str> = actions.difference(held).map(Action::as_str).collect();
        if !outside.is_empty() {
            return Err(GrantError::ActionsOutside {
                relation: requested.to_string(),
                outside: outside.join(", "),
                model_version,
            });
        }
        Ok(Within {
            relation: requested.clone(),
            actions: actions.clone(),
            model_version,
        })
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

impl<S: LeafStore, R: RelationshipStore> Grants<S, R> {
    /// Judge new requests on app kinds by `kinds` from now on, as the apps'
    /// approved schemas give them. Lys's own relations and version stay as
    /// they are, and grants already held keep their actions and their ends.
    pub fn set_kinds(&mut self, kinds: BTreeMap<String, KindModel>) {
        self.model.kinds = kinds;
    }
}
