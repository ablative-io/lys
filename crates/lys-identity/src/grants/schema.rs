//! An app's permission schema: the resource kinds the app owns, each kind's
//! actions, the relations carrying those actions, and the parent kinds whose
//! relations flow down to it.
//!
//! Invariants, each refused `schema_invalid` at the JSON pointer of the fault
//! when a schema is read:
//!
//! - Every kind is written `{app}.{kind}` under the one app that declares it;
//!   a kind outside the app's prefix is refused naming the prefix that owns it.
//! - A relation carries only actions declared on its own kind. An action
//!   declared on another kind of the schema, and one no kind declares, are two
//!   different faults and are named differently.
//! - A parent is a kind of the same app, and following parents never returns
//!   to the kind it started from.
//! - Every kind, relation and action name is one the permission engine takes:
//!   three to sixty-four lowercase letters, digits and underscores, starting
//!   with a letter and not ending with an underscore. A relation is never
//!   named as an action of its kind, nor with the `parent_` the engine keeps
//!   for parent relations, since the engine holds each name of a kind once.
//!
//! The app `lys` is the one exception to the `{app}.{kind}` form. Its schema
//! is Lys's own model, the relations and the actions each carries, and it
//! declares every kind written without a prefix, so the kinds Lys held before
//! apps were records keep their names. Reading a schema writes nothing.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Map, Value, json};

use super::model::KindModel;
use super::types::{Action, Relation, Resource};

/// The app whose schema is Lys's own model.
pub const LYS_APP: &str = "lys";

/// The key the `lys` schema's one kind is held under: every kind written
/// without a prefix.
pub const ANY_KIND: &str = "*";

/// What the permission engine names a relation from a child to its parent.
pub const PARENT_RELATION: &str = "parent_";

// The app id's length is the brief's (R1, 3 to 40); a relation's, action's or
// kind's name length is the permission engine's own identifier rule, so a
// name the engine would refuse is refused here first, at its pointer.
const APP_MIN: usize = 3;
const APP_MAX: usize = 40;
const NAME_MIN: usize = 3;
const NAME_MAX: usize = 64;

/// Why an app id or a schema is refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SchemaError {
    /// The app id is not one the permission store takes as a prefix.
    #[error("app_id_invalid: `{id}` is not an app id: {reason}")]
    AppIdInvalid {
        /// The id given.
        id: String,
        /// The rule it broke.
        reason: &'static str,
    },
    /// The schema breaks a rule, at the JSON pointer of the fault.
    #[error("schema_invalid: at `{pointer}`: {reason}")]
    Invalid {
        /// The JSON pointer of the fault; empty for the whole schema.
        pointer: String,
        /// The rule it broke, in words.
        reason: String,
    },
}

fn invalid(pointer: &str, reason: impl Into<String>) -> SchemaError {
    SchemaError::Invalid {
        pointer: pointer.to_owned(),
        reason: reason.into(),
    }
}

/// `name` as one JSON pointer segment.
fn segment(name: &str) -> String {
    name.replace('~', "~0").replace('/', "~1")
}

/// Refuse `id` unless it is an app id: three to forty lowercase letters,
/// digits and underscores, starting with a letter. The id is the permission
/// store's prefix unchanged, so a hyphen or a dot is refused.
pub fn app_id(id: &str) -> Result<&str, SchemaError> {
    let refused = |reason| SchemaError::AppIdInvalid {
        id: id.to_owned(),
        reason,
    };
    if !(APP_MIN..=APP_MAX).contains(&id.len()) {
        return Err(refused("an app id is three to forty characters"));
    }
    if !id
        .bytes()
        .next()
        .is_some_and(|byte| byte.is_ascii_lowercase())
    {
        return Err(refused("an app id starts with a lowercase letter"));
    }
    let allowed = |byte: u8| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_';
    if !id.bytes().all(allowed) {
        return Err(refused(
            "an app id holds only lowercase letters, digits and underscores",
        ));
    }
    Ok(id)
}

/// The app a resource kind belongs to: the prefix before its first dot, or
/// `lys` for a kind written without one.
pub fn owner_of(kind: &str) -> &str {
    kind.split_once('.').map_or(LYS_APP, |(prefix, _)| prefix)
}

