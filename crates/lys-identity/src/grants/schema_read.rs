//! Reading schema members and checking their local action and parent declarations.

use super::{
    Action, BTreeMap, BTreeSet, Map, PARENT_RELATION, Raw, Relation, SchemaError, Value, class,
    engine_name, invalid, roles, segment,
};

pub(super) fn object<'a>(
    value: &'a Value,
    pointer: &str,
    what: &str,
) -> Result<&'a Map<String, Value>, SchemaError> {
    value
        .as_object()
        .ok_or_else(|| invalid(pointer, format!("{what} is a JSON object")))
}

pub(super) fn only(
    map: &Map<String, Value>,
    pointer: &str,
    allowed: &[&str],
) -> Result<(), SchemaError> {
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
pub(super) fn strings(
    value: &Value,
    pointer: &str,
    what: &str,
) -> Result<Vec<(String, String)>, SchemaError> {
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

pub(super) fn action(pointer: &str, name: &str) -> Result<Action, SchemaError> {
    engine_name(pointer, "action", name)?;
    Action::new(name).map_err(|error| invalid(pointer, error.to_string()))
}

pub(super) fn relation(pointer: &str, name: &str) -> Result<Relation, SchemaError> {
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

pub(super) fn raw_kind(pointer: &str, value: &Value) -> Result<Raw, SchemaError> {
    let body = object(value, pointer, "a kind")?;
    only(
        body,
        pointer,
        &[
            "actions",
            "relations",
            "parents",
            "hot",
            "agents",
            "roles",
            "first_grant",
        ],
    )?;
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
    let hot = class::hot(body, pointer, &actions)?;
    let agents = class::agents(body, pointer, &actions)?;
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
    let named: BTreeSet<&str> = relations
        .iter()
        .map(|(_, relation, _)| relation.as_str())
        .collect();
    let roles = roles::roles(body, pointer, &actions, &named)?;
    let first_grant = first_grant(body, pointer, &named)?;
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
        hot,
        agents,
        roles,
        first_grant,
    })
}

/// The kind's `first_grant`, the relation a newly admitted person is given
/// first (DIRECTORY-116 R2): one name among the kind's own relations, or
/// none when the kind declares none.
fn first_grant(
    body: &Map<String, Value>,
    pointer: &str,
    named: &BTreeSet<&str>,
) -> Result<Option<Relation>, SchemaError> {
    let Some(value) = body.get("first_grant") else {
        return Ok(None);
    };
    let at = format!("{pointer}/first_grant");
    let name = value
        .as_str()
        .ok_or_else(|| invalid(&at, "`first_grant` names one relation of the kind"))?;
    if !named.contains(name) {
        let declared = if named.is_empty() {
            "it declares no relation".to_owned()
        } else {
            format!(
                "its relations are {}",
                named.iter().copied().collect::<Vec<_>>().join(", ")
            )
        };
        return Err(invalid(
            &at,
            format!("the first grant `{name}` is not a relation of this kind; {declared}"),
        ));
    }
    relation(&at, name).map(Some)
}

/// The pointer of the parent entry through which `kinds` returns to a kind
/// it started from, the first such entry in the order of the kinds' names.
pub(super) fn cycle(kinds: &BTreeMap<String, Raw>) -> Option<String> {
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
