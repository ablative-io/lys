//! The permissions a Claude Code launch's settings file carries (HOME-037
//! R5): the profile's allow, deny and ask rules, its permission mode and its
//! additional directories, with the profile's tools allowed and each hard
//! rule of the agent's Tool policy denied. A profile rule the settings file
//! cannot express is refused by name when the profile is recorded; a hard
//! policy rule it cannot express is refused by its id at the start. A policy rule a
//! grant may lift stays with the judge hook, which reads the live grant; a
//! settings deny could never be lifted.

use lys_runner::judge::{Authority, Policy, Rule, RuleKind};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::error::ServerError;

/// The permission modes Claude Code takes, as `claude --help` lists the
/// choices of `--permission-mode` in Claude Code 2.1.284.
const MODES: [&str; 6] = [
    "acceptEdits",
    "auto",
    "bypassPermissions",
    "manual",
    "dontAsk",
    "plan",
];

/// A profile's permissions, as the operator recorded them.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct Permissions {
    /// Rules allowed without asking.
    #[serde(default)]
    pub allow: Vec<String>,
    /// Rules refused.
    #[serde(default)]
    pub deny: Vec<String>,
    /// Rules asked about.
    #[serde(default)]
    pub ask: Vec<String>,
    /// The permission mode, when the profile sets one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_mode: Option<String>,
    /// Directories beyond the working directory, absolute.
    #[serde(default)]
    pub additional_directories: Vec<String>,
}

fn unrepresentable(rule: &str, reason: &str) -> ServerError {
    ServerError::PolicyUnrepresentable {
        rule: rule.to_owned(),
        reason: reason.to_owned(),
    }
}

/// Whether `rule` is `Tool` or `Tool(specifier)` as the settings file reads
/// it: a specifier is closed, not empty, and holds no bracket of its own.
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

/// `given`, refused by the first rule, mode or directory the settings file
/// cannot express.
pub fn checked(given: Permissions) -> Result<Permissions, ServerError> {
    for rule in given.allow.iter().chain(&given.deny).chain(&given.ask) {
        if !expressible(rule) {
            return Err(unrepresentable(
                rule,
                "a rule is a tool name, or a tool name with one parenthesised specifier",
            ));
        }
    }
    if let Some(mode) = &given.default_mode
        && !MODES.contains(&mode.as_str())
    {
        return Err(unrepresentable(
            mode,
            "the permission mode is not one Claude Code takes",
        ));
    }
    for dir in &given.additional_directories {
        if !dir.starts_with('/') || dir.contains(|c: char| c.is_control()) {
            return Err(unrepresentable(
                dir,
                "an additional directory is one absolute path",
            ));
        }
    }
    Ok(given)
}

/// The deny rules the settings file writes for a hard policy `rule`: the
/// whole tool, a path prefix on `Read` or `Edit` as that path and everything
/// under it, or `WebFetch`'s host; any other rule is refused by its id.
fn denied(rule: &Rule) -> Result<Vec<String>, ServerError> {
    let refused = |reason: &str| unrepresentable(&rule.id, reason);
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

fn push(list: &mut Vec<String>, rule: String) {
    if !list.contains(&rule) {
        list.push(rule);
    }
}

/// The settings file's `permissions`: the profile's, with each of `tools`
/// allowed and each hard rule of `policy` denied, in that order, each rule
/// once. A hard rule the settings file cannot express is refused by its id.
pub fn settings(
    permissions: Option<&Permissions>,
    tools: &[String],
    policy: Option<&Policy>,
) -> Result<Value, ServerError> {
    let given = permissions.cloned().unwrap_or_default();
    let mut allow = given.allow;
    for tool in tools {
        push(&mut allow, tool.clone());
    }
    let mut deny = given.deny;
    for rule in policy.map_or(&[][..], |policy| policy.rules.as_slice()) {
        if rule.authority == Authority::Hard {
            for one in denied(rule)? {
                push(&mut deny, one);
            }
        }
    }
    let mut out = json!({
        "allow": allow,
        "deny": deny,
        "ask": given.ask,
        "additionalDirectories": given.additional_directories,
    });
    if let Some(mode) = given.default_mode {
        out["defaultMode"] = json!(mode);
    }
    Ok(out)
}
