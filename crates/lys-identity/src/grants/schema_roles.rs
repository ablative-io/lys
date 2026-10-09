//! The roles of an app's schema (ACCESS-004 R1): each a name and a set of
//! its kind's actions, so a person is granted `administrator` rather than
//! thirty actions assembled by hand.
//!
//! A kind names its roles under `roles`, `{role: [action, ...]}`. A role's
//! name is one the permission engine takes, since the engine holds it as a
//! relation of its kind: it is never one of the kind's relations or actions
//! and never starts with `parent_`. Each action a role carries is one its own
//! kind declares, named once, and a role carries at least one. A schema
//! written before roles existed names none, and its JSON is written as it was
//! read: `roles` only when a kind names one.
//!
//! A grant naming a role is judged as the role's actions under the schema
//! version current when it is judged, never the version it was issued under.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Map, Value, json};

use super::{
    AppSchema, KindSchema, SchemaError, action, invalid, object, relation, segment, strings,
};
use crate::grants::types::{Action, Relation};

/// The roles a kind's `body` names under `roles`, each carrying only
/// `actions` and named as none of `relations` nor any of `actions`.
pub(super) fn roles(
    body: &Map<String, Value>,
    pointer: &str,
    actions: &BTreeSet<Action>,
    relations: &BTreeSet<&str>,
) -> Result<BTreeMap<Relation, BTreeSet<Action>>, SchemaError> {
    let Some(listed) = body.get("roles") else {
        return Ok(BTreeMap::new());
    };
    let roles_at = format!("{pointer}/roles");
    let mut roles = BTreeMap::new();
    for (name, carried) in object(listed, &roles_at, "roles")? {
        let at = format!("{roles_at}/{}", segment(name));
        let role = relation(&at, name)?;
        if relations.contains(name.as_str()) {
            return Err(invalid(
                &at,
                format!(
                    "the role `{name}` is also a relation of its kind, and the permission engine holds each name of a kind once"
                ),
            ));
        }
        if actions.iter().any(|declared| declared.as_str() == name) {
            return Err(invalid(
                &at,
                format!(
                    "the role `{name}` is also an action of its kind, and the permission engine holds each name of a kind once"
                ),
            ));
        }
        let mut bundle = BTreeSet::new();
        for (action_at, named) in strings(carried, &at, "a role's actions")? {
            let one = action(&action_at, &named)?;
            if !actions.contains(&one) {
                return Err(invalid(
                    &action_at,
                    format!(
                        "the role `{name}` carries `{named}`, which is not an action of its kind"
                    ),
                ));
            }
            if !bundle.insert(one) {
                return Err(invalid(
                    &action_at,
                    format!("the role `{name}` names `{named}` twice"),
                ));
            }
        }
        if bundle.is_empty() {
            return Err(invalid(&at, format!("the role `{name}` carries no action")));
        }
        roles.insert(role, bundle);
    }
    Ok(roles)
}

/// `roles` as the JSON a kind names them under.
pub(super) fn to_json(roles: &BTreeMap<Relation, BTreeSet<Action>>) -> Value {
    let roles: Map<String, Value> = roles
        .iter()
        .map(|(role, actions)| {
            let names: Vec<&str> = actions.iter().map(Action::as_str).collect();
            (role.to_string(), json!(names))
        })
        .collect();
    Value::Object(roles)
}

impl KindSchema {
    /// The actions the role `role` of this kind carries, if the kind names it.
    pub fn role(&self, role: &Relation) -> Option<&BTreeSet<Action>> {
        self.roles.get(role)
    }
}

impl AppSchema {
    /// The actions the role `role` of `kind` carries in this schema.
    pub fn role(&self, kind: &str, role: &Relation) -> Option<&BTreeSet<Action>> {
        self.kind(kind)?.role(role)
    }
}
