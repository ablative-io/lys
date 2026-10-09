//! The class of each action of an app's schema (ACCESS-001 R2, D2, D4), and
//! the schema written back as JSON.
//!
//! A kind names its hot actions under `hot`: decided from the caller's pass
//! alone, offline. Every action it does not name is deliberate, asked of Lys
//! live. So a schema written before the class existed has every action
//! deliberate, and its JSON is written as it was read: `hot` only when a kind
//! names one. A hot action is one its kind declares, named once. A grant held
//! by draft or by two never applies to a hot action. Roles are written the
//! same way: `roles` only when a kind names one (ACCESS-004 R1).

use std::collections::BTreeSet;

use serde_json::{Map, Value, json};

use super::{ANY_KIND, AppSchema, KindSchema, LYS_APP, SchemaError, action, invalid, strings};
use crate::grants::types::{Action, Relation};

/// The hot actions a kind's `body` names under `hot`, each one of `actions`.
pub(super) fn hot(
    body: &Map<String, Value>,
    pointer: &str,
    actions: &BTreeSet<Action>,
) -> Result<BTreeSet<Action>, SchemaError> {
    marked(body, pointer, actions, ("hot", "hot actions", "hot action"))
}

/// The actions an agent may hold that a kind's `body` names under `agents`,
/// each one of `actions`. A kind that names none gives an agent none: the
/// default is no agent act, and an app opts in act by act.
pub(super) fn agents(
    body: &Map<String, Value>,
    pointer: &str,
    actions: &BTreeSet<Action>,
) -> Result<BTreeSet<Action>, SchemaError> {
    marked(
        body,
        pointer,
        actions,
        ("agents", "an agent's actions", "agent's action"),
    )
}

/// The actions `body` names under `key`, each one of `actions`, named once.
fn marked(
    body: &Map<String, Value>,
    pointer: &str,
    actions: &BTreeSet<Action>,
    (key, what, one): (&str, &str, &str),
) -> Result<BTreeSet<Action>, SchemaError> {
    let Some(listed) = body.get(key) else {
        return Ok(BTreeSet::new());
    };
    let mut marked = BTreeSet::new();
    for (at, name) in strings(listed, &format!("{pointer}/{key}"), what)? {
        let named = action(&at, &name)?;
        if !actions.contains(&named) {
            return Err(invalid(
                &at,
                format!("the {one} `{name}` is not an action of its kind"),
            ));
        }
        if !marked.insert(named) {
            return Err(invalid(&at, format!("the {one} `{name}` is named twice")));
        }
    }
    Ok(marked)
}

impl KindSchema {
    /// Whether `action` is decided from the pass alone; every other action
    /// of the kind is deliberate.
    pub fn is_hot(&self, action: &Action) -> bool {
        self.hot.contains(action)
    }
}

impl AppSchema {
    /// The first hot action, in name order, that `relation` of `kind` carries,
    /// or the role of that name (ACCESS-004 R1): a grant held by draft or by
    /// two under it is refused.
    pub fn hot_action(&self, kind: &str, relation: &Relation) -> Option<&Action> {
        let kind = self.kind(kind)?;
        kind.relations
            .get(relation)
            .or_else(|| kind.roles.get(relation))?
            .iter()
            .find(|action| kind.is_hot(action))
    }

    /// The schema as JSON, in the form it is read from.
    pub fn to_json(&self) -> Value {
        let names = |actions: &BTreeSet<Action>| -> Vec<String> {
            actions.iter().map(ToString::to_string).collect()
        };
        let relations = |kind: &KindSchema| -> Map<String, Value> {
            kind.relations
                .iter()
                .map(|(relation, actions)| (relation.to_string(), json!(names(actions))))
                .collect()
        };
        if self.app == LYS_APP {
            let kind = self.kinds.get(ANY_KIND);
            return json!({"relations": kind.map(relations).unwrap_or_default()});
        }
        let kinds: Map<String, Value> = self
            .kinds
            .iter()
            .map(|(name, kind)| {
                let mut body = json!({
                    "actions": names(&kind.actions),
                    "relations": relations(kind),
                    "parents": kind.parents.iter().collect::<Vec<_>>(),
                });
                if !kind.hot.is_empty() {
                    body["hot"] = json!(names(&kind.hot));
                }
                if !kind.agents.is_empty() {
                    body["agents"] = json!(names(&kind.agents));
                }
                if !kind.roles.is_empty() {
                    body["roles"] = super::roles::to_json(&kind.roles);
                }
                (name.clone(), body)
            })
            .collect();
        json!({"kinds": kinds})
    }
}
