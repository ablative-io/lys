//! Native permission syntax and hard-policy conversion, owned by the home.
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum RuleKind {
    Tool,
    PathPrefix,
    Host,
}
#[derive(Deserialize)]
struct Rule {
    id: String,
    tool: String,
    kind: RuleKind,
    target: Option<String>,
}
fn expressible(rule: &str) -> bool {
    let named = |tool: &str| {
        tool.starts_with(|c: char| c.is_ascii_alphabetic())
            && tool
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    };
    let shaped = match rule.split_once('(') {
        None => named(rule),
        Some((tool, rest)) => {
            named(tool)
                && rest.strip_suffix(')').is_some_and(|specifier| {
                    !specifier.is_empty() && !specifier.contains(['(', ')'])
                })
        }
    };
    shaped && !rule.contains(|c: char| c.is_control())
}

/// The deny rules the settings file writes for a hard policy `rule`: the
/// whole tool, a path prefix on `Read` or `Edit` as that path and everything
/// under it, or `WebFetch`'s host; any other rule is refused by its id.
fn denied(rule: &Rule) -> Result<Vec<String>, String> {
    let refused = |reason: &str| format!("policy rule {}: {reason}", rule.id);
    let written = match (rule.kind, rule.target.as_deref()) {
        (RuleKind::Tool, None) => vec![rule.tool.clone()],
        (RuleKind::PathPrefix, Some(path)) if matches!(rule.tool.as_str(), "Read" | "Edit") => {
            if path.contains(['*', '?', '[', ']', '(', ')', '\\', '!']) {
                return Err(refused(
                    "the settings file reads a path rule as a pattern, and this path holds a pattern character",
                ));
            }
            let tool = &rule.tool;
            vec![format!("{tool}(/{path})"), format!("{tool}(/{path}/**)")]
        }
        (RuleKind::PathPrefix, _) => {
            return Err(refused(
                "the settings file takes a path rule only on Read or Edit",
            ));
        }
        (RuleKind::Host, Some(host)) if rule.tool == "WebFetch" => {
            vec![format!("WebFetch(domain:{host})")]
        }
        (RuleKind::Host, _) => {
            return Err(refused(
                "the settings file takes a host rule only on WebFetch",
            ));
        }
        (RuleKind::Tool, Some(_)) => return Err(refused("a whole-tool rule carries no target")),
    };
    match written.iter().find(|one| !expressible(one)) {
        Some(_) => Err(refused("its tool is not a name the settings file reads")),
        None => Ok(written),
    }
}

pub(super) fn render(permissions: &Value) -> Result<Value, String> {
    let rules: Vec<Rule> = serde_json::from_value(permissions["hard_rules"].clone())
        .map_err(|error| error.to_string())?;
    let mut deny: Vec<String> =
        serde_json::from_value(permissions["deny"].clone()).map_err(|error| error.to_string())?;
    for rule in rules {
        for item in denied(&rule)? {
            if !deny.contains(&item) {
                deny.push(item);
            }
        }
    }
    let mut native = json!({"allow": permissions["allow"], "deny": deny,
        "ask": permissions["ask"], "additionalDirectories": permissions["additional_directories"]});
    if let Some(mode) = permissions.get("default_mode") {
        native["defaultMode"] = mode.clone();
    }
    Ok(native)
}