/// Refuse `name` at `pointer` unless the permission engine takes it.
fn engine_name(pointer: &str, what: &str, name: &str) -> Result<(), SchemaError> {
    let bytes = name.as_bytes();
    let inner = |byte: &u8| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'_';
    let fits = (NAME_MIN..=NAME_MAX).contains(&bytes.len())
        && bytes.first().is_some_and(u8::is_ascii_lowercase)
        && bytes.last().is_some_and(|byte| *byte != b'_')
        && bytes.iter().all(inner);
    if fits {
        return Ok(());
    }
    Err(invalid(
        pointer,
        format!(
            "the {what} `{name}` is not a name the permission engine takes: three to sixty-four lowercase letters, digits and underscores, starting with a letter and not ending with an underscore"
        ),
    ))
}

/// One kind of an app's schema.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KindSchema {
    /// The actions the kind declares.
    pub actions: BTreeSet<Action>,
    /// Each relation, with the actions it carries.
    pub relations: BTreeMap<Relation, BTreeSet<Action>>,
    /// The kinds whose relations flow down to this one.
    pub parents: BTreeSet<String>,
}

/// An app's schema, checked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppSchema {
    app: String,
    kinds: BTreeMap<String, KindSchema>,
}

/// Names as they are read, each with its own JSON pointer.
type Pointed = Vec<(String, String)>;

/// A kind as it is first read, before the kinds are checked against each other.
struct Raw {
    actions: BTreeSet<Action>,
    relations: Vec<(String, Relation, Pointed)>,
    parents: Pointed,
}

fn object<'a>(
    value: &'a Value,
    pointer: &str,
    what: &str,
) -> Result<&'a Map<String, Value>, SchemaError> {
    value
        .as_object()
        .ok_or_else(|| invalid(pointer, format!("{what} is a JSON object")))
}

fn only(map: &Map<String, Value>, pointer: &str, allowed: &[&str]) -> Result<(), SchemaError> {
    match map.keys().find(|key| !allowed.contains(&key.as_str())) {
        Some(key) => Err(invalid(
            &format!("{pointer}/{}", segment(key)),
            format!(
                "`{key}` is not a member here; the members are {}",
                allowed.join(", ")
            ),
        )),
        None => Ok(()),
    }
}

/// Each string of the array at `pointer`, with its own pointer.
fn strings(value: &Value, pointer: &str, what: &str) -> Result<Vec<(String, String)>, SchemaError> {
    let items = value
        .as_array()
        .ok_or_else(|| invalid(pointer, format!("{what} is a JSON array of names")))?;
    items
        .iter()
        .enumerate()
        .map(|(index, item)| {
            let at = format!("{pointer}/{index}");
            item.as_str()
                .map(|text| (at.clone(), text.to_owned()))
                .ok_or_else(|| invalid(&at, format!("each of {what} is a name")))
        })
        .collect()
}

fn action(pointer: &str, name: &str) -> Result<Action, SchemaError> {
    engine_name(pointer, "action", name)?;
    Action::new(name).map_err(|error| invalid(pointer, error.to_string()))
}

fn relation(pointer: &str, name: &str) -> Result<Relation, SchemaError> {
    engine_name(pointer, "relation", name)?;
    if name.starts_with(PARENT_RELATION) {
        return Err(invalid(
            pointer,
            format!(
                "the relation `{name}` starts with `{PARENT_RELATION}`, which the permission engine keeps for parent relations"
            ),
        ));
    }
    Relation::new(name).map_err(|error| invalid(pointer, error.to_string()))
}

fn raw_kind(pointer: &str, value: &Value) -> Result<Raw, SchemaError> {
    let body = object(value, pointer, "a kind")?;
    only(body, pointer, &["actions", "relations", "parents"])?;
    let actions_at = format!("{pointer}/actions");
    let listed = body
        .get("actions")
        .ok_or_else(|| invalid(pointer, "a kind declares its actions"))?;
    let mut actions = BTreeSet::new();
    for (at, name) in strings(listed, &actions_at, "actions")? {
        if !actions.insert(action(&at, &name)?) {
            return Err(invalid(
                &at,
                format!("the action `{name}` is declared twice"),
            ));
        }
    }
    if actions.is_empty() {
        return Err(invalid(&actions_at, "a kind declares at least one action"));
    }
    let mut relations = Vec::new();
    if let Some(value) = body.get("relations") {
        let relations_at = format!("{pointer}/relations");
        for (name, carried) in object(value, &relations_at, "relations")? {
            let at = format!("{relations_at}/{}", segment(name));
            let parsed = relation(&at, name)?;
            if actions.iter().any(|action| action.as_str() == name) {
                return Err(invalid(
                    &at,
                    format!(
                        "the relation `{name}` is also an action of its kind, and the permission engine holds each name of a kind once"
                    ),
                ));
            }
            let carried = strings(carried, &at, "a relation's actions")?;
            if carried.is_empty() {
                return Err(invalid(
                    &at,
                    format!("the relation `{name}` carries no action"),
                ));
            }
            relations.push((at, parsed, carried));
        }
    }
    let parents = match body.get("parents") {
        Some(value) => strings(value, &format!("{pointer}/parents"), "parents")?,
        None => Vec::new(),
    };
    if relations.is_empty() && parents.is_empty() {
        return Err(invalid(
            pointer,
            "a kind has at least one relation or one parent, or nothing could ever act on it",
        ));
    }
    Ok(Raw {
        actions,
        relations,
        parents,
    })
}

