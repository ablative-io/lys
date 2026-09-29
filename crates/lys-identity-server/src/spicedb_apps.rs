//! An approved app's kinds in the permission engine's schema language.
//!
//! The kind `{app}.{kind}` is the definition `{app}/{kind}`, under the app's
//! own prefix and no other. It holds the kind's relations, each of subject
//! `grant#holder`; a `parent_` relation to each parent kind; and one
//! permission per declared action, carried by the relations that carry the
//! action and by the same permission on each parent that declares it, so a
//! relation held on a parent flows to the children placed in it. A kind with
//! no term for an action names `nil`, which the engine takes as nothing.

use std::collections::BTreeMap;

use lys_identity::grants::{KindModel, parent_relation};

/// The definition a kind is held under: `{app}/{kind}` for an app kind, and
/// the kind's own name for one of Lys's.
pub(crate) fn definition(kind: &str) -> String {
    kind.replacen('.', "/", 1)
}

/// Each app kind's definition in the schema text `text`, by kind: every
/// block from a line `definition {app}/{kind} {` to the line `}` closing it,
/// or the one line alone when it closes where it opens.
pub(crate) fn app_definitions(text: &str) -> BTreeMap<String, String> {
    let mut found = BTreeMap::new();
    let mut open: Option<(String, Vec<&str>)> = None;
    for line in text.lines() {
        if let Some((kind, lines)) = &mut open {
            lines.push(line);
            if line.trim_end() == "}" {
                found.insert(kind.clone(), lines.join("\n"));
                open = None;
            }
            continue;
        }
        let named = line
            .strip_prefix("definition ")
            .and_then(|rest| rest.split([' ', '{']).next())
            .filter(|name| name.contains('/'));
        match named {
            Some(name) if line.trim_end().ends_with('}') => {
                found.insert(name.replacen('/', ".", 1), line.to_owned());
            }
            Some(name) => open = Some((name.replacen('/', ".", 1), vec![line])),
            None => {}
        }
    }
    found
}

/// The definition of the app kind `kind` as `model` gives it, its parents
/// read from `kinds`: its relations, a parent relation to each parent, and a
/// permission for each declared action carried by its relations and by the
/// same permission on each parent that declares that action.
pub(crate) fn app_definition(
    kind: &str,
    model: &KindModel,
    kinds: &BTreeMap<String, KindModel>,
) -> String {
    let mut lines: Vec<String> = model
        .relations
        .keys()
        .map(|relation| format!("  relation {relation}: grant#holder\n"))
        .collect();
    lines.extend(model.parents.iter().map(|parent| {
        format!(
            "  relation {}: {}\n",
            parent_relation(parent),
            definition(parent)
        )
    }));
    for action in &model.actions {
        let carried = model
            .relations
            .iter()
            .filter(|(_, actions)| actions.contains(action))
            .map(|(relation, _)| relation.to_string());
        let flowed = model
            .parents
            .iter()
            .filter(|parent| {
                kinds
                    .get(parent.as_str())
                    .is_some_and(|above| above.actions.contains(action))
            })
            .map(|parent| format!("{}->{action}", parent_relation(parent)));
        let terms: Vec<String> = carried.chain(flowed).collect();
        let terms = if terms.is_empty() {
            "nil".to_owned()
        } else {
            terms.join(" + ")
        };
        lines.push(format!("  permission {action} = {terms}\n"));
    }
    format!(
        "\ndefinition {} {{\n{}}}\n",
        definition(kind),
        lines.concat()
    )
}
