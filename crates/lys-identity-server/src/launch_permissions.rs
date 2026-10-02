//! The permissions a described launch carries (HOME-037
//! R5): the profile's allow, deny and ask rules, its permission mode and its
//! additional directories, with the profile's tools allowed and each hard
//! rule of the agent's Tool policy denied. A profile rule the settings file
//! cannot express is refused by name when the profile is recorded; a hard
//! policy rule it cannot express is refused by its id at the start. A policy rule a
//! grant may lift stays with the judge hook, which reads the live grant; a
//! settings deny could never be lifted.

use lys_home::harness::description::Permissions as PermissionContract;
use lys_runner::judge::{Authority, Policy};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::ServerError;

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
pub fn checked(
    given: Permissions,
    contract: &PermissionContract,
) -> Result<Permissions, ServerError> {
    if (!given.allow.is_empty() || !given.deny.is_empty() || !given.ask.is_empty())
        && !contract
            .rule_forms
            .iter()
            .any(|form| form == "tool_specifier")
    {
        return Err(unrepresentable(
            "description.permissions.rule_forms",
            "no supported rule form was declared",
        ));
    }
    for rule in given.allow.iter().chain(&given.deny).chain(&given.ask) {
        if !expressible(rule) {
            return Err(unrepresentable(
                rule,
                "a rule is a tool name, or a tool name with one parenthesised specifier",
            ));
        }
    }
    if let Some(mode) = &given.default_mode
        && !contract.modes.contains(mode)
    {
        return Err(unrepresentable(
            mode,
            "the permission mode is absent from description.permissions.modes",
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
    contract: &PermissionContract,
) -> Result<Value, ServerError> {
    if contract.rule_forms.is_empty()
        && let Some(rule) = policy
            .into_iter()
            .flat_map(|policy| &policy.rules)
            .find(|rule| rule.authority == Authority::Hard)
    {
        return Err(unrepresentable(
            &rule.id,
            "the program declares no supported hard policy rule form",
        ));
    }
    let given = permissions.cloned().unwrap_or_default();
    let mut allow = given.allow;
    for tool in tools {
        push(&mut allow, tool.clone());
    }
    let merged = checked(
        Permissions {
            allow,
            deny: given.deny,
            ask: given.ask,
            additional_directories: given.additional_directories,
            default_mode: given.default_mode,
        },
        contract,
    )?;
    let mut value = serde_json::to_value(merged)
        .map_err(|error| unrepresentable("permissions", &error.to_string()))?;
    value["hard_rules"] = serde_json::to_value(
        policy
            .map_or(&[][..], |policy| policy.rules.as_slice())
            .iter()
            .filter(|rule| rule.authority == Authority::Hard)
            .collect::<Vec<_>>(),
    )
    .map_err(|error| unrepresentable("policy", &error.to_string()))?;
    Ok(value)
}