/// The pointer of the parent entry through which `kinds` returns to a kind
/// it started from, the first such entry in the order of the kinds' names.
fn cycle(kinds: &BTreeMap<String, Raw>) -> Option<String> {
    for (name, kind) in kinds {
        for (at, parent) in &kind.parents {
            let mut seen = BTreeSet::new();
            let mut next = vec![parent.as_str()];
            while let Some(current) = next.pop() {
                if current == name {
                    return Some(at.clone());
                }
                if seen.insert(current)
                    && let Some(above) = kinds.get(current)
                {
                    next.extend(above.parents.iter().map(|(_, parent)| parent.as_str()));
                }
            }
        }
    }
    None
}

impl AppSchema {
    /// Read the schema `value` of the app `app`, refusing the first fault by
    /// its JSON pointer. The app `lys` is read as Lys's own model.
    pub fn parse(app: &str, value: &Value) -> Result<Self, SchemaError> {
        if app == LYS_APP {
            return Self::parse_lys(value);
        }
        app_id(app)?;
        let root = object(value, "", "a schema")?;
        only(root, "", &["kinds"])?;
        let listed = root
            .get("kinds")
            .ok_or_else(|| invalid("", "a schema names its kinds under `kinds`"))?;
        let listed = object(listed, "/kinds", "kinds")?;
        if listed.is_empty() {
            return Err(invalid("/kinds", "a schema declares at least one kind"));
        }
        let prefix = format!("{app}.");
        let mut raw = BTreeMap::new();
        for (name, body) in listed {
            let at = format!("/kinds/{}", segment(name));
            let Some(local) = name.strip_prefix(&prefix) else {
                return Err(invalid(
                    &at,
                    format!(
                        "the kind `{name}` is outside this app's prefix `{app}`; it belongs to the prefix `{}`",
                        owner_of(name)
                    ),
                ));
            };
            engine_name(&at, "kind", local)?;
            raw.insert(name.clone(), raw_kind(&at, body)?);
        }
        Self::checked(app, raw)
    }

    /// Check the kinds `raw` read against each other.
    fn checked(app: &str, raw: BTreeMap<String, Raw>) -> Result<Self, SchemaError> {
        for (name, kind) in &raw {
            for (_, _, carried) in &kind.relations {
                for (at, action) in carried {
                    if kind
                        .actions
                        .iter()
                        .any(|declared| declared.as_str() == action)
                    {
                        continue;
                    }
                    let elsewhere = raw.iter().find(|(_, other)| {
                        other
                            .actions
                            .iter()
                            .any(|declared| declared.as_str() == action)
                    });
                    return Err(match elsewhere {
                        Some((other, _)) => invalid(
                            at,
                            format!(
                                "the action `{action}` is not declared on the kind `{name}`; it is declared on `{other}`"
                            ),
                        ),
                        None => invalid(
                            at,
                            format!(
                                "the relation carries the unknown action `{action}`: no kind of this schema declares it"
                            ),
                        ),
                    });
                }
            }
            for (at, parent) in &kind.parents {
                let owner = owner_of(parent);
                if owner != app {
                    return Err(invalid(
                        at,
                        format!(
                            "the parent `{parent}` is in another app, the prefix `{owner}`; a parent is a kind of this app"
                        ),
                    ));
                }
                if !raw.contains_key(parent) {
                    return Err(invalid(
                        at,
                        format!("the parent `{parent}` is not a kind of this schema"),
                    ));
                }
            }
        }
        if let Some(at) = cycle(&raw) {
            return Err(invalid(
                &at,
                "following this parent returns to the kind it starts from: parents never form a cycle",
            ));
        }
        let kinds = raw
            .into_iter()
            .map(|(name, kind)| {
                let relations = kind
                    .relations
                    .into_iter()
                    .map(|(_, relation, carried)| {
                        let carries = |action: &&Action| {
                            carried.iter().any(|(_, name)| name == action.as_str())
                        };
                        let actions = kind.actions.iter().filter(carries).cloned().collect();
                        (relation, actions)
                    })
                    .collect();
                let parents = kind.parents.into_iter().map(|(_, parent)| parent).collect();
                (
                    name,
                    KindSchema {
                        actions: kind.actions,
                        relations,
                        parents,
                    },
                )
            })
            .collect();
        Ok(Self {
            app: app.to_owned(),
            kinds,
        })
    }

