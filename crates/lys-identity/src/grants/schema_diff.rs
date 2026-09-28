//! What a schema change does: the kinds, relations, actions and parents it
//! adds and removes, and the standing grants a removal would strand.
//!
//! A grant is stranded when its kind leaves the schema, its relation leaves
//! its kind, or its kind stops declaring an action the grant carries. A
//! grant keeps the actions it was issued with, so a relation that only
//! carries fewer actions strands nothing. Both answers read and never write:
//! the dry run and the refused change are the same computation, and neither
//! revokes or rewrites a grant.

use std::collections::{BTreeMap, BTreeSet};

use super::schema::{AppSchema, KindSchema, owner_of};
use super::types::{Action, Relation};

/// A name within one kind: a relation, an action or a parent.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Named {
    /// The kind.
    pub kind: String,
    /// The name within it.
    pub name: String,
}

/// What a change from one schema to another adds and removes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SchemaDiff {
    /// Kinds the new schema declares and the old does not.
    pub kinds_added: Vec<String>,
    /// Kinds the old schema declares and the new does not.
    pub kinds_removed: Vec<String>,
    /// Relations added to a kind both declare.
    pub relations_added: Vec<Named>,
    /// Relations removed from a kind both declare.
    pub relations_removed: Vec<Named>,
    /// Actions added to a kind both declare.
    pub actions_added: Vec<Named>,
    /// Actions removed from a kind both declare.
    pub actions_removed: Vec<Named>,
    /// Parents added to a kind both declare.
    pub parents_added: Vec<Named>,
    /// Parents removed from a kind both declare.
    pub parents_removed: Vec<Named>,
}

impl SchemaDiff {
    /// Whether the change changes nothing.
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

fn named<'a>(kind: &str, names: impl Iterator<Item = &'a str>) -> Vec<Named> {
    names
        .map(|name| Named {
            kind: kind.to_owned(),
            name: name.to_owned(),
        })
        .collect()
}

fn relation_names(kind: &KindSchema) -> BTreeSet<&str> {
    kind.relations.keys().map(Relation::as_str).collect()
}

fn action_names(kind: &KindSchema) -> BTreeSet<&str> {
    kind.actions.iter().map(Action::as_str).collect()
}

fn parent_names(kind: &KindSchema) -> BTreeSet<&str> {
    kind.parents.iter().map(String::as_str).collect()
}

/// What changing `old` to `new` adds and removes, each list in name order.
pub fn diff(old: &AppSchema, new: &AppSchema) -> SchemaDiff {
    let mut change = SchemaDiff::default();
    let (before, after) = (old.kinds(), new.kinds());
    change.kinds_added = after
        .keys()
        .filter(|kind| !before.contains_key(*kind))
        .cloned()
        .collect();
    change.kinds_removed = before
        .keys()
        .filter(|kind| !after.contains_key(*kind))
        .cloned()
        .collect();
    for (name, was) in before {
        let Some(now) = after.get(name) else {
            continue;
        };
        let (added, removed) = both_ways(name, &relation_names(was), &relation_names(now));
        change.relations_added.extend(added);
        change.relations_removed.extend(removed);
        let (added, removed) = both_ways(name, &action_names(was), &action_names(now));
        change.actions_added.extend(added);
        change.actions_removed.extend(removed);
        let (added, removed) = both_ways(name, &parent_names(was), &parent_names(now));
        change.parents_added.extend(added);
        change.parents_removed.extend(removed);
    }
    change
}

/// The names `now` holds and `was` does not, then those `was` holds and `now` does not.
fn both_ways(kind: &str, was: &BTreeSet<&str>, now: &BTreeSet<&str>) -> (Vec<Named>, Vec<Named>) {
    (
        named(kind, now.difference(was).copied()),
        named(kind, was.difference(now).copied()),
    )
}

/// A standing grant, as the strand count reads it.
#[derive(Debug, Clone, Copy)]
pub struct Standing<'a> {
    /// The kind of the resource it is on.
    pub kind: &'a str,
    /// Its relation.
    pub relation: &'a Relation,
    /// The actions it carries.
    pub actions: &'a BTreeSet<Action>,
}

/// The standing grants of `old`'s app that changing to `new` would strand,
/// counted by the kind and relation they are held under.
pub fn stranded<'a>(
    old: &AppSchema,
    new: &AppSchema,
    standing: impl IntoIterator<Item = Standing<'a>>,
) -> BTreeMap<Named, u64> {
    let mut counted = BTreeMap::new();
    for grant in standing {
        if owner_of(grant.kind) != old.app() {
            continue;
        }
        let strands = new.kind(grant.kind).is_none_or(|kind| {
            !kind.relations.contains_key(grant.relation) || !grant.actions.is_subset(&kind.actions)
        });
        if strands {
            let key = Named {
                kind: grant.kind.to_owned(),
                name: grant.relation.to_string(),
            };
            *counted.entry(key).or_insert(0) += 1;
        }
    }
    counted
}
