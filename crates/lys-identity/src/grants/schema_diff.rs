//! What a schema change does: the kinds, relations, actions, parents and
//! roles it adds and removes, each role it widens or narrows, and the
//! standing grants a removal would strand.
//!
//! A grant is stranded when its kind leaves the schema, its relation leaves
//! its kind, or its kind stops declaring an action the grant carries. A grant
//! naming a role is stranded when the role leaves its kind or is narrowed,
//! since it is judged as the role's actions under the current version
//! (ACCESS-004 R1); a narrowing no standing grant names strands nothing. A
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
    /// Roles added to a kind both declare.
    pub roles_added: Vec<Named>,
    /// Roles removed from a kind both declare.
    pub roles_removed: Vec<Named>,
    /// Roles both declare whose actions change, in kind and role order.
    pub roles_changed: Vec<RoleChange>,
}

/// A role whose actions a change widens, narrows, or both.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoleChange {
    /// The kind and the role.
    pub role: Named,
    /// The actions it gains: a widening, which an app's change waits on the
    /// administrator to take.
    pub added: Vec<String>,
    /// The actions it loses: a narrowing.
    pub removed: Vec<String>,
}

impl RoleChange {
    /// The change in words, as the Apps screen names it: "adds `seat_retire`
    /// to administrator", "removes `post` from member".
    pub fn words(&self) -> Vec<String> {
        let role = &self.role.name;
        self.added
            .iter()
            .map(|action| format!("adds {action} to {role}"))
            .chain(
                self.removed
                    .iter()
                    .map(|action| format!("removes {action} from {role}")),
            )
            .collect()
    }
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

fn role_names(kind: &KindSchema) -> BTreeSet<&str> {
    kind.roles.keys().map(Relation::as_str).collect()
}

/// Each role `was` and `now` both name whose actions differ.
fn roles_changed(name: &str, was: &KindSchema, now: &KindSchema) -> Vec<RoleChange> {
    let names = |actions: &BTreeSet<Action>| -> BTreeSet<&str> {
        actions.iter().map(Action::as_str).collect()
    };
    was.roles
        .iter()
        .filter_map(|(role, before)| {
            let after = now.roles.get(role)?;
            let (before, after) = (names(before), names(after));
            let added: Vec<String> = after
                .difference(&before)
                .map(|action| (*action).to_owned())
                .collect();
            let removed: Vec<String> = before
                .difference(&after)
                .map(|action| (*action).to_owned())
                .collect();
            (!added.is_empty() || !removed.is_empty()).then(|| RoleChange {
                role: Named {
                    kind: name.to_owned(),
                    name: role.to_string(),
                },
                added,
                removed,
            })
        })
        .collect()
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
        let (added, removed) = both_ways(name, &role_names(was), &role_names(now));
        change.roles_added.extend(added);
        change.roles_removed.extend(removed);
        change.roles_changed.extend(roles_changed(name, was, now));
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
    /// Whether it is held by draft or by two, so a change making one of its
    /// actions hot strands it (ACCESS-001 R2).
    pub held: bool,
    /// Whether its relation names a role of its kind, so a change removing
    /// or narrowing that role strands it (ACCESS-004 R1).
    pub role: bool,
}

/// The standing grants of `old`'s app that changing to `new` would strand,
/// counted by the kind and relation they are held under: one whose relation
/// or actions go, one held by draft or by two whose action becomes hot, and
/// one naming a role the change removes or narrows.
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
            let named = if grant.role {
                narrowed(old, kind, grant)
            } else {
                !kind.relations.contains_key(grant.relation)
            };
            named
                || !grant.actions.is_subset(&kind.actions)
                || (grant.held && grant.actions.iter().any(|action| kind.is_hot(action)))
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

/// Whether `kind` of the new schema drops or narrows the role `grant` names,
/// against what `old` gave that role.
fn narrowed(old: &AppSchema, kind: &KindSchema, grant: Standing<'_>) -> bool {
    let Some(now) = kind.role(grant.relation) else {
        return true;
    };
    old.role(grant.kind, grant.relation)
        .is_some_and(|was| !was.is_subset(now))
}