    /// Read Lys's own model: `{"relations": {relation: [action, ...]}}`.
    fn parse_lys(value: &Value) -> Result<Self, SchemaError> {
        let root = object(value, "", "Lys's schema")?;
        only(root, "", &["relations"])?;
        let listed = root
            .get("relations")
            .ok_or_else(|| invalid("", "Lys's schema names its relations under `relations`"))?;
        let listed = object(listed, "/relations", "relations")?;
        if listed.is_empty() {
            return Err(invalid(
                "/relations",
                "Lys's schema defines at least one relation",
            ));
        }
        let mut relations = BTreeMap::new();
        for (name, carried) in listed {
            let at = format!("/relations/{}", segment(name));
            let parsed = Relation::new(name).map_err(|error| invalid(&at, error.to_string()))?;
            let mut actions = BTreeSet::new();
            for (at, action) in strings(carried, &at, "a relation's actions")? {
                let one = Action::new(&action).map_err(|error| invalid(&at, error.to_string()))?;
                actions.insert(one);
            }
            if actions.is_empty() {
                return Err(invalid(
                    &at,
                    format!("the relation `{name}` carries no action"),
                ));
            }
            relations.insert(parsed, actions);
        }
        Ok(Self::lys(relations))
    }

    /// Lys's own model as the schema of the app `lys`.
    pub fn lys(relations: BTreeMap<Relation, BTreeSet<Action>>) -> Self {
        let actions = relations.values().flatten().cloned().collect();
        let kind = KindSchema {
            actions,
            relations,
            parents: BTreeSet::new(),
        };
        Self {
            app: LYS_APP.to_owned(),
            kinds: BTreeMap::from([(ANY_KIND.to_owned(), kind)]),
        }
    }

    /// The app the schema belongs to.
    pub fn app(&self) -> &str {
        &self.app
    }

    /// Every kind, in the order of their names; the app `lys` holds one,
    /// [`ANY_KIND`].
    pub fn kinds(&self) -> &BTreeMap<String, KindSchema> {
        &self.kinds
    }

    /// The kind of this schema a resource of `kind` is judged by: the kind
    /// itself, or for the app `lys` its one kind when `kind` has no prefix.
    pub fn kind(&self, kind: &str) -> Option<&KindSchema> {
        if self.app == LYS_APP {
            return if kind.contains('.') {
                None
            } else {
                self.kinds.get(ANY_KIND)
            };
        }
        self.kinds.get(kind)
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
                let body = json!({
                    "actions": names(&kind.actions),
                    "relations": relations(kind),
                    "parents": kind.parents.iter().collect::<Vec<_>>(),
                });
                (name.clone(), body)
            })
            .collect();
        json!({"kinds": kinds})
    }

    /// Each kind of an app's schema as the grants judge it, under `version`.
    /// The app `lys` has none: its relations are the model's own.
    pub fn kind_models(&self, version: u64) -> BTreeMap<String, KindModel> {
        if self.app == LYS_APP {
            return BTreeMap::new();
        }
        self.kinds
            .iter()
            .map(|(name, kind)| {
                let model = KindModel {
                    version,
                    actions: kind.actions.clone(),
                    relations: kind.relations.clone(),
                    parents: kind.parents.clone(),
                };
                (name.clone(), model)
            })
            .collect()
    }

    /// The resources whose grants reach `resource`: itself first, then each
    /// resource it is placed in whose kind its own kind lists among its
    /// parents, nearest first. `placed` answers the resource a resource is
    /// placed in. A placement that would return to a resource already
    /// reached ends the walk.
    pub fn reach(
        &self,
        resource: &Resource,
        placed: impl Fn(&Resource) -> Option<Resource>,
    ) -> Vec<Resource> {
        let mut reached = vec![resource.clone()];
        let mut current = resource.clone();
        while let Some(parent) = placed(&current) {
            let flows = self
                .kind(current.kind())
                .is_some_and(|kind| kind.parents.contains(parent.kind()));
            if !flows || reached.contains(&parent) {
                break;
            }
            reached.push(parent.clone());
            current = parent;
        }
        reached
    }
}
